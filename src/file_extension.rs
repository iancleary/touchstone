pub(crate) fn is_valid_file_extension(file_type: &str) -> bool {
    // println!("Validating file type: {file_type}");
    let file_type_length = file_type.len();

    // println!("file type length: {file_type_length}");
    if file_type_length < 1 {
        return false;
    }

    let first_char = &file_type[0..1];
    let first_char_is_s = first_char == "s";

    if !first_char_is_s {
        return false;
    }

    let last_char = &file_type[file_type_length - 1..file_type_length];
    let last_char_is_p = last_char == "p";

    if !last_char_is_p {
        return false;
    }

    let middle_chars = &file_type[1..file_type_length - 1];

    // must have at least one character in the middle
    // these are the number of ports, which must be defined
    if middle_chars.is_empty() {
        return false;
    }

    let middle_chars_are_digits = middle_chars.chars().all(|c| c.is_ascii_digit());

    // must be digits in the middle
    if !middle_chars_are_digits {
        return false;
    }

    // cannot start with 0
    if middle_chars.starts_with('0') {
        return false;
    }

    // println!("middle chars: {middle_chars}");
    let middle_chars_as_int = middle_chars
        .parse::<i32>()
        .expect("Failed to parse middle chars as int {middle_chars}");

    middle_chars_as_int >= 1
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
