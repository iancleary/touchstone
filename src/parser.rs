use std::fs;

use crate::data_line;
use crate::file_extension;
use crate::option_line;
use crate::utils;
use crate::Network;

#[derive(Debug)]
struct ParserState {
    data_lines: Vec<String>,
    option_line_parsed: bool,
    two_port_data_order: data_line::TwoPortDataOrder,
    expected_number_of_frequencies: Option<usize>,
}

pub fn read_file(file_path: String) -> Network {
    tracing::debug!("Parsing touchstone file: {}", file_path);
    let contents = fs::read_to_string(&file_path).expect("Should have been able to read the file");

    let file_type = file_path
        .split(".")
        .last()
        .expect("Failed to get file type from file path");

    let file_type_is_valid = file_extension::is_valid_file_extension(file_type);

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

    let mut parser_state = ParserState {
        option_line_parsed: false,
        data_lines: Vec::new(),
        two_port_data_order: data_line::TwoPortDataOrder::default(),
        expected_number_of_frequencies: None,
    };

    let mut comment_lines: Vec<String> = Vec::new();
    let mut comments_after_option_line: Vec<String> = Vec::new();
    let mut s: Vec<data_line::ParsedDataLine> = Vec::new();
    let mut f: Vec<f64> = Vec::new();
    let mut frequency_unit = String::new();
    let mut parameter = String::new();
    let mut format = String::new();
    let mut resistance_string = String::new();
    let mut reference_resistance = String::new();

    // Helper function to count numeric values on a line (excluding inline comments)
    let count_values_on_line = |line: &str| -> usize {
        line.split('!')
            .next()
            .unwrap_or("")
            .split_whitespace()
            .count()
    };

    // Expected number of values per complete data entry
    let expected_values = (1 + 2 * n_ports * n_ports) as usize;

    let mut current_data_segment: Vec<String> = Vec::new();
    let mut current_value_count: usize = 0;

    for line in contents.lines() {
        // println!("\nWith line: ");

        let trimmed_line = line.trim();
        let is_option_line = trimmed_line.starts_with("#");
        let is_comment = trimmed_line.starts_with("!");
        let is_keyword = trimmed_line.starts_with("[");

        if is_option_line {
            if !parser_state.option_line_parsed {
                // println!("\nWith options: {line}");
                // mutate options as they are parsed
                option_line::parse_option_line(trimmed_line.to_string(), &mut parsed_options);

                frequency_unit = parsed_options.frequency_unit.clone();
                parameter = parsed_options.parameter.clone();
                format = parsed_options.format.clone();
                resistance_string = parsed_options.resistance_string.clone();
                reference_resistance = parsed_options.reference_resistance.clone();

                parser_state.option_line_parsed = true;
            } else {
                panic!("Multiple option lines found in file. Only one option line is allowed.");
            }
        } else if is_keyword {
            if handle_keyword_line(trimmed_line, n_ports, &mut parser_state) {
                break;
            }
        } else if is_comment {
            // println!("\nWith comment: {line}");
            if !parser_state.option_line_parsed {
                comment_lines.push(line.to_string());
            } else {
                comments_after_option_line.push(line.to_string());
            }
        } else if !line.trim().is_empty() {
            // is_data is true (not a variable, just communicating in terms of the pattern)

            // Add line to current segment
            current_data_segment.push(line.to_string());
            current_value_count += count_values_on_line(line);

            // Check if we have collected enough values for a complete entry
            if current_value_count >= expected_values {
                // Process this complete segment
                let line_matrix_data = data_line::parse_data_line_with_order(
                    current_data_segment.clone(),
                    &parsed_options.format,
                    &n_ports,
                    &parsed_options.frequency_unit,
                    parser_state.two_port_data_order,
                );

                f.push(line_matrix_data.frequency);
                s.push(line_matrix_data);

                parser_state.data_lines.extend(current_data_segment.clone());

                // Reset for next segment
                current_data_segment.clear();
                current_value_count = 0;
            }
        }
    }

    // Process the last data segment
    if !current_data_segment.is_empty() {
        let line_matrix_data = data_line::parse_data_line_with_order(
            current_data_segment.clone(),
            &parsed_options.format,
            &n_ports,
            &parsed_options.frequency_unit,
            parser_state.two_port_data_order,
        );

        f.push(line_matrix_data.frequency);
        s.push(line_matrix_data);

        parser_state.data_lines.extend(current_data_segment);
    }

    if let Some(expected_number_of_frequencies) = parser_state.expected_number_of_frequencies {
        if expected_number_of_frequencies != f.len() {
            panic!(
                "[Number of Frequencies] expected {} points, parsed {}",
                expected_number_of_frequencies,
                f.len()
            );
        }
    }

    tracing::debug!(
        num_ports = n_ports,
        num_frequencies = f.len(),
        format = %parsed_options.format,
        frequency_unit = %parsed_options.frequency_unit,
        "Parsing complete"
    );

    Network {
        name: file_path,
        rank: n_ports,
        frequency_unit,
        parameter,
        format,
        resistance_string,
        z0: utils::str_to_f64(reference_resistance.as_str()),
        comments: comment_lines,
        comments_after_option_line,
        f,
        s,
    }
}

fn handle_keyword_line(line: &str, n_ports: i32, parser_state: &mut ParserState) -> bool {
    let line_without_comment = line.split('!').next().unwrap_or("").trim();
    let closing_bracket_index = line_without_comment
        .find(']')
        .unwrap_or_else(|| panic!("Invalid Touchstone keyword line: {}", line));
    let keyword = normalize_keyword(&line_without_comment[1..closing_bracket_index]);
    let argument = line_without_comment[closing_bracket_index + 1..].trim();

    match keyword.as_str() {
        "version" => match argument {
            "2.0" | "2.1" => {}
            _ => panic!("Unsupported [Version]: {}", argument),
        },
        "number of ports" => {
            let keyword_ports = argument
                .parse::<i32>()
                .unwrap_or_else(|_| panic!("Invalid [Number of Ports]: {}", argument));
            if keyword_ports != n_ports {
                panic!(
                    "[Number of Ports] {} does not match file extension port count {}",
                    keyword_ports, n_ports
                );
            }
        }
        "two port data order" => {
            if n_ports != 2 {
                panic!("[Two-Port Data Order] is only valid for 2-port files");
            }
            parser_state.two_port_data_order =
                data_line::TwoPortDataOrder::from_keyword_argument(argument);
        }
        "number of frequencies" => {
            parser_state.expected_number_of_frequencies = Some(
                argument
                    .parse::<usize>()
                    .unwrap_or_else(|_| panic!("Invalid [Number of Frequencies]: {}", argument)),
            );
        }
        "network data" => {}
        "matrix format" => {
            if !argument.eq_ignore_ascii_case("Full") {
                panic!("Only [Matrix Format] Full is currently supported");
            }
        }
        "end" => return true,
        _ => {}
    }

    false
}

fn normalize_keyword(keyword: &str) -> String {
    keyword
        .to_ascii_lowercase()
        .replace('-', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_temp_touchstone_file(name: &str, contents: &str) -> std::path::PathBuf {
        let temp_dir = std::env::temp_dir().join("touchstone_parser_tests");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let file_path = temp_dir.join(format!("{}_{}.s2p", name, nanos));
        std::fs::write(&file_path, contents).unwrap();
        file_path
    }

    #[test]
    fn parse_legacy_2port_defaults_to_21_12_order() {
        let file_path = write_temp_touchstone_file(
            "legacy_21_12",
            "# GHz S RI R 50\n1.0 0.1 0.0 4.0 0.0 0.01 0.0 0.2 0.0\n",
        );

        let network = read_file(file_path.to_str().unwrap().to_string());

        assert_eq!(
            network.s_ri(2, 1)[0].s_ri,
            crate::data_pairs::RealImaginary(4.0, 0.0)
        );
        assert_eq!(
            network.s_ri(1, 2)[0].s_ri,
            crate::data_pairs::RealImaginary(0.01, 0.0)
        );

        std::fs::remove_file(file_path).unwrap();
    }

    #[test]
    fn parse_2_1_2port_accepts_explicit_12_21_order() {
        let file_path = write_temp_touchstone_file(
            "version_2_1_12_21",
            "\
[Version] 2.1
# GHz S RI R 50
[Number of Ports] 2
[Two-Port Data Order] 12_21
[Number of Frequencies] 1
[Matrix Format] Full
[Network Data]
1.0 0.1 0.0 0.01 0.0 4.0 0.0 0.2 0.0
[End]
",
        );

        let network = read_file(file_path.to_str().unwrap().to_string());

        assert_eq!(
            network.s_ri(2, 1)[0].s_ri,
            crate::data_pairs::RealImaginary(4.0, 0.0)
        );
        assert_eq!(
            network.s_ri(1, 2)[0].s_ri,
            crate::data_pairs::RealImaginary(0.01, 0.0)
        );

        std::fs::remove_file(file_path).unwrap();
    }

    #[test]
    fn parse_2port_ntwk1() {
        let network = read_file("files/ntwk1.s2p".to_string());

        assert_eq!(network.name, "files/ntwk1.s2p".to_string());

        assert_eq!(network.rank, 2);

        assert_eq!(network.frequency_unit, "GHz");
        assert_eq!(network.parameter, "S");
        assert_eq!(network.format, "RI");
        assert_eq!(network.resistance_string, "R");
        assert_eq!(network.z0, 50.0);

        assert_eq!(network.comments.len(), 1);
        assert_eq!(network.comments_after_option_line.len(), 3);
        assert_eq!(network.f.len(), 91);
        assert_eq!(network.s.len(), 91);
    }

    #[test]
    fn parse_2port_ntwk2() {
        let network = read_file("files/ntwk2.s2p".to_string());

        assert_eq!(network.name, "files/ntwk2.s2p".to_string());

        assert_eq!(network.rank, 2);

        assert_eq!(network.frequency_unit, "GHz");
        assert_eq!(network.parameter, "S");
        assert_eq!(network.format, "RI");
        assert_eq!(network.resistance_string, "R");
        assert_eq!(network.z0, 50.0);

        assert_eq!(network.comments.len(), 1);
        assert_eq!(network.comments_after_option_line.len(), 3);
        assert_eq!(network.f.len(), 91);
        assert_eq!(network.s.len(), 91);
    }

    #[test]
    fn parse_2port_ntwk3() {
        let network = read_file("files/ntwk3.s2p".to_string());
        assert_eq!(network.name, "files/ntwk3.s2p".to_string());

        assert_eq!(network.rank, 2);

        assert_eq!(network.frequency_unit, "GHz");
        assert_eq!(network.parameter, "S");
        assert_eq!(network.format, "RI");
        assert_eq!(network.resistance_string, "R");
        assert_eq!(network.z0, 50.0);

        assert_eq!(network.comments.len(), 1);
        assert_eq!(network.comments_after_option_line.len(), 3);
        assert_eq!(network.f.len(), 91);
        assert_eq!(network.s.len(), 91);
    }

    #[test]
    fn parse_3port_hfss() {
        let network = read_file("files/hfss_18.2.s3p".to_string());
        assert_eq!(network.name, "files/hfss_18.2.s3p".to_string());

        assert_eq!(network.rank, 3);

        assert_eq!(network.frequency_unit, "GHz");
        assert_eq!(network.parameter, "S");
        assert_eq!(network.format, "MA");
        assert_eq!(network.resistance_string, "R");
        assert_eq!(network.z0, 50.0);

        // Multi-line format file
        assert!(!network.f.is_empty());
        assert_eq!(network.f.len(), network.s.len());

        // Verify we can access all 9 S-parameters (3x3 matrix)
        for i in 1..=3 {
            for j in 1..=3 {
                let s_db = network.s_db(i as i8, j as i8);
                assert_eq!(s_db.len(), network.f.len());
            }
        }
    }

    #[test]
    fn parse_4port_agilent() {
        let network = read_file("files/Agilent_E5071B.s4p".to_string());
        assert_eq!(network.name, "files/Agilent_E5071B.s4p".to_string());

        assert_eq!(network.rank, 4);

        assert_eq!(network.frequency_unit, "Hz");
        assert_eq!(network.parameter, "S");
        assert_eq!(network.format, "DB");
        assert_eq!(network.resistance_string, "R");
        assert_eq!(network.z0, 75.0);

        // Multi-line format file
        assert!(!network.f.is_empty());
        assert_eq!(network.f.len(), network.s.len());

        // Verify we can access all 16 S-parameters (4x4 matrix)
        for i in 1..=4 {
            for j in 1..=4 {
                let s_db = network.s_db(i as i8, j as i8);
                assert_eq!(s_db.len(), network.f.len());
            }
        }
    }
}
