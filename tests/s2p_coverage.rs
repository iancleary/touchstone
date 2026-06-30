use std::fs;

use touchstone::{Network, TouchstoneError};

const SOURCE_COMMIT: &str = "950534a5928d5c99e3fea2beaec7d82519800b0c";
const LICENSE_COMMENT: &str = "! License: BSD-3-Clause";

const S2P_FIXTURES: &[&str] = &[
    "files/LFCN-2352+_Plus125degC.s2p",
    "files/LFCN-2352+_Plus25degC.s2p",
    "files/RS_ZVR_1.20_beta_f.s2p",
    "files/fet.s2p",
    "files/hfss_twoport.s2p",
    "files/ntwk1.s2p",
    "files/ntwk2.s2p",
    "files/ntwk3.s2p",
    "files/ntwk4.s2p",
    "files/ntwk4_n.s2p",
    "files/ntwk_arbitrary_frequency.s2p",
    "files/ntwk_noise.s2p",
    "files/ntwk_noise_interp.s2p",
    "files/ntwks_ntwk1.s2p",
    "files/ntwks_ntwk2.s2p",
    "files/ntwks_ntwk3.s2p",
    "files/thru.s2p",
];

const PARSABLE_S2P_FIXTURES: &[&str] = &[
    "files/LFCN-2352+_Plus125degC.s2p",
    "files/LFCN-2352+_Plus25degC.s2p",
    "files/RS_ZVR_1.20_beta_f.s2p",
    "files/fet.s2p",
    "files/hfss_twoport.s2p",
    "files/ntwk1.s2p",
    "files/ntwk2.s2p",
    "files/ntwk3.s2p",
    "files/ntwk4.s2p",
    "files/ntwk_arbitrary_frequency.s2p",
    "files/ntwks_ntwk1.s2p",
    "files/ntwks_ntwk2.s2p",
    "files/ntwks_ntwk3.s2p",
];

const UNSUPPORTED_NOISE_PARAMETER_FIXTURES: &[&str] = &[
    // These fixtures include Touchstone noise-parameter records after the ordinary
    // two-port network data. The current parser only supports network data rows.
    "files/ntwk4_n.s2p",
    "files/ntwk_noise.s2p",
    "files/ntwk_noise_interp.s2p",
    "files/thru.s2p",
];

#[test]
fn s2p_fixtures_include_source_and_license_comments() {
    for path in S2P_FIXTURES {
        let contents = fs::read_to_string(path).unwrap();
        let mut lines = contents.lines();
        let source = lines.next().unwrap();
        let license = lines.next().unwrap();

        assert!(
            source.starts_with("! Source: https://github.com/"),
            "{path}"
        );
        assert!(source.contains(SOURCE_COMMIT), "{path}");
        assert!(source.ends_with(".s2p"), "{path}");
        assert_eq!(license, LICENSE_COMMENT);
    }
}

#[test]
fn from_str_matches_new_for_s2p_fixtures() {
    for path in PARSABLE_S2P_FIXTURES {
        let contents = fs::read_to_string(path).unwrap();
        let from_file = Network::new(path).unwrap_or_else(|err| panic!("{path}: {err:?}"));
        let from_str =
            Network::from_str(path, &contents).unwrap_or_else(|err| panic!("{path}: {err:?}"));

        assert_same_parse(path, &from_file, &from_str);
    }
}

#[test]
fn from_bytes_matches_new_for_s2p_fixtures() {
    for path in PARSABLE_S2P_FIXTURES {
        let bytes = fs::read(path).unwrap();
        let from_file = Network::new(path).unwrap_or_else(|err| panic!("{path}: {err:?}"));
        let from_bytes =
            Network::from_bytes(path, &bytes).unwrap_or_else(|err| panic!("{path}: {err:?}"));

        assert_same_parse(path, &from_file, &from_bytes);
    }
}

#[test]
fn from_memory_returns_errors_for_noise_parameter_fixtures() {
    for path in UNSUPPORTED_NOISE_PARAMETER_FIXTURES {
        let contents = fs::read_to_string(path).unwrap();
        let bytes = fs::read(path).unwrap();

        let from_str = Network::from_str(path, &contents).unwrap_err();
        let from_bytes = Network::from_bytes(path, &bytes).unwrap_err();

        assert!(
            matches!(
                from_str.root_cause(),
                TouchstoneError::InvalidDataLineParts {
                    expected: 9,
                    actual: 5 | 10
                }
            ),
            "{path}: {from_str:?}"
        );
        assert!(
            matches!(
                from_bytes.root_cause(),
                TouchstoneError::InvalidDataLineParts {
                    expected: 9,
                    actual: 5 | 10
                }
            ),
            "{path}: {from_bytes:?}"
        );
    }
}

fn assert_same_parse(path: &str, left: &Network, right: &Network) {
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
}
