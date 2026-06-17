use touchstone::{Network, TouchstoneError};

const TWO_PORT_RI: &str = "# GHz S RI R 50\n1.0 0.1 0.0 4.0 0.0 0.01 0.0 0.2 0.0\n";

#[test]
fn try_new_reads_existing_touchstone_file() {
    let network = Network::try_new("files/ntwk1.s2p").unwrap();

    assert_eq!(network.name, "files/ntwk1.s2p");
    assert_eq!(network.rank, 2);
    assert!(!network.f.is_empty());
}

#[test]
fn from_str_parses_uploaded_touchstone_data_without_file() {
    let network = Network::from_str("uploaded.s2p", TWO_PORT_RI).unwrap();

    assert_eq!(network.name, "uploaded.s2p");
    assert_eq!(network.rank, 2);
    assert_eq!(network.frequency_unit, "GHz");
    assert_eq!(network.z0, 50.0);
    assert_eq!(network.f, vec![1.0e9]);
    assert_eq!(network.s_ri(2, 1)[0].s_ri.0, 4.0);
    assert_eq!(network.s_ri(1, 2)[0].s_ri.0, 0.01);
}

#[test]
fn from_bytes_parses_uploaded_touchstone_data_without_file() {
    let network = Network::from_bytes("uploaded.s2p", TWO_PORT_RI.as_bytes()).unwrap();

    assert_eq!(network.name, "uploaded.s2p");
    assert_eq!(network.rank, 2);
    assert_eq!(network.s_ri(2, 1)[0].s_ri.0, 4.0);
    assert_eq!(network.s_ri(1, 2)[0].s_ri.0, 0.01);
}

#[test]
fn from_bytes_returns_utf8_errors() {
    let error = Network::from_bytes("uploaded.s1p", &[0xff]).unwrap_err();

    assert!(matches!(error, TouchstoneError::InvalidUtf8(_)));
}

#[test]
fn from_str_returns_unsupported_file_type_errors() {
    let error = Network::from_str("uploaded.txt", TWO_PORT_RI).unwrap_err();

    assert!(matches!(
        error,
        TouchstoneError::UnsupportedFileType { file_type } if file_type == "txt"
    ));
}

#[test]
fn from_str_returns_malformed_data_errors() {
    let error = Network::from_str("uploaded.s1p", "# GHz S RI R 50\n1.0 0.1\n").unwrap_err();

    assert!(matches!(
        error,
        TouchstoneError::InvalidDataLineParts {
            expected: 3,
            actual: 2
        }
    ));
}
