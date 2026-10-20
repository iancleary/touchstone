use touchstone::{
    Complex, MixedModeError, MixedModeLayout, MixedModeView, ModePort::*, Network, NetworkBuilder,
    PortPair, ReferenceImpedance, SMatrix, TouchstoneWarning,
};

const ZERO: Complex = Complex { re: 0.0, im: 0.0 };
const ONE: Complex = Complex { re: 1.0, im: 0.0 };

fn pair(positive: usize, negative: usize) -> PortPair {
    PortPair { positive, negative }
}

fn layout4(reverse: bool) -> MixedModeLayout {
    let a = pair(1, 2);
    let b = if reverse { pair(4, 3) } else { pair(3, 4) };
    MixedModeLayout::new(
        4,
        vec![Differential(a), Differential(b), Common(a), Common(b)],
    )
    .unwrap()
}

fn paths(a: Complex, b: Complex) -> Network {
    let mut data = vec![vec![ZERO; 4]; 4];
    data[2][0] = a;
    data[0][2] = a;
    data[3][1] = b;
    data[1][3] = b;
    NetworkBuilder::new("paths.s4p", 4)
        .point(1e9, SMatrix::try_new(data).unwrap())
        .build()
        .unwrap()
}

fn close(actual: Complex, expected: Complex) {
    assert!(
        (actual.re - expected.re).abs() < 1e-14,
        "{actual:?} != {expected:?}"
    );
    assert!(
        (actual.im - expected.im).abs() < 1e-14,
        "{actual:?} != {expected:?}"
    );
}

#[test]
fn balanced_thru_has_unit_same_mode_transmission_and_zero_conversion() {
    let source = paths(ONE, ONE);
    let layout = layout4(false);
    let view = MixedModeView::new(&source, layout.clone()).unwrap();
    assert_eq!(view.layout(), &layout);
    assert_eq!(view.modal_references(), &[100.0, 100.0, 25.0, 25.0]);
    assert_eq!(view.frequencies(), &[1e9]);
    let s = view.s_matrix_at(0).unwrap();
    for to in 1..=4 {
        for from in 1..=4 {
            close(
                s.get(to, from).unwrap(),
                if matches!((to, from), (1, 2) | (2, 1) | (3, 4) | (4, 3)) {
                    ONE
                } else {
                    ZERO
                },
            );
        }
    }
    assert_eq!(source.s_matrix_at(0).unwrap().get(3, 1).unwrap(), ONE);
    assert!(view.s_matrix_at(1).is_err());
    assert!(s.get(0, 1).is_err());
    assert!(s.get(1, 5).is_err());
}

#[test]
fn asymmetric_complex_paths_and_reversed_polarity_have_analytic_conversion() {
    // a = .8 + .2j; b = .4 - .6j. Sum/2 = .6 - .2j; difference/2 = .2 + .4j.
    let source = paths(Complex { re: 0.8, im: 0.2 }, Complex { re: 0.4, im: -0.6 });
    for reverse in [false, true] {
        let view = MixedModeView::new(&source, layout4(reverse)).unwrap();
        let s = view.s_matrix_at(0).unwrap();
        let sign = if reverse { -1.0 } else { 1.0 };
        close(
            s.get(2, 1).unwrap(),
            Complex {
                re: sign * 0.6,
                im: sign * -0.2,
            },
        );
        close(
            s.get(2, 3).unwrap(),
            Complex {
                re: sign * 0.2,
                im: sign * 0.4,
            },
        );
        close(s.get(4, 1).unwrap(), Complex { re: 0.2, im: 0.4 });
        close(s.get(4, 3).unwrap(), Complex { re: 0.6, im: -0.2 });
        close(s.get(1, 2).unwrap(), s.get(2, 1).unwrap());
    }
}

#[test]
fn generalized_ordering_unpaired_port_and_different_pair_references() {
    let mut source = NetworkBuilder::new("general.s3p", 3)
        .point(
            0.0,
            SMatrix::try_new(vec![
                vec![ZERO, ZERO, Complex { re: 0.2, im: 0.4 }],
                vec![ZERO, ZERO, Complex { re: -0.2, im: 0.0 }],
                vec![ZERO, ZERO, ONE],
            ])
            .unwrap(),
        )
        .build()
        .unwrap();
    source.reference_impedance = ReferenceImpedance::PerPort(vec![75.0, 75.0, 30.0]);
    let p = pair(1, 2);
    let view = MixedModeView::new(
        &source,
        MixedModeLayout::new(3, vec![Common(p), SingleEnded(3), Differential(p)]).unwrap(),
    )
    .unwrap();
    assert_eq!(view.modal_references(), &[37.5, 30.0, 150.0]);
    let s = view.s_matrix_at(0).unwrap();
    close(
        s.get(3, 2).unwrap(),
        Complex { re: 0.4, im: 0.4 } * std::f64::consts::FRAC_1_SQRT_2,
    );
    close(
        s.get(1, 2).unwrap(),
        Complex { re: 0.0, im: 0.4 } * std::f64::consts::FRAC_1_SQRT_2,
    );
    close(s.get(2, 2).unwrap(), ONE);

    let mut source = paths(ONE, ONE);
    source.reference_impedance = ReferenceImpedance::PerPort(vec![50.0, 50.0, 75.0, 75.0]);
    let view = MixedModeView::new(&source, layout4(false)).unwrap();
    assert_eq!(view.modal_references(), &[100.0, 150.0, 25.0, 37.5]);
}

#[test]
fn all_single_ended_permutation_retains_original_references_and_values() {
    let source = Network::from_str(
        "single.s2p",
        "# Hz S RI R 50\n1 0.1 0.2 0.3 0.4 0.5 0.6 0.7 0.8\n",
    )
    .unwrap();
    let mut source = source;
    // Unpaired references have no 2R/R/2 representability constraint.
    source.reference_impedance = ReferenceImpedance::PerPort(vec![f64::MAX, f64::from_bits(1)]);
    let view = MixedModeView::new(
        &source,
        MixedModeLayout::new(2, vec![SingleEnded(2), SingleEnded(1)]).unwrap(),
    )
    .unwrap();
    assert_eq!(view.modal_references(), &[f64::from_bits(1), f64::MAX]);
    let s = view.s_matrix_at(0).unwrap();
    let original = source.s_matrix_at(0).unwrap();
    for to in 1..=2 {
        for from in 1..=2 {
            assert_eq!(
                s.get(to, from).unwrap(),
                original.get(3 - to, 3 - from).unwrap()
            );
        }
    }
}

#[test]
fn invalid_layouts_are_rejected_without_panics() {
    let p = pair(1, 2);
    for (rank, modes) in [
        (0, vec![]),
        (2, vec![SingleEnded(0), SingleEnded(2)]),
        (2, vec![SingleEnded(1), SingleEnded(3)]),
        (2, vec![SingleEnded(1), SingleEnded(1)]),
        (2, vec![Differential(p)]),
        (2, vec![Differential(p), Differential(p)]),
        (2, vec![Common(p), Common(p)]),
        (2, vec![Differential(p), Common(pair(2, 1))]),
        (2, vec![Differential(pair(1, 1)), Common(pair(1, 1))]),
        (
            2,
            vec![
                Differential(pair(1, usize::MAX)),
                Common(pair(1, usize::MAX)),
            ],
        ),
        (3, vec![Differential(p), Common(p), SingleEnded(2)]),
        (
            4,
            vec![
                Differential(p),
                Common(p),
                Differential(pair(2, 3)),
                Common(pair(2, 3)),
            ],
        ),
    ] {
        assert!(matches!(
            MixedModeLayout::new(rank, modes),
            Err(MixedModeError::InvalidLayout(_))
        ));
    }
}

#[test]
fn source_mutable_metadata_and_effective_references_are_checked() {
    let mut source = paths(ONE, ONE);
    source.z0 = 75.0;
    assert_eq!(
        MixedModeView::new(&source, layout4(false))
            .unwrap()
            .modal_references(),
        &[150.0, 150.0, 37.5, 37.5]
    );
    for rank in [-1, 0, 3, 5] {
        source.rank = rank;
        assert!(MixedModeView::new(&source, layout4(false)).is_err());
    }
    // Layout agrees with mutated public rank, but not the stored matrix rank.
    source.rank = 3;
    assert!(MixedModeView::new(
        &source,
        MixedModeLayout::new(3, vec![SingleEnded(1), SingleEnded(2), SingleEnded(3)]).unwrap()
    )
    .is_err());
    source.rank = 4;
    source.parameter = "Y".into();
    assert!(matches!(
        MixedModeView::new(&source, layout4(false)),
        Err(MixedModeError::UnsupportedSource(_))
    ));
    source.parameter = "S".into();
    for r in [
        0.0,
        -1.0,
        f64::NAN,
        f64::INFINITY,
        f64::MAX,
        f64::from_bits(1),
    ] {
        source.z0 = r;
        assert!(matches!(
            MixedModeView::new(&source, layout4(false)),
            Err(MixedModeError::InvalidReference(_))
        ));
    }
    for refs in [
        vec![],
        vec![50.0],
        vec![50.0; 5],
        vec![50.0, 51.0, 50.0, 50.0],
        vec![50.0, 50.0, 0.0, 0.0],
        vec![f64::NAN; 4],
    ] {
        source.reference_impedance = ReferenceImpedance::PerPort(refs);
        assert!(MixedModeView::new(&source, layout4(false)).is_err());
    }
}

#[test]
fn parser_diagnostics_preserved_but_mixed_mode_declarations_block_double_transform() {
    let source = Network::from_str(
        "warn.s2p",
        "[Other] foo\n# Hz S RI R 50\n1 0 0 1 0 1 0 0 0\n",
    )
    .unwrap();
    let p = pair(1, 2);
    let layout = MixedModeLayout::new(2, vec![Differential(p), Common(p)]).unwrap();
    let before = source.to_touchstone_string().unwrap();
    let view = MixedModeView::new(&source, layout.clone()).unwrap();
    assert_eq!(view.source().warnings, source.warnings);
    assert!(matches!(
        view.source().warnings[0],
        TouchstoneWarning::UnknownKeywordIgnored { .. }
    ));
    assert_eq!(view.source().to_touchstone_string().unwrap(), before);
    let source = Network::from_str(
        "mixed.s2p",
        "# Hz S RI R 50\n[Mixed-Mode Order] D1,2 C1,2\n1 0 0 1 0 1 0 0 0\n",
    )
    .unwrap();
    assert!(matches!(
        MixedModeView::new(&source, layout),
        Err(MixedModeError::UnsupportedSource(_))
    ));
}

#[test]
fn invalid_grids_and_nonfinite_source_values_are_rejected() {
    let p = pair(1, 2);
    let layout = MixedModeLayout::new(2, vec![Differential(p), Common(p)]).unwrap();
    for data in [
        "-1 0 0 0 0 0 0 0 0\n",
        "NaN 0 0 0 0 0 0 0 0\n",
        "1 0 0 0 0 0 0 0 0\n1 0 0 0 0 0 0 0 0\n",
        "2 0 0 0 0 0 0 0 0\n1 0 0 0 0 0 0 0 0\n",
        "1 NaN 0 0 0 0 0 0 0\n",
    ] {
        let source = Network::from_str("invalid.s2p", &format!("# Hz S RI R 50\n{data}")).unwrap();
        assert!(MixedModeView::new(&source, layout.clone()).is_err());
    }
}

#[test]
fn normalized_transform_avoids_cancellable_overflow_but_rejects_nonfinite_output() {
    let p = pair(1, 2);
    let layout = MixedModeLayout::new(2, vec![Differential(p), Common(p)]).unwrap();
    let max = Complex {
        re: f64::MAX,
        im: -f64::MAX,
    };
    let source = NetworkBuilder::new("extreme.s2p", 2)
        .point(
            0.0,
            SMatrix::try_new(vec![vec![max, max], vec![max, -max]]).unwrap(),
        )
        .build()
        .unwrap();
    let view = MixedModeView::new(&source, layout.clone()).unwrap();
    let s = view.s_matrix_at(0).unwrap();
    for row in &s.data {
        for z in row {
            assert!(z.re.is_finite() && z.im.is_finite());
        }
    }
    assert!((s.get(2, 2).unwrap().re / f64::MAX - 1.0).abs() < 1e-14);
    let source = NetworkBuilder::new("overflow.s2p", 2)
        .point(
            0.0,
            SMatrix::try_new(vec![vec![max, max], vec![max, max]]).unwrap(),
        )
        .build()
        .unwrap();
    let view = MixedModeView::new(&source, layout).unwrap();
    assert!(view.s_matrix_at(0).is_err());
    assert!(view.iter_matrices().next().unwrap().is_err());
}

#[test]
fn compensated_transform_preserves_small_common_mode_after_large_cancellation() {
    let source = NetworkBuilder::new("cancellation.s2p", 2)
        .point(
            0.0,
            SMatrix::try_new(vec![
                vec![Complex { re: 1e300, im: 0.0 }, ONE],
                vec![
                    Complex {
                        re: -1e300,
                        im: 0.0,
                    },
                    ONE,
                ],
            ])
            .unwrap(),
        )
        .build()
        .unwrap();
    let p = pair(1, 2);
    let view = MixedModeView::new(
        &source,
        MixedModeLayout::new(2, vec![Differential(p), Common(p)]).unwrap(),
    )
    .unwrap();
    // Scc = (1e300 + 1 - 1e300 + 1)/2 = 1, not an f64-derived oracle.
    assert_eq!(view.s_matrix_at(0).unwrap().get(2, 2).unwrap().re, 1.0);
}

#[test]
fn repeated_lazy_reads_equal_explicit_owned_cache() {
    let source = paths(Complex { re: 0.8, im: 0.2 }, Complex { re: 0.4, im: -0.6 });
    let view = MixedModeView::new(&source, layout4(false)).unwrap();
    let cache = view.iter_matrices().collect::<Result<Vec<_>, _>>().unwrap();
    for _ in 0..3 {
        assert_eq!(
            view.iter_matrices().collect::<Result<Vec<_>, _>>().unwrap(),
            cache
        );
    }
}
