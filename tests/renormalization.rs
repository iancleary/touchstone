use touchstone::{Network, ReferenceImpedance, TouchstoneError, TouchstoneWarning};

fn one(s: f64, z: f64) -> Network {
    Network::from_str("load.s1p", &format!("# Hz S RI R {z}\n1 {s} 0\n")).unwrap()
}

fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 2e-14, "{actual} != {expected}");
}

#[test]
fn analytic_loads_and_ideal_boundaries() {
    for (s, expected) in [(0.2, 0.0), (0.0, -0.2), (1.0, 1.0), (-1.0, -1.0)] {
        let source = one(s, 50.0);
        let output = source.renormalize(75.0).unwrap();
        near(output.try_s_ri_at(0, 1, 1).unwrap().re, expected);
        assert_eq!(output.z0, 75.0);
        assert_eq!(output.reference_impedance, ReferenceImpedance::Common(75.0));
        assert_eq!(source.try_s_ri_at(0, 1, 1).unwrap().re, s);
        assert_eq!(source.z0, 50.0);
    }
}

#[test]
fn independent_complex_load_and_effective_common_reference() {
    // A 75+j25 ohm load: (Z-50)/(Z+50)=(3+j2)/13;
    // at 75 ohms the independent result is (1+j6)/37.
    let source = Network::from_str(
        "reactive.s1p",
        &format!("# Hz S RI R 50\n1 {} {}\n", 3.0 / 13.0, 2.0 / 13.0),
    )
    .unwrap();
    let result = source
        .renormalize(75.0)
        .unwrap()
        .try_s_ri_at(0, 1, 1)
        .unwrap();
    near(result.re, 1.0 / 37.0);
    near(result.im, 6.0 / 37.0);
    // Common metadata's effective source follows the mutable compatibility z0.
    let mut effective = one(0.0, 50.0);
    effective.z0 = 75.0;
    let result = effective.renormalize(50.0).unwrap();
    near(result.try_s_ri_at(0, 1, 1).unwrap().re, 0.2);
    assert_eq!(result.reference_impedance, ReferenceImpedance::Common(50.0));
}

#[test]
fn independent_resistor_t_circuit() {
    // Physical T: series arms 75 and 25 ohms, common shunt arm 25 ohms.
    // V1=100 I1+25 I2, V2=25 I1+50 I2. Analytic port-wave solution:
    // S11=((100-r)(50+r)-625)/D, S22=((50-r)(100+r)-625)/D,
    // S12=S21=50r/D, D=(100+r)(50+r)-625.
    let source = Network::from_str(
        "resistors.s2p",
        &format!(
            "# MHz S RI R 50\n1 {} 0 {} 0 {} 0 {} 0\n",
            7.0 / 23.0,
            4.0 / 23.0,
            4.0 / 23.0,
            -1.0 / 23.0
        ),
    )
    .unwrap();
    let output = source.renormalize(75.0).unwrap();
    for (row, column, expected) in [
        (1, 1, 2.0 / 17.0),
        (1, 2, 3.0 / 17.0),
        (2, 1, 3.0 / 17.0),
        (2, 2, -4.0 / 17.0),
    ] {
        let value = output.try_s_ri_at(0, row, column).unwrap();
        near(value.re, expected);
        near(value.im, 0.0);
    }
    let detour = source
        .z_matrix_at(0)
        .unwrap()
        .to_s_matrix_from_z(75.0)
        .unwrap();
    for row in 1..=2 {
        for column in 1..=2 {
            near(
                output.try_s_ri_at(0, row, column).unwrap().re,
                detour.get(row, column).unwrap().re,
            );
        }
    }
}

#[test]
fn supported_metadata_and_caches_survive_transformation_and_serialization() {
    let source = Network::from_str("metadata.s1p", "! measured load\n[Version] 2.1\n# GHz S MA R 50\n[Number of Ports] 1\n[Reference] 50\n[Vendor Note] retained warning\n[Network Data]\n! calibrated\n1 0.2 0\n2 0.4 90\n[End]\n").unwrap();
    assert!(!source.warnings.is_empty());
    let output = source.renormalize(75.0).unwrap();
    assert_eq!(source.warnings, output.warnings);
    assert_eq!(source.comments, output.comments);
    assert_eq!(
        source.comments_after_option_line,
        output.comments_after_option_line
    );
    assert_eq!(source.name, output.name);
    assert_eq!(source.format, output.format);
    assert_eq!(source.parameter, output.parameter);
    assert_eq!(source.resistance_string, output.resistance_string);
    assert_eq!(source.frequency_unit, output.frequency_unit);
    assert_eq!(source.frequencies(), output.frequencies());
    let serialized = output.to_touchstone_string().unwrap();
    let reparsed = Network::from_str("saved.s1p", &serialized).unwrap();
    assert_eq!(reparsed.z0, 75.0);
    assert_eq!(
        reparsed.reference_impedance(),
        ReferenceImpedance::Common(75.0)
    );
    assert_eq!(reparsed.format, "MA");
    assert_eq!(reparsed.frequency_unit, "GHz");
    assert_eq!(reparsed.comments, output.comments);
    assert_eq!(
        reparsed.comments_after_option_line,
        output.comments_after_option_line
    );
    assert_eq!(reparsed.frequencies(), output.frequencies());
    // Unknown keyword text is not serialized. Original warnings survive in memory,
    // not as a claim that a newly parsed document observed the same issues.
    assert!(reparsed.warnings.is_empty());
    for point in 0..2 {
        let ri = output.try_s_ri_at(point, 1, 1).unwrap();
        let saved = reparsed.try_s_ri_at(point, 1, 1).unwrap();
        near(ri.re, saved.re);
        near(ri.im, saved.im);
        let magnitude = ri.re.hypot(ri.im);
        near(output.s_ma(1, 1)[point].s_ma.0, magnitude);
        if magnitude > 1e-14 {
            near(output.s_db(1, 1)[point].s_db.0, 20.0 * magnitude.log10());
        }
    }
}

#[test]
fn identity_is_validated_and_unchanged_even_at_extreme_common_reference() {
    let mut source = one(1.0, f64::MAX);
    source.warnings.push(TouchstoneWarning::MissingOptionLine {
        source_name: "original".into(),
    });
    let output = source.renormalize(f64::MAX).unwrap();
    assert_eq!(
        source.to_touchstone_string().unwrap(),
        output.to_touchstone_string().unwrap()
    );
    assert_eq!(source.warnings, output.warnings);
    source.rank = 2;
    assert!(matches!(
        source.renormalize(f64::MAX),
        Err(TouchstoneError::InvalidMatrixRank { .. })
    ));
    source.rank = -1;
    assert!(matches!(
        source.renormalize(f64::MAX),
        Err(TouchstoneError::InvalidNetworkRank { .. })
    ));
}

#[test]
fn invalid_source_metadata_is_rejected_before_identity_or_reference_change() {
    for new_z0 in [50.0, 75.0] {
        for format in ["bogus", "ri", " RI", ""] {
            let mut source = one(0.2, 50.0);
            source.format = format.into();
            assert!(matches!(
                source.renormalize(new_z0),
                Err(TouchstoneError::UnsupportedFormat { format: rejected }) if rejected == format
            ));
            assert_eq!(source.format, format);
            assert_eq!(source.z0, 50.0);
            assert_eq!(source.try_s_ri_at(0, 1, 1).unwrap().re, 0.2);
        }
        for unit in ["bogus", "hz", "Hz ", "THz", ""] {
            let mut source = one(0.2, 50.0);
            source.frequency_unit = unit.into();
            assert!(matches!(
                source.renormalize(new_z0),
                Err(TouchstoneError::UnsupportedFrequencyUnit { unit: rejected }) if rejected == unit
            ));
            assert_eq!(source.frequency_unit, unit);
            assert_eq!(source.z0, 50.0);
            assert_eq!(source.frequencies(), &[1.0]);
        }
    }
}

#[test]
fn supported_source_formats_and_units_roundtrip_after_renormalization() {
    for format in ["RI", "MA", "DB"] {
        for unit in ["Hz", "kHz", "MHz", "GHz"] {
            let source =
                Network::from_str("load.s1p", &format!("# {unit} S {format} R 50\n1 0.2 0\n"))
                    .unwrap();
            for new_z0 in [50.0, 75.0] {
                let output = source.renormalize(new_z0).unwrap();
                let reparsed =
                    Network::from_str("saved.s1p", &output.to_touchstone_string().unwrap())
                        .unwrap();
                assert_eq!(reparsed.format, format);
                assert_eq!(reparsed.frequency_unit, unit);
                assert_eq!(reparsed.frequencies(), source.frequencies());
                let actual = reparsed.try_s_ri_at(0, 1, 1).unwrap();
                let expected = output.try_s_ri_at(0, 1, 1).unwrap();
                near(actual.re, expected.re);
                near(actual.im, expected.im);
            }
        }
    }
}

#[test]
fn mixed_mode_warning_guard_normalizes_public_keyword_edits() {
    let original = Network::from_str(
        "mixed.s1p",
        "# Hz S RI R 50\n[Mixed-Mode Order] D1\n1 0.2 0\n",
    )
    .unwrap();
    for keyword in [
        "mixed mode order",
        "Mixed-Mode Order",
        "  MIXED - Mode\t ORDER  ",
        "mixed--mode\norder",
    ] {
        let mut source = original.clone();
        for warning in &mut source.warnings {
            if let TouchstoneWarning::UnknownKeywordIgnored {
                keyword: stored, ..
            } = warning
            {
                *stored = keyword.into();
            }
        }
        for new_z0 in [50.0, 75.0] {
            assert!(matches!(
                source.renormalize(new_z0),
                Err(TouchstoneError::UnsupportedRenormalizationKeyword { keyword: rejected }) if rejected == keyword
            ));
            assert_eq!(source.z0, 50.0);
            assert_eq!(source.try_s_ri_at(0, 1, 1).unwrap().re, 0.2);
        }
    }
}

#[test]
fn range_safe_references_and_explicit_ratio_rejection() {
    // Original new+old would overflow, but the ratio is modest and representable.
    near(
        one(0.0, 1e308)
            .renormalize(1.5e308)
            .unwrap()
            .try_s_ri_at(0, 1, 1)
            .unwrap()
            .re,
        -0.2,
    );
    near(
        one(0.0, 1e-308)
            .renormalize(1.5e-308)
            .unwrap()
            .try_s_ri_at(0, 1, 1)
            .unwrap()
            .re,
        -0.2,
    );
    for (old, new) in [(1e-300, 1e300), (1e300, 1e-300), (1.0, 1e17), (1e17, 1.0)] {
        for s in [0.0, 1.0, -1.0] {
            let source = one(s, old);
            assert!(
                matches!(source.renormalize(new), Err(TouchstoneError::UnrepresentableReferenceRatio { old_z0, new_z0 }) if old_z0 == old && new_z0 == new)
            );
            assert_eq!(source.z0, old);
            assert_eq!(source.try_s_ri_at(0, 1, 1).unwrap().re, s);
        }
    }
    // g remains representable here, but the open's tiny denominator fails the
    // existing absolute pivot policy: distinct from lost-ratio rejection.
    assert!(matches!(
        one(1.0, 1.0).renormalize(1e13),
        Err(TouchstoneError::SingularMatrix { .. })
    ));
}

#[test]
fn contextual_singularities_invalid_inputs_and_semantic_guards() {
    // Exactly representable g=0.5, denominator 1-gS=0.
    assert!(matches!(
        one(2.0, 50.0).renormalize(150.0),
        Err(TouchstoneError::SingularMatrix {
            pivot_magnitude: 0.0,
            ..
        })
    ));
    let source = one(5.0, 50.0);
    let error = source.renormalize(75.0).unwrap_err();
    assert!(
        matches!(&error, TouchstoneError::SingularMatrix { operation, .. } if operation.contains("point 0") && operation.contains("50 -> 75"))
    );
    assert_eq!(source.try_s_ri_at(0, 1, 1).unwrap().re, 5.0);
    for invalid in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(matches!(
            source.renormalize(invalid),
            Err(TouchstoneError::InvalidReferenceImpedance { .. })
        ));
        let mut bad = source.clone();
        bad.z0 = invalid;
        assert!(matches!(
            bad.renormalize(75.0),
            Err(TouchstoneError::InvalidReferenceImpedance { .. })
        ));
    }
    let mut bad = source.clone();
    bad.parameter = "Z".into();
    assert!(matches!(
        bad.renormalize(50.0),
        Err(TouchstoneError::UnsupportedNetworkParameter { .. })
    ));
    let mut bad = source.clone();
    bad.reference_impedance = ReferenceImpedance::PerPort(vec![50.0]);
    assert!(matches!(
        bad.renormalize(50.0),
        Err(TouchstoneError::UnsupportedReferenceImpedance { .. })
    ));
    let per_port = Network::from_str(
        "equal.s2p",
        "# Hz S RI R 50\n[Reference] 50 50\n1 0 0 0 0 0 0 0 0\n",
    )
    .unwrap();
    assert!(matches!(
        per_port.renormalize(75.0),
        Err(TouchstoneError::UnsupportedReferenceImpedance { .. })
    ));
    let mixed = Network::from_str(
        "mixed.s2p",
        "# Hz S RI R 50\n[Mixed-Mode Order] D1,2 C1,2\n1 0 0 0 0 0 0 0 0\n",
    )
    .unwrap();
    assert!(matches!(
        mixed.renormalize(50.0),
        Err(TouchstoneError::UnsupportedRenormalizationKeyword { .. })
    ));
    let unsorted = Network::from_str("unsorted.s1p", "# Hz S RI R 50\n2 0 0\n1 0 0\n").unwrap();
    assert!(matches!(
        unsorted.renormalize(75.0),
        Err(TouchstoneError::UnsortedFrequencies { .. })
    ));
    let nonfinite = one(f64::NAN, 50.0);
    assert!(matches!(
        nonfinite.renormalize(50.0),
        Err(TouchstoneError::InvalidParameterMatrixValue { .. })
    ));
}
