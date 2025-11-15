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
}

pub fn read_file(file_path: String) -> Network {
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

    for line in contents.lines() {
        // println!("\nWith line: ");

        let is_option_line = line.starts_with("#");
        let is_comment = line.starts_with("!");

        if is_option_line {
            if parser_state.option_line_parsed == false {
                // println!("\nWith options: {line}");
                // mutate options as they are parsed
                option_line::parse_option_line(line.to_string(), &mut parsed_options);

                frequency_unit = parsed_options.frequency_unit.clone();
                parameter = parsed_options.parameter.clone();
                format = parsed_options.format.clone();
                resistance_string = parsed_options.resistance_string.clone();
                reference_resistance = parsed_options.reference_resistance.clone();

                parser_state.option_line_parsed = true;
            } else {
                panic!("Multiple option lines found in file. Only one option line is allowed.");
            }
        } else {
            if is_comment {
                // println!("\nWith comment: {line}");
                if parser_state.option_line_parsed == false {
                    comment_lines.push(line.to_string());
                } else {
                    comments_after_option_line.push(line.to_string());
                }
            } else {
                // is_data is true (not a variable, just communicating in terms of the pattern)

                // println!("\nWith data: {line}");
                // let parts = line.split_whitespace().collect::<Vec<_>>();
                // println!("Data (len: {}):\n{:?}", parts.len(), parts);

                let line_matrix_data = data_line::parse_data_line(
                    line.to_string(),
                    &parsed_options.format,
                    &n_ports,
                    &parsed_options.frequency_unit,
                );
                parser_state.data_lines.push(line.to_string());

                f.push(line_matrix_data.frequency);

                s.push(line_matrix_data);
            }
        }
    }

    // println!("parsed options:\n{:?}", parsed_options);

    Network {
        name: file_path,
        rank: n_ports,
        frequency_unit: frequency_unit,
        parameter: parameter,
        format: format,
        resistance_string: resistance_string,
        z0: utils::str_to_f64(reference_resistance.as_str()),
        comments: comment_lines,
        comments_after_option_line: comments_after_option_line,
        f: f,
        s: s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
