use touchstone::{ABCDMatrix, Complex, Network, NetworkBuilder, SMatrix};

const Z0: f64 = 50.0;
const FREQUENCIES: [f64; 2] = [1.0e9, 2.0e9];

fn c(re: f64, im: f64) -> Complex {
    Complex { re, im }
}

fn polar(db: f64, degrees: f64) -> Complex {
    let magnitude = 10.0_f64.powf(db / 20.0);
    let angle = degrees.to_radians();
    c(magnitude * angle.cos(), magnitude * angle.sin())
}

fn two_port(s11: Complex, s21: Complex, s12: Complex, s22: Complex) -> SMatrix {
    SMatrix {
        rank: 2,
        data: vec![vec![s11, s12], vec![s21, s22]],
    }
}

fn generated(name: &str, values: SMatrix) -> Network {
    let mut builder = NetworkBuilder::new(name, 2);
    for frequency in FREQUENCIES {
        builder = builder.point(frequency, values.clone());
    }
    builder.build().unwrap()
}

fn matched(name: &str, forward: Complex, reverse: Complex) -> Network {
    generated(name, two_port(c(0.0, 0.0), forward, reverse, c(0.0, 0.0)))
}

fn magnitude(value: Complex) -> f64 {
    value.re.hypot(value.im)
}

fn assert_complex_close(label: &str, actual: Complex, expected: Complex) {
    assert!(
        actual.re.is_finite() && actual.im.is_finite(),
        "{label}: nonfinite result {actual:?}"
    );
    assert!(expected.re.is_finite() && expected.im.is_finite());
    let error = magnitude(actual - expected);
    let tolerance = 1.0e-12 + 1.0e-9 * magnitude(expected);
    assert!(
        error <= tolerance,
        "{label}: actual {actual:?}, expected {expected:?}, error {error:e} exceeds {tolerance:e}"
    );
}

fn assert_s_close(label: &str, actual: &SMatrix, expected: &SMatrix) {
    assert_eq!(actual.rank, 2);
    assert_eq!(expected.rank, 2);
    for to_port in 1..=2 {
        for from_port in 1..=2 {
            assert_complex_close(
                &format!("{label} S{to_port}{from_port}"),
                actual.get(to_port, from_port).unwrap(),
                expected.get(to_port, from_port).unwrap(),
            );
        }
    }
}

fn assert_network_s_close(label: &str, actual: &Network, expected: &Network) {
    assert_eq!(actual.f, expected.f, "{label}: frequency grid");
    for point_index in 0..actual.f.len() {
        assert_s_close(
            &format!("{label} at {} Hz", actual.f[point_index]),
            &actual.s_matrix_at(point_index).unwrap(),
            &expected.s_matrix_at(point_index).unwrap(),
        );
    }
}

fn assert_db_close(label: &str, actual: Complex, expected_db: f64) {
    let actual_db = 20.0 * magnitude(actual).log10();
    assert!(actual_db.is_finite(), "{label}: nonfinite dB value");
    assert!(
        (actual_db - expected_db).abs() <= 1.0e-8,
        "{label}: {actual_db} dB, expected {expected_db} dB"
    );
}

fn abcd_product(left: ABCDMatrix, right: ABCDMatrix) -> ABCDMatrix {
    ABCDMatrix {
        a: left.a * right.a + left.b * right.c,
        b: left.a * right.b + left.b * right.d,
        c: left.c * right.a + left.d * right.c,
        d: left.c * right.b + left.d * right.d,
    }
}

#[test]
fn thru_is_identity_on_either_side_of_a_complex_two_port() {
    let thru = matched("thru.s2p", c(1.0, 0.0), c(1.0, 0.0));
    let device = generated(
        "complex_device.s2p",
        two_port(c(0.08, 0.03), c(0.61, -0.17), c(0.34, 0.09), c(-0.04, 0.02)),
    );

    assert_network_s_close(
        "thru then device",
        &thru.try_cascade(&device).unwrap(),
        &device,
    );
    assert_network_s_close(
        "device then thru",
        &device.try_cascade(&thru).unwrap(),
        &device,
    );
}

#[test]
fn matched_three_and_six_db_losses_add_to_nine_db() {
    let three_db = matched("three_db.s2p", polar(-3.0, 15.0), polar(-3.0, 15.0));
    let six_db = matched("six_db.s2p", polar(-6.0, -5.0), polar(-6.0, -5.0));
    let cascade = three_db.try_cascade(&six_db).unwrap();
    let expected = matched("nine_db.s2p", polar(-9.0, 10.0), polar(-9.0, 10.0));

    assert_network_s_close("matched loss", &cascade, &expected);
    assert_db_close("forward loss", cascade.try_s_ri_at(0, 2, 1).unwrap(), -9.0);
    assert_db_close("reverse loss", cascade.try_s_ri_at(0, 1, 2).unwrap(), -9.0);
}

#[test]
fn twenty_db_amplifier_and_six_db_pad_make_fourteen_db_forward_gain() {
    let amplifier = matched("amplifier.s2p", polar(20.0, 30.0), polar(-35.0, 4.0));
    let pad = matched("six_db_pad.s2p", polar(-6.0, -10.0), polar(-6.0, -10.0));
    let cascade = amplifier.try_cascade(&pad).unwrap();
    let expected = matched(
        "amplifier_then_pad.s2p",
        polar(14.0, 20.0),
        polar(-41.0, -6.0),
    );

    assert_network_s_close("amplifier then pad", &cascade, &expected);
    assert_db_close("forward gain", cascade.try_s_ri_at(0, 2, 1).unwrap(), 14.0);
    assert_db_close(
        "reverse transmission",
        cascade.try_s_ri_at(0, 1, 2).unwrap(),
        -41.0,
    );
}

#[test]
fn complex_reciprocal_passive_cascade_remains_reciprocal() {
    let first = generated(
        "reciprocal_a.s2p",
        two_port(
            c(0.08, 0.03),
            c(0.60, -0.20),
            c(0.60, -0.20),
            c(-0.04, 0.02),
        ),
    );
    let second = generated(
        "reciprocal_b.s2p",
        two_port(c(-0.06, 0.01), c(0.45, 0.18), c(0.45, 0.18), c(0.03, -0.07)),
    );
    let cascade = first.try_cascade(&second).unwrap();

    for index in 0..FREQUENCIES.len() {
        let s = cascade.s_matrix_at(index).unwrap();
        assert_complex_close(
            "reciprocal S12 = S21",
            s.get(1, 2).unwrap(),
            s.get(2, 1).unwrap(),
        );
    }
}

#[test]
fn complex_nonreciprocal_cascade_preserves_distinct_forward_and_reverse_paths() {
    let first = generated(
        "nonreciprocal_a.s2p",
        two_port(c(0.08, 0.03), c(1.4, -0.2), c(0.20, 0.05), c(-0.04, 0.02)),
    );
    let second = generated(
        "nonreciprocal_b.s2p",
        two_port(
            c(-0.06, 0.01),
            c(0.70, 0.18),
            c(0.11, -0.03),
            c(0.03, -0.07),
        ),
    );
    let cascade = first.try_cascade(&second).unwrap();

    for index in 0..FREQUENCIES.len() {
        let expected = SMatrix::try_from_abcd(
            &abcd_product(
                first.abcd_at(index).unwrap(),
                second.abcd_at(index).unwrap(),
            ),
            Z0,
        )
        .unwrap();
        let actual = cascade.s_matrix_at(index).unwrap();
        assert_s_close("public ABCD pair product", &actual, &expected);
        assert!(
            magnitude(actual.get(2, 1).unwrap() - actual.get(1, 2).unwrap()) > 0.1,
            "distinct forward and reverse paths were lost"
        );
    }
}

#[test]
fn cascade_parenthesization_agrees_without_reordering_stages() {
    let a = generated(
        "a.s2p",
        two_port(c(0.08, 0.03), c(0.61, -0.17), c(0.34, 0.09), c(-0.04, 0.02)),
    );
    let b = generated(
        "b.s2p",
        two_port(
            c(-0.06, 0.01),
            c(0.70, 0.18),
            c(0.11, -0.03),
            c(0.03, -0.07),
        ),
    );
    let c = matched("c.s2p", polar(-3.0, 12.0), polar(-3.0, 12.0));

    let left = a.try_cascade(&b).unwrap().try_cascade(&c).unwrap();
    let right = a.try_cascade(&b.try_cascade(&c).unwrap()).unwrap();
    assert_network_s_close("(A B) C = A (B C)", &left, &right);
}

#[test]
fn ordinary_complex_s_to_abcd_to_s_round_trip_preserves_all_four_values() {
    let fixtures = [
        two_port(
            c(0.08, 0.03),
            c(0.60, -0.20),
            c(0.60, -0.20),
            c(-0.04, 0.02),
        ),
        two_port(c(0.08, 0.03), c(1.4, -0.2), c(0.20, 0.05), c(-0.04, 0.02)),
    ];
    for (index, original) in fixtures.iter().enumerate() {
        let abcd = original.to_abcd(Z0).unwrap();
        let recovered = SMatrix::try_from_abcd(&abcd, Z0).unwrap();
        assert_s_close(&format!("round trip fixture {index}"), &recovered, original);
    }
}

#[test]
fn zero_forward_transmission_reports_conversion_and_cascade_errors() {
    let blocked = matched("blocked.s2p", c(0.0, 0.0), c(0.2, 0.0));
    let thru = matched("thru.s2p", c(1.0, 0.0), c(1.0, 0.0));

    assert!(blocked.s_matrix_at(0).unwrap().to_abcd(Z0).is_err());
    assert!(blocked.try_cascade(&thru).is_err());
    assert!(thru.try_cascade(&blocked).is_err());
}
