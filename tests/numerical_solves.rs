use touchstone::{Complex, ParameterMatrix, SMatrix, SolveDiagnostics, TouchstoneError};

fn c(re: f64, im: f64) -> Complex {
    Complex { re, im }
}
fn close(actual: &[Vec<Complex>], expected: &[Vec<Complex>], tolerance: f64) {
    for (a, b) in actual.iter().flatten().zip(expected.iter().flatten()) {
        assert!(
            (a.re - b.re).hypot(a.im - b.im) < tolerance,
            "{a:?} != {b:?}"
        );
    }
}
fn report_ok(report: SolveDiagnostics) {
    assert!(report.reciprocal_condition_inf.is_finite());
    assert!(report.reciprocal_condition_inf > 0.0 && report.reciprocal_condition_inf <= 1.0);
    assert!(
        report.backward_error_inf.is_finite() && report.backward_error_inf < 2e-15,
        "{report:?}"
    );
}

#[test]
fn nonreciprocal_complex_rational_reference_all_four_conversions() {
    // Exact rational oracle: S^2 = (i/8) I, hence (I +/- S)^-1 =
    // (I -/+ S)/(1-i/8). No numerical inverse or round trip supplies the oracle.
    let s = SMatrix::try_new(vec![
        vec![c(0.0, 0.0), c(0.0, 0.5)],
        vec![c(0.25, 0.0), c(0.0, 0.0)],
    ])
    .unwrap();
    let z = ParameterMatrix::try_new(vec![
        vec![
            c(50.0 * 63.0 / 65.0, 50.0 * 16.0 / 65.0),
            c(-50.0 * 8.0 / 65.0, 50.0 * 64.0 / 65.0),
        ],
        vec![
            c(50.0 * 32.0 / 65.0, 50.0 * 4.0 / 65.0),
            c(50.0 * 63.0 / 65.0, 50.0 * 16.0 / 65.0),
        ],
    ])
    .unwrap();
    let y = ParameterMatrix::try_new(vec![
        vec![
            c(0.02 * 63.0 / 65.0, 0.02 * 16.0 / 65.0),
            c(0.02 * 8.0 / 65.0, -0.02 * 64.0 / 65.0),
        ],
        vec![
            c(-0.02 * 32.0 / 65.0, -0.02 * 4.0 / 65.0),
            c(0.02 * 63.0 / 65.0, 0.02 * 16.0 / 65.0),
        ],
    ])
    .unwrap();
    let (actual_z, rz) = s.to_z_matrix_with_diagnostics(50.0).unwrap();
    let (actual_y, ry) = s.to_y_matrix_with_diagnostics(50.0).unwrap();
    let (from_z, rzs) = z.to_s_matrix_from_z_with_diagnostics(50.0).unwrap();
    let (from_y, rys) = y.to_s_matrix_from_y_with_diagnostics(50.0).unwrap();
    close(&actual_z.data, &z.data, 2e-14);
    close(&actual_y.data, &y.data, 1e-17);
    close(&from_z.data, &s.data, 3e-16);
    close(&from_y.data, &s.data, 3e-16);
    assert_eq!(s.to_z_matrix(50.0).unwrap(), actual_z);
    assert_eq!(s.to_y_matrix(50.0).unwrap(), actual_y);
    assert_eq!(SMatrix::try_from_z_matrix(&z, 50.0).unwrap(), from_z);
    assert_eq!(SMatrix::try_from_y_matrix(&y, 50.0).unwrap(), from_y);
    for report in [rz, ry, rzs, rys] {
        report_ok(report);
    }
    // Exact infinity-norm condition number of I +/- S, using complex moduli.
    assert!((rz.reciprocal_condition_inf - 65.0_f64.sqrt() / 18.0).abs() < 2e-16);
    assert!((ry.reciprocal_condition_inf - 65.0_f64.sqrt() / 18.0).abs() < 2e-16);
}

#[test]
fn analytic_resistive_and_reactive_loads() {
    // 75+j25 ohms at 50 ohms: Gamma=(3+2j)/13; Y=(3-j)/250.
    let z = ParameterMatrix::try_new(vec![vec![c(75.0, 25.0)]]).unwrap();
    let y = ParameterMatrix::try_new(vec![vec![c(3.0 / 250.0, -1.0 / 250.0)]]).unwrap();
    let expected = vec![vec![c(3.0 / 13.0, 2.0 / 13.0)]];
    close(&z.to_s_matrix_from_z(50.0).unwrap().data, &expected, 1e-16);
    close(&y.to_s_matrix_from_y(50.0).unwrap().data, &expected, 1e-16);
    let s = SMatrix::try_new(expected).unwrap();
    close(&s.to_z_matrix(50.0).unwrap().data, &z.data, 2e-14);
    close(&s.to_y_matrix(50.0).unwrap().data, &y.data, 1e-17);
    // A zero RHS has an explicitly defined zero backward error.
    let matched = SMatrix::try_new(vec![vec![c(0.0, 0.0)]]).unwrap();
    let (z, report) = matched.to_z_matrix_with_diagnostics(50.0).unwrap();
    assert_eq!(report.reciprocal_condition_inf, 1.0);
    let (s, report) = z.to_s_matrix_from_z_with_diagnostics(50.0).unwrap();
    assert_eq!(s.data[0][0], c(0.0, 0.0));
    assert_eq!(report.backward_error_inf, 0.0);
}

#[test]
fn ill_conditioned_finite_solve_is_not_an_accuracy_certificate() {
    let s = SMatrix::try_new(vec![
        vec![c(1.0 - 1e-10, 0.0), c(0.0, 0.0)],
        vec![c(0.0, 0.0), c(0.0, 0.0)],
    ])
    .unwrap();
    let (z, report) = s.to_z_matrix_with_diagnostics(50.0).unwrap();
    let delta = 1.0 - s.data[0][0].re;
    let expected = 50.0 * (2.0 - delta) / delta;
    assert!((z.data[0][0].re / expected - 1.0).abs() < 3e-16);
    assert!((report.reciprocal_condition_inf / delta - 1.0).abs() < 3e-16);
    assert!(report.backward_error_inf < 1e-15);
    assert!(report.reciprocal_condition_inf < 2e-10);
}

#[test]
fn scaled_rational_circuit_and_cutoff_are_explicit() {
    // D=scale*[[1,1],[2,1]], z0=scale/8: S=I-(scale/4)D^-1.
    let expected = vec![
        vec![c(1.25, 0.0), c(-0.25, 0.0)],
        vec![c(-0.5, 0.0), c(1.25, 0.0)],
    ];
    for scale in [2e-12, 1.0, 1e150, 1e300] {
        let z0 = scale / 8.0;
        let z = ParameterMatrix::try_new(vec![
            vec![c(scale - z0, 0.0), c(scale, 0.0)],
            vec![c(2.0 * scale, 0.0), c(scale - z0, 0.0)],
        ])
        .unwrap();
        let (s, report) = z.to_s_matrix_from_z_with_diagnostics(z0).unwrap();
        close(&s.data, &expected, 5e-16);
        assert!((report.reciprocal_condition_inf - 1.0 / 9.0).abs() < 1e-16);
        report_ok(report);
    }
    for scale in [1e-13, 1e-12] {
        let z0 = scale / 8.0;
        let z = ParameterMatrix::try_new(vec![
            vec![c(scale - z0, 0.0), c(scale, 0.0)],
            vec![c(2.0 * scale, 0.0), c(scale - z0, 0.0)],
        ])
        .unwrap();
        assert!(matches!(
            z.to_s_matrix_from_z(z0),
            Err(TouchstoneError::SingularMatrix { .. })
        ));
    }
    // The old D-based Gauss-Jordan accepted this denominator, but the
    // transposed solve reaches an exactly-at-cutoff second pivot. See the
    // runnable experiment for both directions of compatibility differences.
    let z0 = 1e-15;
    let z = ParameterMatrix::try_new(vec![
        vec![c(1e-12 - z0, 0.0), c(1.0, 0.0)],
        vec![c(2e-12, 0.0), c(1.0 - z0, 0.0)],
    ])
    .unwrap();
    assert!(matches!(
        z.to_s_matrix_from_z(z0),
        Err(TouchstoneError::SingularMatrix { pivot_index: 1, .. })
    ));
}

#[test]
fn singular_and_extreme_finite_inputs_never_silently_return_nan() {
    for s_value in [-1.0, 1.0] {
        let s = SMatrix::try_new(vec![vec![c(s_value, 0.0)]]).unwrap();
        let result = if s_value == -1.0 {
            s.to_y_matrix_with_diagnostics(50.0)
        } else {
            s.to_z_matrix_with_diagnostics(50.0)
        };
        assert!(matches!(
            result,
            Err(TouchstoneError::SingularMatrix { .. })
        ));
    }
    let p = ParameterMatrix::try_new(vec![vec![c(-1.0, 0.0)]]).unwrap();
    assert!(matches!(
        p.to_s_matrix_from_y_with_diagnostics(1.0),
        Err(TouchstoneError::SingularMatrix { .. })
    ));
    assert!(matches!(
        p.to_s_matrix_from_z_with_diagnostics(1.0),
        Err(TouchstoneError::SingularMatrix { .. })
    ));
    let p = ParameterMatrix::try_new(vec![vec![c(f64::MAX, 0.0)]]).unwrap();
    assert!(p.to_s_matrix_from_z_with_diagnostics(f64::MAX).is_err());
    assert!(p.to_s_matrix_from_y_with_diagnostics(2.0).is_err());
    // The ordinary path can succeed while diagnostic norm accumulation exceeds
    // f64 range. Reporting an error is preferable to NaN or a false certificate.
    let p = ParameterMatrix::try_new(vec![
        vec![c(1e308, 0.0), c(1e308, 0.0)],
        vec![c(0.0, 0.0), c(1e308, 0.0)],
    ])
    .unwrap();
    assert!(p.to_s_matrix_from_z(1.0).is_ok());
    assert!(p.to_s_matrix_from_z_with_diagnostics(1.0).is_err());
    let s = SMatrix::try_new(vec![vec![c(0.0, 0.0)]]).unwrap();
    assert!(s.to_y_matrix_with_diagnostics(f64::from_bits(1)).is_err());
}
