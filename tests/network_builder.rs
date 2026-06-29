use touchstone::{Complex, Network, NetworkBuilder, SMatrix, TouchstoneError};

fn c(re: f64, im: f64) -> Complex {
    Complex { re, im }
}

fn matrix(rows: Vec<Vec<Complex>>) -> SMatrix {
    SMatrix {
        rank: rows.len(),
        data: rows,
    }
}

#[test]
fn builds_one_port_network_with_comments_and_derived_data() {
    let network = NetworkBuilder::new("generated.s1p", 1)
        .comment("generated one-port")
        .network_data_comment("measured in memory")
        .z0(75.0)
        .point(1.0e9, matrix(vec![vec![c(0.5, -0.25)]]))
        .point(2.0e9, matrix(vec![vec![c(0.6, -0.35)]]))
        .build()
        .unwrap();

    assert_eq!(network.name, "generated.s1p");
    assert_eq!(network.rank, 1);
    assert_eq!(network.frequency_unit, "Hz");
    assert_eq!(network.format, "RI");
    assert_eq!(network.z0, 75.0);
    assert_eq!(network.f, vec![1.0e9, 2.0e9]);
    assert!(network.warnings.is_empty());
    assert_eq!(network.try_s_ri_at(1, 1, 1).unwrap(), c(0.6, -0.35));
    assert!(network.s_ma(1, 1)[0].s_ma.magnitude().is_finite());
    assert!(network.s_db(1, 1)[0].s_db.decibel().is_finite());

    let serialized = network.to_touchstone_string().unwrap();
    assert!(serialized.starts_with("! generated one-port\n[Version] 2.1\n"));
    assert!(serialized.contains("[Network Data]\n! measured in memory\n"));
}

#[test]
fn builds_two_port_network_with_21_12_order_and_roundtrips_from_str() {
    let network = NetworkBuilder::new("generated.s2p", 2)
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
        .expect("serialized two-port data line");
    let values = data_line
        .split_whitespace()
        .map(|part| part.parse::<f64>().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        values,
        vec![1.0, 0.11, 0.01, 0.21, 0.02, 0.12, 0.03, 0.22, 0.04]
    );

    let reparsed = Network::from_str("generated.s2p", &serialized).unwrap();
    assert_eq!(reparsed.try_s_ri_at(0, 1, 2).unwrap(), c(0.12, 0.03));
    assert_eq!(reparsed.try_s_ri_at(0, 2, 1).unwrap(), c(0.21, 0.02));
}

#[test]
fn serializes_thz_frequency_unit() {
    let network = NetworkBuilder::new("generated.s1p", 1)
        .frequency_unit("THz")
        .point(1.0e12, matrix(vec![vec![c(0.5, 0.0)]]))
        .build()
        .unwrap();

    let serialized = network.to_touchstone_string().unwrap();

    assert!(serialized.contains("# THz S RI R 50\n"));
    assert!(serialized.contains("1 0.5 0\n"));
}

#[test]
fn builds_three_port_network_with_multiline_full_matrix_output() {
    let network = NetworkBuilder::new("generated.s3p", 3)
        .point(
            1.0e9,
            matrix(vec![
                vec![c(11.0, 0.1), c(12.0, 0.2), c(13.0, 0.3)],
                vec![c(21.0, 0.4), c(22.0, 0.5), c(23.0, 0.6)],
                vec![c(31.0, 0.7), c(32.0, 0.8), c(33.0, 0.9)],
            ]),
        )
        .build()
        .unwrap();

    let serialized = network.to_touchstone_string().unwrap();
    let lines = serialized.lines().collect::<Vec<_>>();
    let data_index = lines
        .iter()
        .position(|line| *line == "[Network Data]")
        .unwrap();

    assert_eq!(lines[data_index + 1], "1000000000 11 0.1 12 0.2 13 0.3");
    assert_eq!(lines[data_index + 2], " 21 0.4 22 0.5 23 0.6");
    assert_eq!(lines[data_index + 3], " 31 0.7 32 0.8 33 0.9");
    assert_eq!(lines[data_index + 4], "[End]");

    let reparsed = Network::from_str("generated.s3p", &serialized).unwrap();
    assert_eq!(reparsed.try_s_ri_at(0, 3, 2).unwrap(), c(32.0, 0.8));
}

#[test]
fn save_matches_in_memory_serialization() {
    let network = NetworkBuilder::new("generated.s2p", 2)
        .point(
            1.0e9,
            matrix(vec![
                vec![c(0.11, 0.01), c(0.12, 0.03)],
                vec![c(0.21, 0.02), c(0.22, 0.04)],
            ]),
        )
        .build()
        .unwrap();
    let temp_dir = std::env::temp_dir().join("touchstone_network_builder_tests");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = temp_dir.join(format!("save_matches_in_memory_{nanos}.s2p"));

    network.save(path.to_str().unwrap()).unwrap();
    let saved = std::fs::read_to_string(&path).unwrap();

    assert_eq!(saved, network.to_touchstone_string().unwrap());

    std::fs::remove_file(path).unwrap();
}

#[test]
fn rejects_invalid_network_rank() {
    let error = NetworkBuilder::new("generated.s0p", 0).build().unwrap_err();

    assert!(matches!(
        error,
        TouchstoneError::InvalidNetworkRank { rank: 0 }
    ));
}

#[test]
fn rejects_extension_rank_mismatch() {
    let error = NetworkBuilder::new("generated.s2p", 1)
        .point(1.0e9, matrix(vec![vec![c(0.5, 0.0)]]))
        .build()
        .unwrap_err();

    assert!(matches!(
        error,
        TouchstoneError::NetworkRankExtensionMismatch {
            rank: 1,
            extension_rank: 2
        }
    ));
}

#[test]
fn rejects_empty_data() {
    let error = NetworkBuilder::new("generated.s1p", 1).build().unwrap_err();

    assert!(matches!(error, TouchstoneError::EmptyNetworkData));
}

#[test]
fn rejects_matrix_rank_mismatch() {
    let error = NetworkBuilder::new("generated.s2p", 2)
        .point(1.0e9, matrix(vec![vec![c(0.5, 0.0)]]))
        .build()
        .unwrap_err();

    assert!(matches!(
        error,
        TouchstoneError::InvalidMatrixRank {
            point_index: 0,
            matrix_rank: 1,
            expected_rank: 2
        }
    ));
}

#[test]
fn rejects_matrix_shape_mismatch() {
    let error = NetworkBuilder::new("generated.s2p", 2)
        .point(
            1.0e9,
            SMatrix {
                rank: 2,
                data: vec![vec![c(0.5, 0.0)]],
            },
        )
        .build()
        .unwrap_err();

    assert!(matches!(
        error,
        TouchstoneError::InvalidMatrixShape {
            point_index: 0,
            rows: 1,
            row_index: None,
            columns: 0,
            expected_rank: 2
        }
    ));
}

#[test]
fn rejects_non_finite_frequency() {
    let error = NetworkBuilder::new("generated.s1p", 1)
        .point(f64::NAN, matrix(vec![vec![c(0.5, 0.0)]]))
        .build()
        .unwrap_err();

    assert!(matches!(
        error,
        TouchstoneError::InvalidFrequency { point_index: 0, frequency } if frequency.is_nan()
    ));
}

#[test]
fn rejects_non_finite_s_parameter_value() {
    let error = NetworkBuilder::new("generated.s1p", 1)
        .point(1.0e9, matrix(vec![vec![c(f64::INFINITY, 0.0)]]))
        .build()
        .unwrap_err();

    assert!(matches!(
        error,
        TouchstoneError::InvalidSParameterValue {
            point_index: 0,
            to_port: 1,
            from_port: 1,
            re,
            im: 0.0,
        } if re.is_infinite()
    ));
}

#[test]
fn rejects_non_positive_z0() {
    let error = NetworkBuilder::new("generated.s1p", 1)
        .z0(0.0)
        .point(1.0e9, matrix(vec![vec![c(0.5, 0.0)]]))
        .build()
        .unwrap_err();

    assert!(matches!(
        error,
        TouchstoneError::InvalidReferenceImpedance { z0: 0.0 }
    ));
}
