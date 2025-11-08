use std::fs;
mod data_line;
mod option_line;

#[derive(Debug)]
pub struct Network {
    // pub s: MagnitudeAngleMatrix,
    pub z0: f64,
    pub frequency_unit: String,
    pub name: String,
    pub options: option_line::Options,
    pub comments: Vec<String>,
    pub data_lines: Vec<String>,
}

fn is_valid_file_type(file_type: &str) -> bool {
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
    if middle_chars.len() < 1 {
        return false;
    }

    let middle_chars_are_digits = middle_chars.chars().all(|c| c.is_digit(10));

    // must be digits in the middle
    if !middle_chars_are_digits {
        return false;
    }

    // cannot start with 0
    if middle_chars.chars().next().unwrap() == '0' {
        return false;
    }

    // println!("middle chars: {middle_chars}");
    let middle_chars_as_int = middle_chars
        .parse::<i32>()
        .expect("Failed to parse middle chars as int {middle_chars}");

    let n_ports_valid = middle_chars_as_int >= 1;

    n_ports_valid
}

fn read_file(file_path: String) -> Network {
    let contents = fs::read_to_string(&file_path).expect("Should have been able to read the file");

    let file_type = file_path
        .split(".")
        .last()
        .expect("Failed to get file type from file path");

    let file_type_is_valid = is_valid_file_type(file_type);

    if !file_type_is_valid {
        panic!(
            "File type not supported: {}, only sNp supported, where n is an integer without leading zeros.",
            file_type
        );
    }

    let n_ports_str = &file_type[1..file_type.len() - 1];
    let n_ports = n_ports_str
        .parse::<i32>()
        .expect("Failed to parse number of ports from file type");

    let mut parsed_options = option_line::Options::default();
    // println!("default options:\n{:?}", parsed_options);

    let mut found_first_option_line = false;

    let mut comment_lines: Vec<String> = Vec::new();
    let mut data_lines: Vec<String> = Vec::new();

    for line in contents.lines() {
        // println!("\nWith line: ");

        let is_option_line = line.starts_with("#");
        let is_comment = line.starts_with("!");

        if is_option_line {
            if found_first_option_line == false {
                found_first_option_line = true;

                // println!("\nWith options: {line}");
                // mutate options as they are parsed
                option_line::parse_option_line(line.to_string(), &mut parsed_options)
            }
        } else {
            if is_comment {
                // println!("\nWith comment: {line}");
                comment_lines.push(line.to_string());
            } else {
                // is_data is true (not a variable, just communicating in terms of the pattern)

                // println!("\nWith data: {line}");
                let parts = line.split_whitespace().collect::<Vec<_>>();
                // println!("Data (len: {}):\n{:?}", parts.len(), parts);

                data_line::parse_data_line(line.to_string(), &parsed_options.format, &n_ports);
                data_lines.push(line.to_string());
            }
        }
    }
    println!("parsed options:\n{:?}", parsed_options);

    Network {
        // s: MagnitudeAngleMatrix(
        //     (crate::data_line::MagnitudeAngle(0.0, 0.0), crate::data_line::MagnitudeAngle(0.0, 0.0)),
        //     (crate::data_line::MagnitudeAngle(0.0, 0.0), crate::data_line::MagnitudeAngle(0.0, 0.0)),
        // ),
        z0: parsed_options
            .reference_resistance
            .parse::<f64>()
            .unwrap_or(50.0),
        frequency_unit: parsed_options.frequency_unit.clone(),
        name: String::from(file_path),
        options: parsed_options,
        comments: comment_lines,
        data_lines: data_lines,
    }
}

impl Network {
    pub fn new(file_path: String) -> Self {
        read_file(file_path)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn is_valid_file_type_single_port() {
        assert_eq!(super::is_valid_file_type("s1p"), true);
    }

    #[test]
    fn is_valid_file_type_expected_two_port() {
        assert_eq!(super::is_valid_file_type("s2p"), true);
    }

    #[test]
    fn is_valid_file_type_expected_three_port() {
        assert_eq!(super::is_valid_file_type("s3p"), true);
    }

    #[test]
    fn is_valid_file_type_expected_four_port() {
        assert_eq!(super::is_valid_file_type("s4p"), true);
    }

    #[test]
    fn is_valid_file_type_large_values() {
        assert_eq!(super::is_valid_file_type("s10p"), true);
        assert_eq!(super::is_valid_file_type("s217p"), true);
    }

    #[test]
    fn is_valid_file_type_zeros() {
        assert_eq!(super::is_valid_file_type("s0p"), false);
        assert_eq!(super::is_valid_file_type("s01p"), false);
    }

    #[test]
    fn is_valid_file_type_other_extensions() {
        assert_eq!(super::is_valid_file_type("txt"), false);
        assert_eq!(super::is_valid_file_type("sxp"), false);
        assert_eq!(super::is_valid_file_type("s2x"), false);
        assert_eq!(super::is_valid_file_type("x2p"), false);
        assert_eq!(super::is_valid_file_type("2p"), false);
        assert_eq!(super::is_valid_file_type("s2"), false);
        assert_eq!(super::is_valid_file_type("sp"), false);
        assert_eq!(super::is_valid_file_type("s"), false);
        assert_eq!(super::is_valid_file_type("1p"), false);
    }

    #[test]
    fn is_valid_file_type_no_extension() {
        assert_eq!(super::is_valid_file_type(""), false);
    }

    #[test]
    fn parse_2port() {
        let network = crate::read_file("files/2port.s2p".to_string());

        assert_eq!(network.options.frequency_unit, "GHz");
        assert_eq!(network.options.parameter, "S");
        assert_eq!(network.options.format, "RI");
        assert_eq!(network.options.resistance_string, "R");
        assert_eq!(network.options.reference_resistance, "50");

        assert_eq!(network.z0, 50.0);
        assert_eq!(network.frequency_unit, "GHz");
        assert_eq!(network.name, "files/2port.s2p".to_string());

        assert_eq!(network.comments.len(), 4);
        assert_eq!(network.data_lines.len(), 91);
    }
}
