pub(crate) fn is_valid_file_extension(file_type: &str) -> bool {
    // Strip ASCII delimiters without slicing through a possible UTF-8 code point.
    let Some(port_digits) = file_type
        .strip_prefix('s')
        .and_then(|s| s.strip_suffix('p'))
    else {
        return false;
    };

    !port_digits.starts_with('0')
        && port_digits.bytes().all(|byte| byte.is_ascii_digit())
        && port_digits.parse::<i32>().is_ok_and(|ports| ports > 0)
}

#[cfg(test)]
mod tests {
    #[test]
    fn is_valid_file_extension_single_port() {
        assert!(super::is_valid_file_extension("s1p"));
    }

    #[test]
    fn is_valid_file_extension_expected_two_port() {
        assert!(super::is_valid_file_extension("s2p"));
    }

    #[test]
    fn is_valid_file_extension_expected_three_port() {
        assert!(super::is_valid_file_extension("s3p"));
    }

    #[test]
    fn is_valid_file_extension_expected_four_port() {
        assert!(super::is_valid_file_extension("s4p"));
    }

    #[test]
    fn is_valid_file_extension_large_values() {
        assert!(super::is_valid_file_extension("s10p"));
        assert!(super::is_valid_file_extension("s217p"));
    }

    #[test]
    fn is_valid_file_extension_zeros() {
        assert!(!super::is_valid_file_extension("s0p"));
        assert!(!super::is_valid_file_extension("s01p"));
    }

    #[test]
    fn is_valid_file_extension_other_extensions() {
        assert!(!super::is_valid_file_extension("txt"));
        assert!(!super::is_valid_file_extension("sxp"));
        assert!(!super::is_valid_file_extension("s2x"));
        assert!(!super::is_valid_file_extension("x2p"));
        assert!(!super::is_valid_file_extension("2p"));
        assert!(!super::is_valid_file_extension("s2"));
        assert!(!super::is_valid_file_extension("sp"));
        assert!(!super::is_valid_file_extension("s"));
        assert!(!super::is_valid_file_extension("1p"));
    }

    #[test]
    fn is_valid_file_extension_no_extension() {
        assert!(!super::is_valid_file_extension(""));
    }
}
