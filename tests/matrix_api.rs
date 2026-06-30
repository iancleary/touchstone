use touchstone::{Complex, Network, SMatrix, TouchstoneError};

const ONE_PORT_RI: &str = "# Hz S RI R 50\n1000000000 0.5 -0.25\n2000000000 0.6 -0.35\n";

const TWO_PORT_ASYMMETRIC_RI: &str =
    "# Hz S RI R 50\n1000000000 0.11 0.01 0.21 0.02 0.12 0.03 0.22 0.04\n";

const THREE_PORT_RI: &str = "\
# Hz S RI R 50
1000000000 11 0.1 12 0.2 13 0.3 21 0.4 22 0.5 23 0.6 31 0.7 32 0.8 33 0.9
";

const THREE_PORT_VALUES: &str =
    "1000000000 0.11 0.0 0.12 0.0 0.13 0.0 0.21 0.0 0.22 0.0 0.23 0.0 0.31 0.0 0.32 0.0 0.33 0.0\n";

fn c(re: f64, im: f64) -> Complex {
    Complex { re, im }
}

fn s_matrix(data: Vec<Vec<Complex>>) -> SMatrix {
    SMatrix {
        rank: data.len(),
        data,
    }
}

fn assert_complex_close(actual: Complex, expected: Complex) {
    let tolerance = 1.0e-9;
    assert!(
        (actual.re - expected.re).abs() <= tolerance,
        "expected re {} to be within {tolerance} of {}",
        actual.re,
        expected.re
    );
    assert!(
        (actual.im - expected.im).abs() <= tolerance,
        "expected im {} to be within {tolerance} of {}",
        actual.im,
        expected.im
    );
}

fn assert_s_matrix_close(actual: &SMatrix, expected: &SMatrix) {
    assert_eq!(actual.rank, expected.rank);
    for row in 1..=actual.rank {
        for column in 1..=actual.rank {
            assert_complex_close(
                actual.get(row, column).unwrap(),
                expected.get(row, column).unwrap(),
            );
        }
    }
}

#[test]
fn one_port_accessors_use_zero_based_frequency_indexes() {
    let network = Network::from_str("uploaded.s1p", ONE_PORT_RI).unwrap();

    assert_eq!(
        network.try_s_ri_at(0, 1, 1).unwrap(),
        Complex { re: 0.5, im: -0.25 }
    );

    let point = network.point_at(1).unwrap();
    assert_eq!(point.frequency, 2.0e9);
    assert_eq!(point.s.get(1, 1).unwrap(), Complex { re: 0.6, im: -0.35 });

    let points = network.points().unwrap();
    assert_eq!(points.len(), 2);
    assert_eq!(points[0].s.rank, 1);
    assert_eq!(points[0].s.data, vec![vec![Complex { re: 0.5, im: -0.25 }]]);
}

#[test]
fn two_port_matrix_preserves_asymmetric_s21_and_s12() {
    let network = Network::from_str("uploaded.s2p", TWO_PORT_ASYMMETRIC_RI).unwrap();
    let matrix = network.s_matrix_at(0).unwrap();

    assert_eq!(matrix.rank, 2);
    assert_eq!(matrix.get(1, 1).unwrap(), Complex { re: 0.11, im: 0.01 });
    assert_eq!(matrix.get(2, 1).unwrap(), Complex { re: 0.21, im: 0.02 });
    assert_eq!(matrix.get(1, 2).unwrap(), Complex { re: 0.12, im: 0.03 });
    assert_eq!(matrix.get(2, 2).unwrap(), Complex { re: 0.22, im: 0.04 });

    assert_ne!(matrix.get(2, 1).unwrap(), matrix.get(1, 2).unwrap());
}

#[test]
fn three_port_matrix_uses_one_based_rf_port_indexes() {
    let network = Network::from_str("uploaded.s3p", THREE_PORT_RI).unwrap();
    let matrix = network.s_matrix_at(0).unwrap();

    assert_eq!(matrix.rank, 3);
    assert_eq!(matrix.get(3, 2).unwrap(), Complex { re: 32.0, im: 0.8 });
    assert_eq!(
        network.try_s_ri_at(0, 2, 3).unwrap(),
        Complex { re: 23.0, im: 0.6 }
    );
}

#[test]
fn invalid_point_index_returns_structured_error() {
    let network = Network::from_str("uploaded.s1p", ONE_PORT_RI).unwrap();

    let error = network.point_at(2).unwrap_err();

    assert!(matches!(
        error,
        TouchstoneError::InvalidPointIndex {
            point_index: 2,
            point_count: 2
        }
    ));
}

#[test]
fn invalid_port_index_returns_structured_error() {
    let network = Network::from_str("uploaded.s2p", TWO_PORT_ASYMMETRIC_RI).unwrap();

    let error = network.try_s_ri_at(0, 3, 1).unwrap_err();

    assert!(matches!(
        error,
        TouchstoneError::InvalidPortIndex {
            to_port: 3,
            from_port: 1,
            rank: 2
        }
    ));
}

#[test]
fn matched_load_converts_between_s_y_and_z() {
    let s = s_matrix(vec![vec![c(0.0, 0.0)]]);

    let y = s.to_y_matrix(50.0).unwrap();
    assert_complex_close(y.get(1, 1).unwrap(), c(0.02, 0.0));
    assert_s_matrix_close(&SMatrix::try_from_y_matrix(&y, 50.0).unwrap(), &s);

    let z = s.to_z_matrix(50.0).unwrap();
    assert_complex_close(z.get(1, 1).unwrap(), c(50.0, 0.0));
    assert_s_matrix_close(&SMatrix::try_from_z_matrix(&z, 50.0).unwrap(), &s);
}

#[test]
fn through_two_port_converts_to_identity_abcd() {
    let network = Network::from_str(
        "through.s2p",
        "# Hz S RI R 50\n1000000000 0.0 0.0 1.0 0.0 1.0 0.0 0.0 0.0\n",
    )
    .unwrap();

    let abcd = network.abcd_at(0).unwrap();
    assert_complex_close(abcd.a, c(1.0, 0.0));
    assert_complex_close(abcd.b, c(0.0, 0.0));
    assert_complex_close(abcd.c, c(0.0, 0.0));
    assert_complex_close(abcd.d, c(1.0, 0.0));

    assert_s_matrix_close(
        &abcd.to_s_matrix(50.0).unwrap(),
        &network.s_matrix_at(0).unwrap(),
    );
}

#[test]
fn three_port_s_y_round_trip_preserves_matrix() {
    let s = s_matrix(vec![
        vec![c(0.10, 0.01), c(0.02, -0.01), c(0.01, 0.00)],
        vec![c(0.03, 0.02), c(-0.05, 0.01), c(0.04, -0.02)],
        vec![c(0.01, -0.01), c(-0.02, 0.00), c(0.08, 0.03)],
    ]);

    let y = s.to_y_matrix(50.0).unwrap();
    let s_back = SMatrix::try_from_y_matrix(&y, 50.0).unwrap();

    assert_s_matrix_close(&s_back, &s);
}

#[test]
fn abcd_rejects_non_two_port_rank_with_structured_error() {
    let s = s_matrix(vec![
        vec![c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.0)],
        vec![c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.0)],
        vec![c(0.0, 0.0), c(0.0, 0.0), c(0.0, 0.0)],
    ]);

    let error = s.to_abcd(50.0).unwrap_err();

    assert!(matches!(
        error,
        TouchstoneError::UnsupportedConversionRank {
            conversion,
            rank: 3,
            expected_rank: 2
        } if conversion == "S to ABCD"
    ));
}

#[test]
fn network_conversions_reject_per_port_reference_impedance() {
    let contents = format!(
        "[Version] 2.1\n# Hz S RI R 50\n[Number of Ports] 3\n[Reference] 45 50 55\n[Network Data]\n{THREE_PORT_VALUES}[End]\n"
    );
    let network = Network::from_str("uploaded.s3p", &contents).unwrap();

    let error = network.y_matrix_at(0).unwrap_err();

    assert!(matches!(
        error,
        TouchstoneError::UnsupportedReferenceImpedance { values }
            if values == vec![45.0, 50.0, 55.0]
    ));
}

#[test]
fn network_conversions_reject_non_s_source_parameter() {
    let network =
        Network::from_str("uploaded.s1p", "# Hz Y RI R 50\n1000000000 0.02 0.0\n").unwrap();

    let error = network.z_matrix_at(0).unwrap_err();

    assert!(matches!(
        error,
        TouchstoneError::UnsupportedNetworkParameter { parameter } if parameter == "Y"
    ));
}

#[test]
fn z_conversion_rejects_singular_matrix() {
    let s = s_matrix(vec![vec![c(1.0, 0.0)]]);

    let error = s.to_z_matrix(50.0).unwrap_err();

    assert!(matches!(
        error,
        TouchstoneError::SingularMatrix {
            operation,
            pivot_index: 0,
            ..
        } if operation == "S to Z matrix inversion"
    ));
}

#[test]
fn y_conversion_rejects_near_singular_matrix() {
    let s = s_matrix(vec![vec![c(-1.0 + 5.0e-13, 0.0)]]);

    let error = s.to_y_matrix(50.0).unwrap_err();

    assert!(matches!(
        error,
        TouchstoneError::SingularMatrix {
            operation,
            pivot_index: 0,
            ..
        } if operation == "S to Y matrix inversion"
    ));
}

#[test]
fn abcd_conversion_rejects_zero_forward_transmission() {
    let s = s_matrix(vec![
        vec![c(0.1, 0.0), c(0.2, 0.0)],
        vec![c(0.0, 0.0), c(0.3, 0.0)],
    ]);

    let error = s.to_abcd(50.0).unwrap_err();

    assert!(matches!(
        error,
        TouchstoneError::SingularMatrix {
            operation,
            pivot_index: 0,
            ..
        } if operation == "S to ABCD denominator"
    ));
}
