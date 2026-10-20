#![cfg(feature = "experimental-vector-fitting")]

use touchstone::vector_fitting::{
    FitError, FitOptions, FitStatus, InitialPoles, RationalNetworkModel,
};
use touchstone::{Complex, Network, NetworkBuilder, ReferenceImpedance, SMatrix};

fn c(re: f64, im: f64) -> Complex {
    Complex { re, im }
}
fn pair() -> InitialPoles {
    InitialPoles::ConjugatePair {
        real: -0.15,
        imaginary: 0.2,
    }
}
fn options(poles: InitialPoles) -> FitOptions {
    FitOptions {
        initial_poles: poles,
        max_iterations: 12,
    }
}

// Independent transfer equations, not evaluation through partial-fraction residues.
fn response(x: f64, kind: usize) -> Vec<Complex> {
    let z = c(0.0, x);
    match kind {
        0 => vec![c(0.15, 0.0) + c(0.18, 0.0) / (z + c(0.27, 0.0))],
        1 => {
            let q = (z + c(0.12, 0.0)) * (z + c(0.7, 0.0));
            [
                (0.1, 0.05, 0.016),
                (-0.03, 0.01, -0.009),
                (0.25, 0.08, 0.011),
                (0.0, -0.03, 0.004),
            ]
            .iter()
            .map(|&(d, a, b)| c(d, 0.0) + (z * a + c(b, 0.0)) / q)
            .collect()
        }
        3 => [(0.15, 0.18), (-0.07, -0.09), (0.31, 0.04), (-0.19, 0.12)]
            .iter()
            .map(|&(d, r)| c(d, 0.0) + c(r, 0.0) / (z + c(0.27, 0.0)))
            .collect(),
        _ => {
            let q = z * z + z * 0.032 + c(0.61 * 0.61, 0.0);
            [
                (0.13, 0.008, 0.012),
                (-0.07, 0.021, -0.004),
                (0.31, 0.034, 0.006),
                (0.19, -0.014, 0.003),
            ]
            .iter()
            .map(|&(d, a, b)| c(d, 0.0) + (z * a + c(b, 0.0)) / q)
            .collect()
        }
    }
}
fn network(
    xs: &[f64],
    scale: f64,
    amplitude: f64,
    response: impl Fn(f64) -> Vec<Complex>,
) -> Network {
    let rank = if response(xs[0]).len() == 1 { 1 } else { 2 };
    let mut builder = NetworkBuilder::new(format!("analytic.s{rank}p"), rank);
    for &x in xs {
        let values: Vec<_> = response(x).into_iter().map(|v| v * amplitude).collect();
        builder = builder.point(
            x * scale,
            SMatrix::try_new(values.chunks(rank).map(|r| r.to_vec()).collect()).unwrap(),
        );
    }
    builder.build().unwrap()
}
fn grid(count: usize) -> Vec<f64> {
    (0..count)
        .map(|i| 0.05 + 0.95 * i as f64 / (count - 1) as f64)
        .collect()
}
fn withheld(xs: &[f64]) -> Vec<f64> {
    xs.windows(2)
        .flat_map(|w| [0.125, 0.375, 0.625, 0.875].map(|t| w[0] + t * (w[1] - w[0])))
        .collect()
}

#[test]
fn analytical_public_workflows_recover_both_topology_transitions_and_all_entries() {
    for (kind, seeds) in [
        (0, vec![InitialPoles::OneReal(-1.3)]),
        (1, vec![pair(), InitialPoles::TwoReal(-0.03, -1.5)]),
        (2, vec![pair(), InitialPoles::TwoReal(-0.03, -1.5)]),
        (3, vec![InitialPoles::OneReal(-1.3)]),
    ] {
        for seed in seeds {
            let xs = grid(41);
            let training = network(&xs, 1e9, 1.0, |x| response(x, kind));
            let heldout = network(&withheld(&xs), 1e9, 1.0, |x| response(x, kind));
            let before = training.to_touchstone_string().unwrap();
            let model = RationalNetworkModel::fit(&training, options(seed)).unwrap();
            assert_eq!(model.report().status, FitStatus::Converged);
            assert_eq!(model.report().iterations.len(), 2);
            let assessment = model.assess(&heldout).unwrap();
            assert!(assessment.model.aggregate.max < 2e-12, "{assessment:?}");
            assert!(assessment.model.aggregate.rms < assessment.ri_linear.aggregate.rms * 1e-6);
            assert!(assessment.model.per_entry.iter().all(|e| e.max < 2e-12));
            assert_eq!(training.to_touchstone_string().unwrap(), before);
            assert_eq!(
                model.reference_impedance(),
                &ReferenceImpedance::Common(50.0)
            );
            if kind == 1 {
                assert!(model.normalized_poles().iter().all(|p| p.im == 0.0));
            }
            if kind == 2 {
                assert!(model.normalized_poles()[0].im > 0.0);
            }
            assert!(model.evaluate(0.0).is_err());
            assert!(model.evaluate(1.001e9).is_err());
            assert!(model.evaluate(f64::NAN).is_err());
            assert!(model.assess(&training).is_err());
        }
    }
}

#[test]
fn finite_scaling_dc_and_maximum_dimensions() {
    for (scale, amplitude, count) in [
        (1e-300, 1e-300, 41),
        (1e-200, 1e-200, 41),
        (1e200, 1e200, 41),
        (1e300, 1e300, 41),
        (1e9, 1.0, 8),
        (1e9, 1.0, 512),
    ] {
        let mut xs = grid(count);
        xs[0] = -0.0;
        let training = network(&xs, scale, amplitude, |x| response(x, 2));
        let model = RationalNetworkModel::fit(&training, options(pair())).unwrap();
        assert_eq!(model.report().status, FitStatus::Converged);
        let validation = network(&withheld(&xs), scale, amplitude, |x| response(x, 2));
        assert!(
            model
                .assess(&validation)
                .unwrap()
                .model
                .aggregate
                .relative_l2
                .unwrap()
                < 1e-10
        );
        assert_eq!(model.report().real_rows, 8 * count);
        assert_eq!(model.report().relocation_columns, 14);
        let same_dc = network(&[0.0], scale, amplitude, |x| response(x, 2));
        assert!(model.assess(&same_dc).is_err());
    }
}

#[test]
fn subnormal_errors_match_independently_normalized_public_predictions() {
    let xs: Vec<_> = (0..81).map(|i| i as f64 / 80.0).collect();
    let mids: Vec<_> = (0..80).map(|i| (i as f64 + 0.5) / 80.0).collect();
    for amplitude in [1e-300, 1e-320, 1e-322, 5e-323] {
        let training = network(&xs, 1.0, amplitude, |x| response(x, 0));
        let heldout = network(&mids, 1.0, amplitude, |x| response(x, 0));
        let model =
            RationalNetworkModel::fit(&training, options(InitialPoles::OneReal(-1.3))).unwrap();
        assert_eq!(model.report().status, FitStatus::Converged);
        let report = model.assess(&heldout).unwrap();
        for (reference, metrics) in [
            (&training, &model.report().training_error),
            (&heldout, &report.model),
        ] {
            let values: Vec<_> = reference
                .iter_points()
                .map(|p| {
                    let truth = p.s().get(1, 1).unwrap();
                    let prediction = model.evaluate(p.frequency()).unwrap().get(1, 1).unwrap();
                    (prediction - truth, truth)
                })
                .collect();
            // Two external choices of units: generating amplitude and measured peak.
            // Ordinary sums of squares are safe in either normalized coordinate system.
            let peak = values
                .iter()
                .fold(0.0_f64, |a, (_, v)| a.max(v.re.abs()).max(v.im.abs()));
            for units in [amplitude, peak] {
                let error_energy: f64 = values
                    .iter()
                    .map(|(e, _)| (e.re / units).powi(2) + (e.im / units).powi(2))
                    .sum();
                let reference_energy: f64 = values
                    .iter()
                    .map(|(_, v)| (v.re / units).powi(2) + (v.im / units).powi(2))
                    .sum();
                let expected_relative = (error_energy / reference_energy).sqrt();
                let expected_rms = (error_energy / values.len() as f64).sqrt() * units;
                for actual in [metrics.aggregate, metrics.per_entry[0]] {
                    assert!(
                        (actual.relative_l2.unwrap() - expected_relative).abs()
                            <= 1e-14 * expected_relative,
                        "{amplitude:e}: {actual:?} vs {expected_relative:e}"
                    );
                    // Physical RMS rounds to f64; allow one subnormal quantum.
                    assert!(
                        (actual.rms - expected_rms).abs()
                            <= (expected_rms * 1e-14).max(f64::from_bits(1))
                    );
                }
                if amplitude < 1e-310 {
                    assert!(expected_relative > 1e-5);
                }
            }
        }
    }
}

#[test]
fn minimum_subnormal_and_zero_references_keep_absolute_and_relative_meanings() {
    let xs: Vec<_> = (0..81).map(|i| i as f64 / 80.0).collect();
    let mids: Vec<_> = (0..80).map(|i| (i as f64 + 0.5) / 80.0).collect();
    let quantum = f64::from_bits(1);
    let training = network(&xs, 1.0, quantum, |_| vec![c(1.0, 0.0)]);
    let model =
        RationalNetworkModel::fixed_pole_baseline(&training, options(InitialPoles::OneReal(-1.3)))
            .unwrap();
    assert_eq!(
        model.report().training_error.aggregate.relative_l2,
        Some(0.0)
    );
    assert_eq!(model.report().training_error.aggregate.rms, 0.0);
    for &frequency in &mids {
        assert_eq!(
            model.evaluate(frequency).unwrap().get(1, 1).unwrap(),
            c(quantum, 0.0)
        );
    }
    for truth in [0.0, quantum, 2.0 * quantum] {
        let heldout = network(&mids, 1.0, truth, |_| vec![c(1.0, 0.0)]);
        let assessment = model.assess(&heldout).unwrap();
        // Constant errors have exactly known RMS/max, independent of accumulation.
        let expected_absolute = (quantum - truth).abs();
        let expected_relative = if truth == 0.0 {
            None
        } else {
            Some(((quantum / quantum) - (truth / quantum)).abs() / (truth / quantum))
        };
        for errors in [&assessment.model, &assessment.ri_linear] {
            for actual in [errors.aggregate, errors.per_entry[0]] {
                assert_eq!(actual.rms, expected_absolute);
                assert_eq!(actual.max, expected_absolute);
                assert_eq!(actual.relative_l2, expected_relative);
            }
        }
    }
}

#[test]
fn finite_rms_does_not_require_representable_reference_norm_and_overflow_is_explicit() {
    let xs = grid(81);
    let mids = withheld(&xs);
    let training = network(&xs, 1.0, f64::MAX * 0.75, |_| vec![c(1.0, 0.0)]);
    let model =
        RationalNetworkModel::fixed_pole_baseline(&training, options(InitialPoles::OneReal(-1.3)))
            .unwrap();
    assert!(model.report().training_error.aggregate.relative_l2.unwrap() < 1e-14);
    let zero = network(&mids, 1.0, 0.0, |_| vec![c(1.0, 0.0)]);
    let assessment = model.assess(&zero).unwrap();
    assert!((assessment.model.aggregate.rms / f64::MAX - 0.75).abs() < 1e-14);
    assert_eq!(assessment.model.aggregate.relative_l2, None);
    let tiny = network(&mids, 1.0, f64::from_bits(1), |_| vec![c(1.0, 0.0)]);
    assert!(matches!(model.assess(&tiny), Err(FitError::Numerical)));
    let opposite = network(&mids, 1.0, f64::MAX, |_| vec![c(-1.0, 0.0)]);
    assert!(matches!(model.assess(&opposite), Err(FitError::Numerical)));
}

#[test]
fn rank_failures_are_not_order_reduction_or_invented_poles() {
    let xs = grid(41);
    for training in [
        network(&xs, 1.0, 1.0, |_| {
            vec![c(0.0, 0.0), c(1.0, 0.0), c(1.0, 0.0), c(0.0, 0.0)]
        }),
        network(&xs, 1.0, 1.0, |x| response(x, 0)),
        network(&xs, 1.0, 0.0, |x| response(x, 2)),
    ] {
        assert!(matches!(
            RationalNetworkModel::fit(&training, options(pair())),
            Err(FitError::RankDeficient { .. })
        ));
    }
    // A dynamic signal well below the absolute-identification scale is not rescued by arbitrary regularization.
    let near_direct = network(&xs, 1.0, 1.0, |x| {
        response(x, 2)
            .iter()
            .map(|v| c(1.0, 0.0) + *v * 1e-13)
            .collect()
    });
    assert!(matches!(
        RationalNetworkModel::fit(&near_direct, options(pair())),
        Err(FitError::RankDeficient { .. })
    ));
}

#[test]
fn nonconvergence_and_stability_are_not_accuracy_claims() {
    let xs = grid(41);
    let training = network(&xs, 1.0, 1.0, |x| response(x, 2));
    let one_step = RationalNetworkModel::fit(
        &training,
        FitOptions {
            max_iterations: 1,
            ..options(pair())
        },
    )
    .unwrap();
    assert_eq!(one_step.report().status, FitStatus::IterationLimit);
    assert!(one_step.report().training_error.aggregate.max < 1e-10);
    let unstable = network(&xs, 1.0, 1.0, |x| {
        vec![c(0.1, 0.0) + c(0.1, 0.0) / c(-0.2, x)]
    });
    let reflected =
        RationalNetworkModel::fit(&unstable, options(InitialPoles::OneReal(-0.7))).unwrap();
    assert_eq!(reflected.report().status, FitStatus::IterationLimit);
    assert!(
        reflected
            .report()
            .iterations
            .last()
            .unwrap()
            .reflected_poles
            > 0
    );
    assert!(
        reflected
            .report()
            .iterations
            .last()
            .unwrap()
            .max_sigma_minus_one
            > 1e-7
    );
    let fixed = RationalNetworkModel::fixed_pole_baseline(&training, options(pair())).unwrap();
    assert_eq!(fixed.report().status, FitStatus::FixedPolesNotRelocated);
    assert!(fixed.report().training_error.aggregate.rms > 0.01);
}

#[test]
fn strict_options_counts_and_pole_bounds() {
    let training = network(&grid(41), 1.0, 1.0, |x| response(x, 2));
    for limit in [0, 13, usize::MAX] {
        assert!(RationalNetworkModel::fit(
            &training,
            FitOptions {
                max_iterations: limit,
                ..options(pair())
            }
        )
        .is_err());
    }
    for poles in [
        InitialPoles::OneReal(0.0),
        InitialPoles::OneReal(f64::NAN),
        InitialPoles::OneReal(-1e5),
        InitialPoles::OneReal(-1e-11),
        InitialPoles::TwoReal(-0.2, -0.200000001),
        InitialPoles::ConjugatePair {
            real: -0.1,
            imaginary: 0.0,
        },
        InitialPoles::ConjugatePair {
            real: -0.1,
            imaginary: f64::INFINITY,
        },
    ] {
        assert!(matches!(
            RationalNetworkModel::fit(&training, options(poles)),
            Err(FitError::InvalidPoles)
        ));
    }
    for count in [7, 513] {
        assert!(RationalNetworkModel::fit(
            &network(&grid(count), 1.0, 1.0, |x| response(x, 2)),
            options(pair())
        )
        .is_err());
    }
    let narrow: Vec<_> = (0..41).map(|i| 0.6 + i as f64 * 1e-10).collect();
    assert!(RationalNetworkModel::fit(
        &network(&narrow, 1.0, 1.0, |x| response(x, 2)),
        options(pair())
    )
    .is_err());
    let repeated = network(&grid(41), 1.0, 1.0, |x| {
        vec![c(0.1, 0.0) + c(0.03, 0.0) / (c(0.3, x) * c(0.3, x))]
    });
    assert!(matches!(
        RationalNetworkModel::fit(&repeated, options(pair())),
        Err(FitError::InvalidPoles)
    ));
}

#[test]
fn current_metadata_is_validated_and_model_snapshots_survive_later_edits() {
    let training = network(&grid(41), 1.0, 1.0, |x| response(x, 2));
    for edit in 0..4 {
        let mut bad = training.clone();
        match edit {
            0 => bad.rank = 1,
            1 => bad.parameter = "Y".into(),
            2 => bad.z0 = 75.0,
            _ => bad.reference_impedance = ReferenceImpedance::PerPort(vec![50.0]),
        }
        assert!(RationalNetworkModel::fit(&bad, options(pair())).is_err());
        let model = RationalNetworkModel::fit(&training, options(pair())).unwrap();
        assert!(model.assess(&bad).is_err());
    }
    let text = training.to_touchstone_string().unwrap();
    let ytext = text.replace(" S RI ", " Y RI ");
    let non_s = Network::from_str("non-s.s2p", &ytext).unwrap();
    assert_eq!(non_s.parameter, "Y");
    assert!(matches!(
        RationalNetworkModel::fit(&non_s, options(pair())),
        Err(FitError::SourceSemantics(_))
    ));
    let mixed = format!("[Mixed-Mode Order] D1,2 C1,2\n{text}");
    let mixed = Network::from_str("mixed.s2p", &mixed).unwrap();
    assert!(matches!(
        RationalNetworkModel::fit(&mixed, options(pair())),
        Err(FitError::SourceSemantics(_))
    ));
    let warned = format!("[Unknown Vendor Marker] yes\n{text}");
    let mut warned = Network::from_str("warned.s2p", &warned).unwrap();
    // Coherent current metadata is accepted; changing it is not a physical conversion.
    warned.z0 = 75.0;
    warned.reference_impedance = ReferenceImpedance::Common(75.0);
    let model = RationalNetworkModel::fit(&warned, options(pair())).unwrap();
    let mut heldout = network(&withheld(&grid(41)), 1.0, 1.0, |x| response(x, 2));
    heldout.z0 = 75.0;
    heldout.reference_impedance = ReferenceImpedance::Common(75.0);
    let before = model.assess(&heldout).unwrap();
    let prediction = model.evaluate(0.61).unwrap();
    warned.warnings.clear();
    warned.parameter = "Y".into();
    warned.rank = 1;
    warned.z0 = 100.0;
    warned.reference_impedance = ReferenceImpedance::Common(100.0);
    assert_eq!(model.source_warnings().len(), 1);
    assert_eq!(
        model.reference_impedance(),
        &ReferenceImpedance::Common(75.0)
    );
    assert_eq!(model.evaluate(0.61).unwrap(), prediction);
    let after = model.assess(&heldout).unwrap();
    assert_eq!(
        before.ri_linear.aggregate.rms,
        after.ri_linear.aggregate.rms
    );
    assert_eq!(before.model.aggregate.rms, after.model.aggregate.rms);
    assert!(model.assess(&non_s).is_err());
    assert!(model.assess(&mixed).is_err());
}

#[test]
fn scalar_conjugate_model_assessment_and_underorder_quality_are_separate() {
    let xs = grid(41);
    let training = network(&xs, 1e9, 1.0, |x| vec![response(x, 2)[0]]);
    let heldout = network(&withheld(&xs), 1e9, 1.0, |x| vec![response(x, 2)[0]]);
    let pair_model = RationalNetworkModel::fit(&training, options(pair())).unwrap();
    assert_eq!(pair_model.report().status, FitStatus::Converged);
    assert!(pair_model.assess(&heldout).unwrap().model.aggregate.max < 1e-12);
    let underordered =
        RationalNetworkModel::fit(&training, options(InitialPoles::OneReal(-0.15))).unwrap();
    assert_eq!(underordered.report().status, FitStatus::Converged);
    assert!(underordered.assess(&heldout).unwrap().model.aggregate.max > 0.1);

    let other_reference_text = heldout
        .to_touchstone_string()
        .unwrap()
        .replace("R 50", "R 75");
    let other_reference = Network::from_str("other.s1p", &other_reference_text).unwrap();
    assert!(pair_model.assess(&other_reference).is_err());
    let other_rank = network(&withheld(&xs), 1e9, 1.0, |x| response(x, 2));
    assert!(pair_model.assess(&other_rank).is_err());
    let outside = network(&[0.04], 1e9, 1.0, |x| vec![response(x, 2)[0]]);
    assert!(pair_model.assess(&outside).is_err());
    let one_point = network(&[0.61], 1e9, 1.0, |x| vec![response(x, 2)[0]]);
    assert_eq!(pair_model.assess(&one_point).unwrap().points, 1);
}

#[test]
fn matching_per_port_references_remain_model_metadata() {
    let xs = grid(41);
    let training = network(&xs, 1e9, 1.0, |x| response(x, 2));
    let heldout = network(&withheld(&xs), 1e9, 1.0, |x| response(x, 2));
    let with_references = |source: &Network| {
        let text = source.to_touchstone_string().unwrap().replace(
            "[Number of Ports] 2",
            "[Number of Ports] 2\n[Reference] 50 75",
        );
        Network::from_str("per-port.s2p", &text).unwrap()
    };
    let training = with_references(&training);
    let heldout = with_references(&heldout);
    let model = RationalNetworkModel::fit(&training, options(pair())).unwrap();
    assert_eq!(
        model.reference_impedance(),
        &ReferenceImpedance::PerPort(vec![50.0, 75.0])
    );
    assert!(model.assess(&heldout).unwrap().model.aggregate.max < 1e-12);
}

#[test]
fn maximum_workload_delay_returns_bounded_diagnostic_candidate() {
    let xs = grid(512);
    let cable = |x: f64| {
        let phase = 24.0 * std::f64::consts::PI * x;
        let h = c(phase.cos(), -phase.sin()) * 0.9;
        vec![c(0.0, 0.0), h, h, c(0.0, 0.0)]
    };
    let training = network(&xs, 1e9, 1.0, cable);
    let heldout = network(&withheld(&xs), 1e9, 1.0, cable);
    let model = RationalNetworkModel::fit(&training, options(pair())).unwrap();
    assert_eq!(model.report().status, FitStatus::IterationLimit);
    assert_eq!(model.report().iterations.len(), 12);
    assert_eq!(model.report().real_rows, 4096);
    assert_eq!(model.report().relocation_columns, 14);
    let assessment = model.assess(&heldout).unwrap();
    assert!(assessment.model.aggregate.rms > 0.5);
    assert!(assessment.model.aggregate.rms > assessment.ri_linear.aggregate.rms * 10.0);
}

#[test]
fn malformed_parsed_grids_are_rejected_at_fit_boundary() {
    for frequencies in [
        vec![1.0; 8],
        vec![8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0],
        vec![-1.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
        vec![f64::NAN, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
    ] {
        let mut text = String::from("# Hz S RI R 50\n");
        for f in frequencies {
            text.push_str(&format!("{f} 0.5 0.1\n"));
        }
        let parsed = Network::from_str("invalid.s1p", &text).unwrap();
        assert!(RationalNetworkModel::fit(&parsed, options(InitialPoles::OneReal(-0.2))).is_err());
    }
    let text =
        "# Hz S RI R 50\n1 inf 0\n2 0.5 0\n3 0.5 0\n4 0.5 0\n5 0.5 0\n6 0.5 0\n7 0.5 0\n8 0.5 0\n";
    if let Ok(parsed) = Network::from_str("infinite.s1p", text) {
        assert!(RationalNetworkModel::fit(&parsed, options(InitialPoles::OneReal(-0.2))).is_err());
    }
}
