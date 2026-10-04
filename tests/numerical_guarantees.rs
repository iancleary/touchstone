use touchstone::{
    ABCDMatrix, Complex, Extrapolation, Interpolation, Network, NetworkBuilder, ParameterMatrix,
    SMatrix, TouchstoneError,
};

fn c(re: f64, im: f64) -> Complex {
    Complex { re, im }
}

fn matrix(entries: [[Complex; 2]; 2]) -> SMatrix {
    SMatrix {
        rank: 2,
        data: entries.into_iter().map(Vec::from).collect(),
    }
}

fn network(s: SMatrix) -> Network {
    NetworkBuilder::new("numerics.s2p", 2)
        .point(1.0e9, s)
        .build()
        .unwrap()
}

fn matched(forward: f64, reverse: f64) -> Network {
    network(matrix([
        [c(0.0, 0.0), c(reverse, 0.0)],
        [c(forward, 0.0), c(0.0, 0.0)],
    ]))
}

fn assert_finite(data: &[Vec<Complex>]) {
    for value in data.iter().flatten() {
        assert!(value.re.is_finite() && value.im.is_finite(), "{value:?}");
    }
}

fn assert_close(actual: Complex, expected: Complex, absolute: f64, relative: f64) {
    assert!(actual.re.is_finite() && actual.im.is_finite(), "{actual:?}");
    let error = (actual.re - expected.re).hypot(actual.im - expected.im);
    assert!(
        error <= absolute + relative * expected.re.hypot(expected.im),
        "actual {actual:?}, expected {expected:?}, error {error:e}"
    );
}

fn assert_matched_loss(network: &Network, attenuation_db: f64) {
    let transmission = 10.0_f64.powf(-attenuation_db / 20.0);
    for (to, from) in [(2, 1), (1, 2)] {
        let value = network.try_s_ri_at(0, to, from).unwrap();
        // No absolute floor: a zero transmission must fail even for deep loss.
        assert_close(value, c(transmission, 0.0), 0.0, 5.0e-13);
        assert!((20.0 * value.re.hypot(value.im).log10() + attenuation_db).abs() < 1.0e-9);
    }
    assert_close(
        network.try_s_ri_at(0, 1, 1).unwrap(),
        c(0.0, 0.0),
        1.0e-15,
        0.0,
    );
    assert_close(
        network.try_s_ri_at(0, 2, 2).unwrap(),
        c(0.0, 0.0),
        1.0e-15,
        0.0,
    );
}

#[test]
fn deep_matched_loss_preserves_both_transmission_paths() {
    for loss_db in [90.0, 120.0, 1500.0] {
        let transmission = 10.0_f64.powf(-loss_db / 20.0);
        let stage = matched(transmission, transmission);
        assert_matched_loss(&stage.try_cascade(&stage).unwrap(), 2.0 * loss_db);
    }

    let stage = matched(0.1, 0.1);
    let mut cascade = matched(1.0, 1.0);
    for count in 1..=12 {
        cascade = cascade.try_cascade(&stage).unwrap();
        assert_matched_loss(&cascade, count as f64 * 20.0);
    }
}

#[test]
fn complex_mismatched_nonreciprocal_pair_matches_exact_rational_reference() {
    let left = network(matrix([
        [c(0.1, 0.2), c(0.3, -0.1)],
        [c(0.8, 0.4), c(0.25, 0.25)],
    ]));
    let right = network(matrix([
        [c(0.2, -0.4), c(0.1, 0.3)],
        [c(0.7, -0.2), c(-0.15, 0.05)],
    ]));

    // Exact rational solution of the two internal-wave equations:
    // x - R11*y = R12*a_right; y - L22*x = L21*a_left.
    // Excite each external port separately, then b_left=L11*a_left+L12*x
    // and b_right=R21*y+R22*a_right. These fractions were evaluated without
    // floating point or the crate's ABCD/conversion/cascade implementations.
    let expected = [
        [
            c(257.0 / 1450.0, 53.0 / 725.0),
            c(11.0 / 145.0, 13.0 / 145.0),
        ],
        [c(22.0 / 29.0, 14.0 / 145.0), c(-47.0 / 290.0, 21.0 / 145.0)],
    ];
    let result = left.try_cascade(&right).unwrap().s_matrix_at(0).unwrap();
    for (row, values) in expected.iter().enumerate() {
        for (column, value) in values.iter().enumerate() {
            assert_close(result.data[row][column], *value, 1.0e-15, 1.0e-13);
        }
    }
}

#[test]
fn singular_internal_connection_has_a_structured_error() {
    for loop_reflection in [1.0, 1.0 - 5.0e-13] {
        let left = network(matrix([
            [c(0.0, 0.0), c(0.5, 0.0)],
            [c(0.5, 0.0), c(1.0, 0.0)],
        ]));
        let right = network(matrix([
            [c(loop_reflection, 0.0), c(0.5, 0.0)],
            [c(0.5, 0.0), c(0.0, 0.0)],
        ]));
        assert!(matches!(
            left.try_cascade(&right),
            Err(TouchstoneError::SingularMatrix { operation, pivot_index: 0, .. })
                if operation == "cascade internal connection"
        ));
    }
}

#[test]
fn resistor_circuits_provide_independent_parameter_conversion_references() {
    // A 100-ohm series resistor between 50-ohm ports has Sij=1/2.
    let series = matrix([[c(0.5, 0.0); 2]; 2]);
    let y = series.to_y_matrix(50.0).unwrap();
    for row in 0..2 {
        for column in 0..2 {
            let expected = if row == column { 0.01 } else { -0.01 };
            assert_close(y.data[row][column], c(expected, 0.0), 1.0e-16, 1.0e-13);
        }
    }
    let abcd = ABCDMatrix {
        a: c(1.0, 0.0),
        b: c(100.0, 0.0),
        c: c(0.0, 0.0),
        d: c(1.0, 0.0),
    };
    for result in [
        y.to_s_matrix_from_y(50.0).unwrap(),
        abcd.to_s_matrix(50.0).unwrap(),
    ] {
        for value in result.data.iter().flatten() {
            assert_close(*value, c(0.5, 0.0), 1.0e-15, 1.0e-13);
        }
    }

    // A 50-ohm shunt resistor at the junction has Zij=50 ohms,
    // reflection -1/3 and transmission 2/3 at 50-ohm ports.
    let shunt = matrix([
        [c(-1.0 / 3.0, 0.0), c(2.0 / 3.0, 0.0)],
        [c(2.0 / 3.0, 0.0), c(-1.0 / 3.0, 0.0)],
    ]);
    let z = shunt.to_z_matrix(50.0).unwrap();
    for value in z.data.iter().flatten() {
        assert_close(*value, c(50.0, 0.0), 1.0e-13, 1.0e-13);
    }
    let recovered = z.to_s_matrix_from_z(50.0).unwrap();
    for row in 0..2 {
        for column in 0..2 {
            assert_close(
                recovered.data[row][column],
                shunt.data[row][column],
                1.0e-15,
                1.0e-13,
            );
        }
    }
}

#[test]
fn singular_parameter_conversions_return_errors() {
    let scalar = |re| SMatrix {
        rank: 1,
        data: vec![vec![c(re, 0.0)]],
    };
    let parameter = |re| ParameterMatrix {
        rank: 1,
        data: vec![vec![c(re, 0.0)]],
    };
    for result in [
        scalar(-1.0).to_y_matrix(50.0),
        scalar(1.0).to_z_matrix(50.0),
    ] {
        assert!(matches!(
            result,
            Err(TouchstoneError::SingularMatrix { .. })
        ));
    }
    for result in [
        parameter(-0.02).to_s_matrix_from_y(50.0),
        parameter(-50.0).to_s_matrix_from_z(50.0),
        ABCDMatrix {
            a: c(1.0, 0.0),
            b: c(0.0, 0.0),
            c: c(0.0, 0.0),
            d: c(-1.0, 0.0),
        }
        .to_s_matrix(50.0),
    ] {
        assert!(matches!(
            result,
            Err(TouchstoneError::SingularMatrix { .. })
        ));
    }
}

#[test]
fn successful_conversions_have_finite_components_across_extreme_scales() {
    for value in [
        1.0e-320, 1.0e-300, 1.0e-200, 1.0e-50, 1.0, 1.0e50, 1.0e200, 1.0e300, 1.0e308,
    ] {
        for z0 in [1.0e-320, 1.0e-200, 50.0, 1.0e200, 1.0e308] {
            let s = matrix([
                [c(value, -value), c(0.25, 0.0)],
                [c(0.5, 0.0), c(-value, value)],
            ]);
            for converted in [s.to_y_matrix(z0), s.to_z_matrix(z0)].into_iter().flatten() {
                assert_finite(&converted.data);
            }
            let p = ParameterMatrix {
                rank: 2,
                data: s.data.clone(),
            };
            for converted in [p.to_s_matrix_from_y(z0), p.to_s_matrix_from_z(z0)]
                .into_iter()
                .flatten()
            {
                assert_finite(&converted.data);
            }
            if let Ok(abcd) = s.to_abcd(z0) {
                for entry in [abcd.a, abcd.b, abcd.c, abcd.d] {
                    assert!(entry.re.is_finite() && entry.im.is_finite());
                }
                if let Ok(converted) = abcd.to_s_matrix(z0) {
                    assert_finite(&converted.data);
                }
            }
        }
    }
    // An overflowing finite-input cascade must fail, not return infinite gain.
    let huge = matched(1.0e200, 1.0e200);
    assert!(matches!(
        huge.try_cascade(&huge),
        Err(TouchstoneError::InvalidParameterMatrixValue { .. })
    ));
    // S=0 at this impedance requires an unrepresentable admittance.
    let zero = SMatrix {
        rank: 1,
        data: vec![vec![c(0.0, 0.0)]],
    };
    assert!(zero.to_y_matrix(1.0e-320).is_err());
}

#[test]
fn complex_division_and_derived_magnitudes_handle_extreme_finite_values() {
    for scale in [f64::from_bits(1), 1.0e-300, 1.0, 1.0e300, f64::MAX] {
        assert_close(
            c(scale, scale) / c(scale, -scale),
            c(0.0, 1.0),
            1.0e-15,
            0.0,
        );
    }
    assert_close(
        c(f64::MAX, f64::MAX) / c(1.0, 1.0),
        c(f64::MAX, 0.0),
        0.0,
        1.0e-15,
    );
    assert_close(
        c(1.0, 1.0) / c(1.0e-308, 1.0e-308),
        c(1.0e308, 0.0),
        0.0,
        1.0e-15,
    );

    for value in [1.0e-200, 1.0e200] {
        let n = matched(value, value);
        assert!((n.s_ma(2, 1)[0].s_ma.0 / value - 1.0).abs() < 1.0e-14);
        assert!((n.s_db(2, 1)[0].s_db.0 - 20.0 * value.log10()).abs() < 1.0e-10);
    }
}

#[test]
fn interpolation_handles_opposite_extreme_values_without_overflow() {
    let scalar = |value| SMatrix {
        rank: 1,
        data: vec![vec![c(value, -value)]],
    };
    let n = NetworkBuilder::new("interpolation.s1p", 1)
        .point(0.0, scalar(-f64::MAX))
        .point(f64::MAX, scalar(f64::MAX))
        .build()
        .unwrap();
    let sampled = n
        .sample_at(f64::MAX / 2.0, Interpolation::Linear, Extrapolation::Error)
        .unwrap();
    assert_eq!(sampled.s.get(1, 1).unwrap(), c(0.0, 0.0));
    let resampled = n
        .resample(
            [f64::MAX / 2.0],
            Interpolation::Linear,
            Extrapolation::Error,
        )
        .unwrap();
    assert_eq!(resampled.try_s_ri_at(0, 1, 1).unwrap(), c(0.0, 0.0));
    assert_eq!(resampled.s_db(1, 1)[0].s_db.0, f64::NEG_INFINITY);
}

#[test]
fn sampling_never_succeeds_with_selected_nonfinite_source_values() {
    for value in ["NaN", "inf", "-inf"] {
        let n = Network::from_str(
            "invalid.s1p",
            &format!("# Hz S RI R 50\n1 {value} 0\n2 0 0\n"),
        )
        .unwrap();
        for (frequency, interpolation, extrapolation) in [
            (1.0, Interpolation::Linear, Extrapolation::Error),
            (1.25, Interpolation::Nearest, Extrapolation::Error),
            (1.5, Interpolation::Linear, Extrapolation::Error),
            (0.0, Interpolation::Linear, Extrapolation::Clamp),
        ] {
            assert!(n
                .sample_at(frequency, interpolation, extrapolation)
                .is_err());
            assert!(n
                .resample([frequency], interpolation, extrapolation)
                .is_err());
        }
    }
}
