use std::fs;

use touchstone::{Network, ReferenceImpedance, TouchstoneError, TouchstoneWarning};

const TWO_PORT_RI: &str = "# GHz S RI R 50\n1.0 0.1 0.0 4.0 0.0 0.01 0.0 0.2 0.0\n";
const THREE_PORT_RI_DATA: &str =
    "1.0 0.11 0.0 0.12 0.0 0.13 0.0 0.21 0.0 0.22 0.0 0.23 0.0 0.31 0.0 0.32 0.0 0.33 0.0\n";

#[test]
fn new_reads_existing_touchstone_file() {
    let network = Network::new("files/ntwk1.s2p").unwrap();

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
fn from_str_reports_common_reference_impedance_from_option_line() {
    let network = Network::from_str("uploaded.s1p", "# GHz S RI R 75\n1.0 0.5 0.0\n").unwrap();

    assert_eq!(network.z0, 75.0);
    assert_eq!(
        network.reference_impedance(),
        ReferenceImpedance::Common(75.0)
    );
}

#[test]
fn from_str_parses_common_reference_keyword_over_option_line() {
    let network = Network::from_str(
        "uploaded.s2p",
        "[Version] 2.1\n# GHz S RI R 50\n[Number of Ports] 2\n[Reference] 75\n[Network Data]\n1.0 0.1 0.0 4.0 0.0 0.01 0.0 0.2 0.0\n[End]\n",
    )
    .unwrap();

    assert_eq!(network.z0, 75.0);
    assert_eq!(
        network.reference_impedance(),
        ReferenceImpedance::Common(75.0)
    );
}

#[test]
fn from_str_parses_per_port_reference_keyword() {
    let contents = format!(
        "[Version] 2.1\n# GHz S RI R 50\n[Number of Ports] 3\n[Reference]\n45 50 55\n[Network Data]\n{THREE_PORT_RI_DATA}[End]\n"
    );

    let network = Network::from_str("uploaded.s3p", &contents).unwrap();

    assert_eq!(network.z0, 45.0);
    assert_eq!(
        network.reference_impedance(),
        ReferenceImpedance::PerPort(vec![45.0, 50.0, 55.0])
    );
}

#[test]
fn to_touchstone_string_round_trips_per_port_reference_keyword() {
    let contents = format!(
        "[Version] 2.1\n# GHz S RI R 50\n[Number of Ports] 3\n[Reference] 45 50 55\n[Network Data]\n{THREE_PORT_RI_DATA}[End]\n"
    );
    let network = Network::from_str("uploaded.s3p", &contents).unwrap();

    let serialized = network.to_touchstone_string().unwrap();
    assert!(serialized.contains("[Reference] 45 50 55\n"));

    let reparsed = Network::from_str("uploaded.s3p", &serialized).unwrap();
    assert_eq!(reparsed.z0, 45.0);
    assert_eq!(
        reparsed.reference_impedance(),
        ReferenceImpedance::PerPort(vec![45.0, 50.0, 55.0])
    );
    assert_eq!(reparsed.points().unwrap(), network.points().unwrap());
}

#[test]
fn from_str_rejects_reference_keyword_count_mismatch() {
    let contents = format!(
        "[Version] 2.1\n# GHz S RI R 50\n[Number of Ports] 3\n[Reference] 45 50\n[Network Data]\n{THREE_PORT_RI_DATA}[End]\n"
    );

    let error = Network::from_str("uploaded.s3p", &contents).unwrap_err();

    assert!(matches!(
        error.root_cause(),
        TouchstoneError::InvalidReferenceImpedanceCount {
            ports: 3,
            actual: 2
        }
    ));
    assert_eq!(error.context().unwrap().line_number, Some(4));
}

#[test]
fn from_str_reports_missing_reference_following_line_at_reference_keyword() {
    let contents = format!(
        "[Version] 2.1\n# GHz S RI R 50\n[Reference]\n[Network Data]\n{THREE_PORT_RI_DATA}[End]\n"
    );

    let error = Network::from_str("uploaded.s3p", &contents).unwrap_err();

    assert!(matches!(
        error.root_cause(),
        TouchstoneError::InvalidReferenceImpedanceCount {
            ports: 3,
            actual: 0
        }
    ));
    assert_eq!(error.context().unwrap().line_number, Some(3));
    assert_eq!(
        error.context().unwrap().line.as_deref(),
        Some("[Reference]")
    );
}

#[test]
fn from_str_rejects_non_positive_reference_keyword_value() {
    let error = Network::from_str(
        "uploaded.s1p",
        "[Version] 2.1\n# GHz S RI R 50\n[Reference] 0\n[Network Data]\n1.0 0.5 0.0\n[End]\n",
    )
    .unwrap_err();

    assert!(matches!(
        error.root_cause(),
        TouchstoneError::InvalidReferenceImpedance { z0: 0.0 }
    ));
    assert_eq!(error.context().unwrap().line_number, Some(3));
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
fn from_memory_matches_new_for_s1p_file() {
    assert_from_memory_matches_new("files/hfss_oneport.s1p");
}

#[test]
fn from_memory_matches_new_for_s3p_file() {
    assert_from_memory_matches_new("files/hfss_18.2.s3p");
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
        error.root_cause(),
        TouchstoneError::InvalidDataLineParts {
            expected: 3,
            actual: 2
        }
    ));

    let context = error.context().unwrap();
    assert_eq!(context.source_name, "uploaded.s1p");
    assert_eq!(context.line_number, Some(2));
    assert_eq!(context.line.as_deref(), Some("1.0 0.1"));
    assert!(error.to_string().contains("uploaded.s1p:2"));
}

#[test]
fn from_str_records_missing_option_line_warning() {
    let network = Network::from_str("uploaded.s1p", "1.0 0.1 0.0\n").unwrap();

    assert_eq!(network.frequency_unit, "GHz");
    assert_eq!(network.format, "MA");
    assert_eq!(network.z0, 50.0);
    assert!(matches!(
        network.warnings.as_slice(),
        [TouchstoneWarning::MissingOptionLine { source_name }] if source_name == "uploaded.s1p"
    ));
}

#[test]
fn from_str_records_ignored_extra_option_line_warning() {
    let network = Network::from_str(
        "uploaded.s1p",
        "# GHz S RI R 50\n# MHz S RI R 75\n1.0 0.1 0.0\n",
    )
    .unwrap();

    assert_eq!(network.frequency_unit, "GHz");
    assert_eq!(network.z0, 50.0);
    assert!(matches!(
        network.warnings.as_slice(),
        [TouchstoneWarning::AdditionalOptionLineIgnored {
            source_name,
            line_number: 2,
            line,
        }] if source_name == "uploaded.s1p" && line == "# MHz S RI R 75"
    ));
}

#[test]
fn from_str_records_unknown_keyword_warning() {
    let network = Network::from_str(
        "uploaded.s1p",
        "[Version] 2.1\n[Unsupported Keyword] ignored\n# GHz S RI R 50\n[Network Data]\n1.0 0.1 0.0\n[End]\n",
    )
    .unwrap();

    assert!(matches!(
        network.warnings.as_slice(),
        [TouchstoneWarning::UnknownKeywordIgnored {
            source_name,
            line_number: 2,
            keyword,
        }] if source_name == "uploaded.s1p" && keyword == "unsupported keyword"
    ));
}

fn assert_from_memory_matches_new(path: &str) {
    let contents = fs::read_to_string(path).unwrap();
    let bytes = fs::read(path).unwrap();

    let from_file = Network::new(path).unwrap();
    let from_str = Network::from_str(path, &contents).unwrap();
    let from_bytes = Network::from_bytes(path, &bytes).unwrap();

    assert_same_network_shape(path, &from_file, &from_str);
    assert_same_network_shape(path, &from_file, &from_bytes);
}

fn assert_same_network_shape(path: &str, left: &Network, right: &Network) {
    assert_eq!(left.name, right.name, "{path}");
    assert_eq!(left.rank, right.rank, "{path}");
    assert_eq!(left.frequency_unit, right.frequency_unit, "{path}");
    assert_eq!(left.parameter, right.parameter, "{path}");
    assert_eq!(left.format, right.format, "{path}");
    assert_eq!(left.resistance_string, right.resistance_string, "{path}");
    assert_eq!(left.z0, right.z0, "{path}");
    assert_eq!(
        left.reference_impedance(),
        right.reference_impedance(),
        "{path}"
    );
    assert_eq!(left.comments, right.comments, "{path}");
    assert_eq!(
        left.comments_after_option_line, right.comments_after_option_line,
        "{path}"
    );
    assert_eq!(left.warnings, right.warnings, "{path}");
    assert_eq!(left.f, right.f, "{path}");
    assert_eq!(left.s.len(), right.s.len(), "{path}");

    for j in 1..=left.rank {
        for k in 1..=left.rank {
            let left_s = left.s_ri(j as i8, k as i8);
            let right_s = right.s_ri(j as i8, k as i8);
            assert_eq!(left_s.len(), right_s.len(), "{path} S{j}{k}");
            for (left_point, right_point) in left_s.iter().zip(right_s) {
                assert_eq!(
                    left_point.frequency, right_point.frequency,
                    "{path} S{j}{k}"
                );
                assert_eq!(left_point.s_ri, right_point.s_ri, "{path} S{j}{k}");
            }
        }
    }
}
