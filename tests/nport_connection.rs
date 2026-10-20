use touchstone::{
    Complex, Network, NetworkBuilder, NetworkSide, ReferenceImpedance, SMatrix, SourcePort,
    TouchstoneError,
};

fn c(re: f64, im: f64) -> Complex {
    Complex { re, im }
}
fn generated(name: &str, data: Vec<Vec<Complex>>) -> Network {
    NetworkBuilder::new(name, data.len())
        .point(1e9, SMatrix::try_new(data).unwrap())
        .build()
        .unwrap()
}
fn load(gamma: Complex) -> Network {
    generated("load", vec![vec![gamma]])
}
fn splitter() -> Network {
    generated(
        "splitter",
        (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| c(if i == j { -1.0 / 3.0 } else { 2.0 / 3.0 }, 0.0))
                    .collect()
            })
            .collect(),
    )
}
fn close(a: Complex, b: Complex) {
    assert!(
        (a.re - b.re).hypot(a.im - b.im) <= 2e-14 * b.re.hypot(b.im).max(1e-290),
        "{a:?} != {b:?}"
    );
}
fn absolute_close(a: Complex, b: Complex) {
    assert!((a.re - b.re).hypot(a.im - b.im) < 2e-15, "{a:?} != {b:?}");
}

#[test]
fn analytic_splitter_matched_open_and_complex_load_roundtrip() {
    for gamma in [c(0.0, 0.0), c(1.0, 0.0), c(-1.0, 0.0), c(0.2, 0.4)] {
        let result = splitter().connect(&load(gamma), 2, 1).unwrap();
        assert_eq!(
            result.surviving_ports,
            vec![
                SourcePort {
                    side: NetworkSide::Left,
                    port: 1
                },
                SourcePort {
                    side: NetworkSide::Left,
                    port: 3
                }
            ]
        );
        // Independent load formula: S'jk = Sjk + Sj2 Gamma S2k / (1-S22 Gamma).
        let correction = gamma * (4.0 / 9.0) / (c(1.0, 0.0) + gamma / 3.0);
        let matrix = result.network.s_matrix_at(0).unwrap();
        absolute_close(matrix.get(1, 1).unwrap(), c(-1.0 / 3.0, 0.0) + correction);
        absolute_close(matrix.get(1, 2).unwrap(), c(2.0 / 3.0, 0.0) + correction);
        close(matrix.get(2, 1).unwrap(), matrix.get(1, 2).unwrap());
        let serialized = result.network.to_touchstone_string().unwrap();
        let reread = Network::from_str("connected.s2p", &serialized).unwrap();
        close(
            reread.s_matrix_at(0).unwrap().get(1, 2).unwrap(),
            matrix.get(1, 2).unwrap(),
        );
    }
}

#[test]
fn block_solve_matches_scalar_nonreciprocal_cascade_including_deep_loss() {
    for transmission in [0.0, 0.7, 1e-100, 1e-150] {
        let a = generated(
            "a",
            vec![
                vec![c(0.1, 0.02), c(0.3, -0.2)],
                vec![c(transmission, 0.0), c(-0.2, 0.1)],
            ],
        );
        let b = generated(
            "b",
            vec![
                vec![c(0.4, -0.1), c(transmission, 0.0)],
                vec![c(0.8, 0.2), c(0.2, 0.0)],
            ],
        );
        let scalar = a.try_cascade(&b).unwrap().s_matrix_at(0).unwrap();
        let block = a.connect(&b, 2, 1).unwrap().network.s_matrix_at(0).unwrap();
        for i in 1..=2 {
            for j in 1..=2 {
                close(block.get(i, j).unwrap(), scalar.get(i, j).unwrap());
            }
        }
    }
}

#[test]
fn every_joined_pair_of_dense_nonreciprocal_three_and_four_port_networks() {
    let make = |rank| {
        generated(
            "dense",
            (0..rank)
                .map(|i| {
                    (0..rank)
                        .map(|j| {
                            if i == j {
                                c(0.0, 0.0)
                            } else {
                                c((i + 1) as f64 / 10.0, (j + 1) as f64 / 20.0)
                            }
                        })
                        .collect()
                })
                .collect(),
        )
    };
    let a = make(3);
    let b = make(4);
    let am = a.s_matrix_at(0).unwrap();
    let bm = b.s_matrix_at(0).unwrap();
    for lp in 1..=3 {
        for rp in 1..=4 {
            let result = a.connect(&b, lp, rp).unwrap();
            let matrix = result.network.s_matrix_at(0).unwrap();
            for (i, to) in result.surviving_ports.iter().enumerate() {
                for (j, from) in result.surviving_ports.iter().enumerate() {
                    // Joined self reflections vanish: no feedback. Same-side responses
                    // are unchanged; cross-side response is a single path product.
                    let expected = match (to.side, from.side) {
                        (NetworkSide::Left, NetworkSide::Left) => {
                            am.get(to.port, from.port).unwrap()
                        }
                        (NetworkSide::Right, NetworkSide::Right) => {
                            bm.get(to.port, from.port).unwrap()
                        }
                        (NetworkSide::Left, NetworkSide::Right) => {
                            am.get(to.port, lp).unwrap() * bm.get(rp, from.port).unwrap()
                        }
                        (NetworkSide::Right, NetworkSide::Left) => {
                            bm.get(to.port, rp).unwrap() * am.get(lp, from.port).unwrap()
                        }
                    };
                    absolute_close(matrix.get(i + 1, j + 1).unwrap(), expected);
                }
            }
        }
    }
}

#[test]
fn hz_grids_metadata_warnings_and_survivor_order() {
    let left = Network::from_str("left.s3p", "! left\n[Version] 2.0\n# GHz S RI R 50\n[Reference] 75 50 25\n[Unknown] x\n[Network Data]\n1 0.1 0 0.2 0 0.3 0 0.4 0 0.1 0 0.2 0 0.3 0 0.4 0 0.1 0\n[End]\n").unwrap();
    let right = Network::from_str(
        "right.s2p",
        "! right\n# MHz S RI R 50\n# Hz S RI R 50\n1000 0.1 0 0.2 0 0.3 0 0.4 0\n",
    )
    .unwrap();
    let result = left.connect(&right, 2, 1).unwrap();
    assert_eq!(
        result.surviving_ports,
        vec![
            SourcePort {
                side: NetworkSide::Left,
                port: 1
            },
            SourcePort {
                side: NetworkSide::Left,
                port: 3
            },
            SourcePort {
                side: NetworkSide::Right,
                port: 2
            }
        ]
    );
    assert_eq!(
        result.network.reference_impedance,
        ReferenceImpedance::PerPort(vec![75.0, 25.0, 50.0])
    );
    assert_eq!(result.network.z0, 75.0);
    assert_eq!(result.network.frequency_unit, "GHz");
    assert_eq!(
        result.network.warnings,
        [left.warnings, right.warnings].concat()
    );
    assert!(result.network.comments.iter().any(|s| s.contains("left")));
    assert!(result.network.comments.iter().any(|s| s.contains("right")));
    let reread = Network::from_str(
        "result.s3p",
        &result.network.to_touchstone_string().unwrap(),
    )
    .unwrap();
    assert_eq!(
        reread.reference_impedance,
        result.network.reference_impedance
    );
    assert_eq!(reread.frequencies(), &[1e9]);
}

#[test]
fn boundary_rejections_and_singular_context() {
    let s = splitter();
    let l = load(c(0.0, 0.0));
    for (lp, rp) in [(0, 1), (4, 1), (1, 0), (1, 2), (usize::MAX, 1)] {
        assert!(s.connect(&l, lp, rp).is_err());
    }
    assert!(l.connect(&l, 1, 1).is_err());
    for mutate in 0..8 {
        let mut bad = s.clone();
        match mutate {
            0 => bad.rank = 2,
            1 => bad.parameter = "Y".into(),
            2 => bad.reference_impedance = ReferenceImpedance::PerPort(vec![]),
            3 => bad.reference_impedance = ReferenceImpedance::PerPort(vec![50.0, f64::NAN, 50.0]),
            4 => bad.z0 = 75.0,
            5 => bad.frequency_unit = "invalid".into(),
            6 => bad.format = "invalid".into(),
            _ => bad.rank = i32::MAX,
        }
        assert!(bad.connect(&l, 2, 1).is_err());
    }
    let mut wrong_ref = l.clone();
    wrong_ref.z0 = 75.0;
    wrong_ref.reference_impedance = ReferenceImpedance::Common(75.0);
    assert!(s.connect(&wrong_ref, 2, 1).is_err());
    let wrong_grid = Network::from_str("wrong.s1p", "# Hz S RI R 50\n1 0 0\n").unwrap();
    assert!(s.connect(&wrong_grid, 2, 1).is_err());
    let mixed = Network::from_str(
        "mixed.s2p",
        "# GHz S RI R 50\n[Mixed-Mode Order] D1,2 C1,2\n1 0 0 1 0 1 0 0 0\n",
    )
    .unwrap();
    assert!(mixed.connect(&l, 2, 1).is_err());
    for reflection in [1.0, 1.0 - 5e-13] {
        let resonant = generated(
            "resonant",
            vec![
                vec![c(0.0, 0.0), c(1.0, 0.0)],
                vec![c(1.0, 0.0), c(reflection, 0.0)],
            ],
        );
        let error = resonant.connect(&load(c(1.0, 0.0)), 2, 1).unwrap_err();
        assert!(matches!(
            error.root_cause(),
            TouchstoneError::SingularMatrix {
                tolerance: 1e-12,
                ..
            }
        ));
        match error {
            TouchstoneError::Connection {
                point_index,
                frequency_hz,
                ..
            } => {
                assert_eq!(point_index, 0);
                assert_eq!(frequency_hz, 1e9);
            }
            error => panic!("unexpected {error}"),
        }
    }
}

#[test]
fn multiple_points_formats_and_terahertz_display_preserve_hz_semantics() {
    let mut generated_thz = splitter();
    generated_thz.frequency_unit = "THz".into();
    assert_eq!(
        generated_thz
            .connect(&load(c(0.0, 0.0)), 2, 1)
            .unwrap()
            .network
            .frequency_unit,
        "THz"
    );
    for format in ["RI", "MA", "DB"] {
        let mut source = Network::from_str(
            "through.s2p",
            "# GHz S RI R 50\n! after left\n1 0 0 1 0 1 0 0 0\n2 0 0 1 0 1 0 0 0\n",
        )
        .unwrap();
        source.format = format.into();
        let load = Network::from_str(
            "load.s1p",
            "# MHz S RI R 50\n! after right\n1000 0.2 0\n2000 0.4 0\n",
        )
        .unwrap();
        let output = source.connect(&load, 2, 1).unwrap().network;
        assert_eq!(output.format, format);
        assert_eq!(output.frequency_unit, "GHz");
        assert_eq!(
            output.comments_after_option_line,
            [
                source.comments_after_option_line,
                load.comments_after_option_line
            ]
            .concat()
        );
        let reread =
            Network::from_str("output.s1p", &output.to_touchstone_string().unwrap()).unwrap();
        assert_eq!(reread.frequencies(), &[1e9, 2e9]);
        absolute_close(
            reread.s_matrix_at(0).unwrap().get(1, 1).unwrap(),
            c(0.2, 0.0),
        );
        absolute_close(
            reread.s_matrix_at(1).unwrap().get(1, 1).unwrap(),
            c(0.4, 0.0),
        );
    }
    let source = Network::from_str(
        "source.s2p",
        "# GHz S RI R 50\n1 0 0 1 0 1 0 0 0\n2 0 0 1 0 1 0 1 0\n",
    )
    .unwrap();
    let load = Network::from_str("open.s1p", "# GHz S RI R 50\n1 1 0\n2 1 0\n").unwrap();
    assert!(matches!(
        source.connect(&load, 2, 1),
        Err(TouchstoneError::Connection {
            point_index: 1,
            frequency_hz: 2e9,
            ..
        })
    ));
}

#[test]
fn malformed_parsed_grids_and_nonfinite_source_are_rejected() {
    for data in [
        "",
        "1 0 0\n1 0 0\n",
        "2 0 0\n1 0 0\n",
        "NaN 0 0\n",
        "1 NaN 0\n",
    ] {
        let bad = Network::from_str("bad.s1p", &format!("# GHz S RI R 50\n{data}")).unwrap();
        assert!(splitter().connect(&bad, 2, 1).is_err());
    }
}

#[test]
fn finite_extremes_and_rank_limit_do_not_wrap() {
    let huge = generated(
        "huge",
        vec![
            vec![c(0.0, 0.0), c(1e308, 0.0)],
            vec![c(1e308, 0.0), c(0.0, 0.0)],
        ],
    );
    assert!(matches!(
        huge.connect(&load(c(1.0, 0.0)), 2, 1),
        Err(TouchstoneError::Connection { .. })
    ));
    let matched = huge.connect(&load(c(0.0, 0.0)), 2, 1).unwrap();
    assert_eq!(
        matched.network.s_matrix_at(0).unwrap().get(1, 1).unwrap(),
        c(0.0, 0.0)
    );
    let zero = |rank| generated("zero", vec![vec![c(0.0, 0.0); rank]; rank]);
    let a = zero(127);
    let b = zero(2);
    assert_eq!(a.connect(&b, 127, 1).unwrap().network.rank, 127);
    assert!(a.connect(&zero(3), 1, 1).is_err());
    assert!(zero(128).connect(&load(c(0.0, 0.0)), 1, 1).is_err());
    let result = load(c(0.0, 0.0)).connect(&splitter(), 1, 1).unwrap();
    assert!(result
        .surviving_ports
        .iter()
        .all(|p| p.side == NetworkSide::Right));
}
