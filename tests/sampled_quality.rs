use touchstone::{
    BoundaryClassification::{AboveBoundary, BelowBoundary, NearBoundary},
    Complex, Network, NetworkBuilder, ReferenceImpedance, SMatrix, SampledQualityOptions,
    TouchstoneError,
};

fn options(tolerance: f64) -> SampledQualityOptions {
    SampledQualityOptions { tolerance }
}

fn parsed(data: &str) -> Network {
    Network::from_str("quality.s2p", &format!("# Hz S RI R 50\n{data}")).unwrap()
}

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 2e-15 * expected.abs().max(f64::MIN_POSITIVE),
        "{actual} != {expected}"
    );
}

#[test]
fn ideal_thru_attenuator_and_coherent_excitation_preserve_active_data_and_warnings() {
    let network = parsed("[Vendor Detail] ignored\n0 0 0 1 0 1 0 0 0\n1 0 0 0.5 0 0.5 0 0 0\n2 0.6 0 0.6 0 0.6 0 0.6 0\n");
    let before = network.to_touchstone_string().unwrap();
    let warnings = network.warnings.clone();
    let report = network.sampled_quality(options(1e-12)).unwrap();
    close(report.points[0].sigma_max, 1.0);
    close(report.points[1].sigma_max, 0.5);
    close(report.points[2].sigma_max, 1.2);
    close(report.points[2].excess_above_one, 0.2);
    assert_eq!(report.points[0].passivity, NearBoundary);
    assert_eq!(report.points[1].passivity, BelowBoundary);
    assert_eq!(report.points[2].passivity, AboveBoundary);
    assert_eq!(report.worst_passivity.point_index, 2);
    assert_eq!(report.worst_symmetry.point_index, 0);
    assert_eq!(report.worst_passivity.frequency_hz, 2.0);
    assert!(report.points.iter().all(|p| p.symmetry_residual == 0.0));
    assert_eq!(network.warnings, warnings);
    assert!(!warnings.is_empty());
    assert_eq!(network.to_touchstone_string().unwrap(), before);
    let s = network.s_matrix_at(2).unwrap();
    let output = s.get(1, 1).unwrap() + s.get(1, 2).unwrap();
    close(output.re * output.re, 1.44);
}

#[test]
fn independent_worst_locations_and_complex_symmetry() {
    let network =
        parsed("1 0.6 0 0.6 0 0.6 0 0.6 0\n2 0 0 0 0.8 0.3 0 0 0\n3 0 0 0 0.8 0.3 0 0 0\n");
    let report = network.sampled_quality(options(0.0)).unwrap();
    assert_eq!(report.worst_passivity.point_index, 0);
    assert_eq!(report.worst_symmetry.point_index, 1);
    close(report.points[1].sigma_max, 0.8);
    close(report.points[1].symmetry_residual, 0.3_f64.hypot(0.8));
}

#[test]
fn explicit_tolerance_inclusive_band_and_extreme_thresholds() {
    let network = Network::from_str(
        "band.s1p",
        "# Hz S RI R 50\n0 0.5 0\n1 0.75 0\n2 1 0\n3 1.25 0\n4 1.5 0\n",
    )
    .unwrap();
    let report = network.sampled_quality(options(0.25)).unwrap();
    assert_eq!(
        report
            .points
            .iter()
            .map(|p| p.passivity)
            .collect::<Vec<_>>(),
        vec![
            BelowBoundary,
            NearBoundary,
            NearBoundary,
            NearBoundary,
            AboveBoundary
        ]
    );
    assert!(report.points.iter().all(|p| p.symmetry_residual == 0.0));
    assert!(network
        .sampled_quality(options(f64::MAX))
        .unwrap()
        .points
        .iter()
        .all(|p| p.passivity == NearBoundary));
    for tolerance in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(network.sampled_quality(options(tolerance)).is_err());
    }
}

#[test]
fn scaled_analytic_norm_handles_tiny_large_complex_and_degenerate_matrices() {
    for (name, data, expected) in [
        ("zero", [[(0.0, 0.0); 2]; 2], 0.0),
        (
            "tiny",
            [[(0.0, 0.0), (1e-300, 0.0)], [(1e-300, 0.0), (0.0, 0.0)]],
            1e-300,
        ),
        (
            "subnormal",
            [[(1e-320, 0.0), (0.0, 0.0)], [(0.0, 0.0), (1e-320, 0.0)]],
            1e-320,
        ),
        ("large", [[(5e299, 0.0); 2]; 2], 1e300),
        (
            "complex_unitary",
            [[(0.0, 0.0), (0.0, 1.0)], [(0.0, -1.0), (0.0, 0.0)]],
            1.0,
        ),
        ("rank_one_complex", [[(0.3, 0.4); 2]; 2], 1.0),
    ] {
        let matrix = SMatrix::try_new(
            data.into_iter()
                .map(|row| row.into_iter().map(|(re, im)| Complex { re, im }).collect())
                .collect(),
        )
        .unwrap();
        let network = NetworkBuilder::new(format!("{name}.s2p"), 2)
            .point(0.0, matrix)
            .build()
            .unwrap();
        close(
            network.sampled_quality(options(1e-12)).unwrap().points[0].sigma_max,
            expected,
        );
    }
    let network = Network::from_str("large.s1p", "# Hz S RI R 50\n0 1e300 1e300\n").unwrap();
    close(
        network.sampled_quality(options(0.0)).unwrap().points[0].sigma_max,
        1e300_f64.hypot(1e300),
    );
    for data in [
        "7 1.7e308 1.7e308 0 0 0 0 0 0\n",
        "7 1e308 0 1e308 0 1e308 0 1e308 0\n",
        "7 0 0 -1e308 0 1e308 0 0 0\n",
    ] {
        let error = parsed(data).sampled_quality(options(0.0)).unwrap_err();
        assert!(matches!(
            error,
            TouchstoneError::SampledQuality {
                point_index: Some(0),
                ..
            }
        ));
        assert!(error.to_string().contains("7 Hz"));
    }
}

#[test]
fn real_per_port_references_supported_but_malformed_mutable_metadata_rejected() {
    let valid = parsed("[Reference] 50 75\n1 0 0 0.5 0 0.5 0 0 0\n");
    close(
        valid.sampled_quality(options(0.0)).unwrap().points[0].sigma_max,
        0.5,
    );
    for reference in [
        ReferenceImpedance::Common(f64::NAN),
        ReferenceImpedance::PerPort(vec![]),
        ReferenceImpedance::PerPort(vec![50.0]),
        ReferenceImpedance::PerPort(vec![50.0, -1.0]),
        ReferenceImpedance::PerPort(vec![50.0, f64::INFINITY]),
        ReferenceImpedance::PerPort(vec![75.0, 50.0]),
    ] {
        let mut network = valid.clone();
        network.reference_impedance = reference;
        assert!(network.sampled_quality(options(0.0)).is_err());
    }
    for rank in [-1, 0, 1, 3] {
        let mut network = valid.clone();
        network.rank = rank;
        assert!(network.sampled_quality(options(0.0)).is_err());
    }
    let mut network = valid.clone();
    network.z0 = f64::NAN;
    assert!(network.sampled_quality(options(0.0)).is_err());
    let mut common = parsed("1 0 0 1 0 1 0 0 0\n");
    common.z0 = 75.0;
    assert!(common.sampled_quality(options(0.0)).is_ok());
    let mut scalar = Network::from_str("scalar.s1p", "# Hz S RI R 50\n0 0.5 0\n").unwrap();
    scalar.rank = 2;
    assert!(scalar.sampled_quality(options(0.0)).is_err());
    let mut relabeled = valid;
    relabeled.parameter = "Y".into();
    assert!(relabeled.sampled_quality(options(0.0)).is_err());
}

#[test]
fn unsupported_semantics_nonfinite_values_and_invalid_grids_are_not_blessed() {
    for data in [
        "[Mixed-Mode Order] D1,2 C1,2\n1 0 0 1 0 1 0 0 0\n",
        "1 NaN 0 1 0 1 0 0 0\n",
        "1 0 inf 1 0 1 0 0 0\n",
        "-1 0 0 1 0 1 0 0 0\n",
        "NaN 0 0 1 0 1 0 0 0\n",
        "inf 0 0 1 0 1 0 0 0\n",
        "2 0 0 1 0 1 0 0 0\n1 0 0 1 0 1 0 0 0\n",
        "1 0 0 1 0 1 0 0 0\n1 0 0 1 0 1 0 0 0\n",
    ] {
        let network = parsed(data);
        let warnings = network.warnings.clone();
        assert!(network.sampled_quality(options(1e-12)).is_err(), "{data}");
        assert_eq!(network.warnings, warnings);
    }
    let non_s = Network::from_str("z.s1p", "# Hz Z RI R 50\n1 1 0\n").unwrap();
    assert!(non_s.sampled_quality(options(0.0)).is_err());
    let three_port = NetworkBuilder::new("three.s3p", 3)
        .point(
            0.0,
            SMatrix::try_new(vec![vec![Complex { re: 0.0, im: 0.0 }; 3]; 3]).unwrap(),
        )
        .build()
        .unwrap();
    assert!(three_port.sampled_quality(options(0.0)).is_err());
}

#[test]
fn normal_passive_fixture_is_reported_through_public_file_api() {
    let network = Network::new("files/ntwk1.s2p").unwrap();
    let report = network.sampled_quality(options(1e-9)).unwrap();
    assert_eq!(report.points.len(), network.frequencies().len());
    assert!(report
        .points
        .iter()
        .all(|point| point.sigma_max <= 1.0 + 1e-9));
}
