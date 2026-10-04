use touchstone::Network;

/// Exercise filenames and file bodies together. The first line is the source
/// name; remaining bytes are the file body. Invalid UTF-8 bodies are intentional.
pub fn exercise(input: &[u8]) {
    let split = input
        .iter()
        .position(|&byte| byte == b'\n')
        .unwrap_or(input.len());
    let source = String::from_utf8_lossy(&input[..split]);
    let body = input.get(split + 1..).unwrap_or_default();
    parse_both(&source, body);

    // Fixed valid names let arbitrary bodies reach parsing beyond extension
    // validation even when mutation destroys the structured source name.
    for source in ["fuzz.s1p", "fuzz.s2p", "fuzz.s3p"] {
        parse_both(source, body);
    }
    parse_both("fuzz.s1p", input);
}

fn parse_both(source: &str, contents: &[u8]) {
    let bytes_result = Network::from_bytes(source, contents);
    if let Ok(text) = std::str::from_utf8(contents) {
        let string_result = Network::from_str(source, text);
        assert_eq!(bytes_result.is_ok(), string_result.is_ok());
    } else {
        assert!(bytes_result.is_err());
    }
}
