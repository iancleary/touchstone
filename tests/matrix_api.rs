use touchstone::{Complex, Network, TouchstoneError};

const ONE_PORT_RI: &str = "# Hz S RI R 50\n1000000000 0.5 -0.25\n2000000000 0.6 -0.35\n";

const TWO_PORT_ASYMMETRIC_RI: &str =
    "# Hz S RI R 50\n1000000000 0.11 0.01 0.21 0.02 0.12 0.03 0.22 0.04\n";

const THREE_PORT_RI: &str = "\
# Hz S RI R 50
1000000000 11 0.1 12 0.2 13 0.3 21 0.4 22 0.5 23 0.6 31 0.7 32 0.8 33 0.9
";

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
