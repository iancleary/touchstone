use touchstone::{Complex, Network, NetworkBuilder, SMatrix};

fn c(re: f64, im: f64) -> Complex {
    Complex { re, im }
}

fn matrix(data: Vec<Vec<Complex>>) -> SMatrix {
    SMatrix {
        rank: data.len(),
        data,
    }
}

fn round_trip_from_string(network: &Network) -> Network {
    let serialized = network.to_touchstone_string().unwrap();
    Network::from_str(network.name.as_str(), &serialized).unwrap()
}

fn assert_round_trip_points(network: &Network) {
    let reparsed = round_trip_from_string(network);

    assert_eq!(
        reparsed.reference_impedance(),
        network.reference_impedance()
    );
    assert_eq!(reparsed.points().unwrap(), network.points().unwrap());
}

#[test]
fn matched_one_port_s_parameter_fixture_round_trips() {
    let matched = matrix(vec![vec![c(0.0, 0.0)]]);
    let network = NetworkBuilder::new("generated_matched_one_port.s1p", 1)
        .comment("generated fixture: matched one-port")
        .point(1.0e6, matched.clone())
        .point(2.0e6, matched)
        .build()
        .unwrap();

    assert_eq!(network.rank, 1);
    assert_eq!(network.try_s_ri_at(0, 1, 1).unwrap(), c(0.0, 0.0));
    assert_eq!(
        network.point_at(1).unwrap().s.get(1, 1).unwrap(),
        c(0.0, 0.0)
    );

    let reparsed = round_trip_from_string(&network);
    assert_eq!(reparsed.f, vec![1.0e6, 2.0e6]);
    assert_eq!(
        reparsed.s_matrix_at(0).unwrap().get(1, 1).unwrap(),
        c(0.0, 0.0)
    );
    assert_eq!(reparsed.points().unwrap(), network.points().unwrap());
}

#[test]
fn open_and_short_one_port_reflection_fixtures_round_trip() {
    let fixtures = [
        ("generated_open_reflection.s1p", c(1.0, 0.0)),
        ("generated_short_reflection.s1p", c(-1.0, 0.0)),
    ];

    for (name, reflection) in fixtures {
        let network = NetworkBuilder::new(name, 1)
            .comment("generated fixture: one-port reflection")
            .point(1.0e9, matrix(vec![vec![reflection]]))
            .build()
            .unwrap();
        let serialized = network.to_touchstone_string().unwrap();
        let reparsed = Network::from_bytes(name, serialized.as_bytes()).unwrap();

        assert_eq!(network.try_s_ri_at(0, 1, 1).unwrap(), reflection);
        assert_eq!(reparsed.try_s_ri_at(0, 1, 1).unwrap(), reflection);
        assert_eq!(reparsed.points().unwrap(), network.points().unwrap());
    }
}

#[test]
fn asymmetric_two_port_fixture_preserves_distinct_s21_and_s12() {
    let network = NetworkBuilder::new("generated_asymmetric_two_port.s2p", 2)
        .point(
            1.0e9,
            matrix(vec![
                vec![c(0.11, 0.01), c(0.12, 0.03)],
                vec![c(0.21, 0.02), c(0.22, 0.04)],
            ]),
        )
        .build()
        .unwrap();

    let s = network.s_matrix_at(0).unwrap();
    assert_eq!(s.get(1, 1).unwrap(), c(0.11, 0.01));
    assert_eq!(s.get(2, 1).unwrap(), c(0.21, 0.02));
    assert_eq!(s.get(1, 2).unwrap(), c(0.12, 0.03));
    assert_eq!(s.get(2, 2).unwrap(), c(0.22, 0.04));
    assert_ne!(s.get(2, 1).unwrap(), s.get(1, 2).unwrap());

    let reparsed = round_trip_from_string(&network);
    assert_eq!(reparsed.try_s_ri_at(0, 2, 1).unwrap(), c(0.21, 0.02));
    assert_eq!(reparsed.try_s_ri_at(0, 1, 2).unwrap(), c(0.12, 0.03));
    assert_ne!(
        reparsed.try_s_ri_at(0, 2, 1).unwrap(),
        reparsed.try_s_ri_at(0, 1, 2).unwrap()
    );
}

#[test]
fn generated_two_port_serializes_21_12_order() {
    let network = NetworkBuilder::new("generated_two_port_order.s2p", 2)
        .frequency_unit("GHz")
        .point(
            1.0e9,
            matrix(vec![
                vec![c(0.11, 0.01), c(0.12, 0.03)],
                vec![c(0.21, 0.02), c(0.22, 0.04)],
            ]),
        )
        .build()
        .unwrap();

    let serialized = network.to_touchstone_string().unwrap();
    assert!(serialized.contains("[Two-Port Data Order] 21_12\n"));

    let data_line = serialized
        .lines()
        .find(|line| line.starts_with("1 "))
        .expect("generated two-port data line");
    let values = data_line
        .split_whitespace()
        .map(|part| part.parse::<f64>().unwrap())
        .collect::<Vec<_>>();

    assert_eq!(
        values,
        vec![1.0, 0.11, 0.01, 0.21, 0.02, 0.12, 0.03, 0.22, 0.04]
    );
    assert_round_trip_points(&network);
}

#[test]
fn small_three_port_fixture_round_trips_full_matrix_data() {
    let first = matrix(vec![
        vec![c(0.11, 0.01), c(0.12, 0.02), c(0.13, 0.03)],
        vec![c(0.21, 0.04), c(0.22, 0.05), c(0.23, 0.06)],
        vec![c(0.31, 0.07), c(0.32, 0.08), c(0.33, 0.09)],
    ]);
    let second = matrix(vec![
        vec![c(1.11, -0.01), c(1.12, -0.02), c(1.13, -0.03)],
        vec![c(1.21, -0.04), c(1.22, -0.05), c(1.23, -0.06)],
        vec![c(1.31, -0.07), c(1.32, -0.08), c(1.33, -0.09)],
    ]);
    let network = NetworkBuilder::new("generated_small_three_port.s3p", 3)
        .point(1.0e9, first.clone())
        .point(2.0e9, second.clone())
        .build()
        .unwrap();

    let serialized = network.to_touchstone_string().unwrap();
    let lines = serialized.lines().collect::<Vec<_>>();
    let data_index = lines
        .iter()
        .position(|line| *line == "[Network Data]")
        .unwrap();

    assert_eq!(
        lines[data_index + 1],
        "1000000000 0.11 0.01 0.12 0.02 0.13 0.03"
    );
    assert_eq!(lines[data_index + 2], " 0.21 0.04 0.22 0.05 0.23 0.06");
    assert_eq!(lines[data_index + 3], " 0.31 0.07 0.32 0.08 0.33 0.09");
    assert_eq!(
        network.s_matrix_at(0).unwrap(),
        SMatrix {
            rank: 3,
            data: first.data.clone()
        }
    );

    let reparsed = Network::from_str("generated_small_three_port.s3p", &serialized).unwrap();
    assert_eq!(reparsed.point_at(0).unwrap().s, first);
    assert_eq!(reparsed.point_at(1).unwrap().s, second);
    assert_eq!(
        reparsed.s_matrix_at(1).unwrap().get(3, 2).unwrap(),
        c(1.32, -0.08)
    );
    assert_eq!(reparsed.points().unwrap(), network.points().unwrap());
}

#[test]
fn generated_data_preserves_comments_and_touchstone_2_1_metadata() {
    let network = NetworkBuilder::new("generated_metadata_fixture.s1p", 1)
        .frequency_unit("MHz")
        .z0(75.0)
        .comment("generated fixture: public metadata")
        .comment("! generated fixture: explicit comment marker")
        .network_data_comment("generated fixture: network data")
        .point(1.0e6, matrix(vec![vec![c(0.5, -0.25)]]))
        .build()
        .unwrap();

    let serialized = network.to_touchstone_string().unwrap();
    let expected = "\
! generated fixture: public metadata
! generated fixture: explicit comment marker
[Version] 2.1
# MHz S RI R 75
[Number of Ports] 1
[Number of Frequencies] 1
[Matrix Format] Full
[Network Data]
! generated fixture: network data
1 0.5 -0.25
[End]
";

    assert_eq!(serialized, expected);

    let reparsed = Network::from_str("generated_metadata_fixture.s1p", &serialized).unwrap();
    assert_eq!(
        reparsed.comments,
        vec![
            "! generated fixture: public metadata".to_string(),
            "! generated fixture: explicit comment marker".to_string(),
        ]
    );
    assert_eq!(
        reparsed.comments_after_option_line,
        vec!["! generated fixture: network data".to_string()]
    );
    assert_eq!(reparsed.frequency_unit, "MHz");
    assert_eq!(reparsed.z0, 75.0);
    assert_eq!(reparsed.try_s_ri_at(0, 1, 1).unwrap(), c(0.5, -0.25));
}
