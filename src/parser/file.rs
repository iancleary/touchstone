use crate::parser::data_line;
use crate::parser::option_line;
use std::fs;

pub fn read_file(file_path: String) -> option_line::Options {
    let contents = fs::read_to_string(file_path).expect("Should have been able to read the file");

    let mut parsed_options = option_line::Options::default();
    println!("default options:\n{:?}", parsed_options);

    let mut found_first_option_line = false;

    for line in contents.lines() {
        // println!("\nWith line: ");

        let is_option_line = line.starts_with("#");
        let is_comment = line.starts_with("!");

        if is_option_line {
            if found_first_option_line == false {
                found_first_option_line = true;

                println!("\nWith options: {line}");
                // mutate options as they are parsed
                option_line::parse_option_line(line.to_string(), &mut parsed_options)
            }
        } else {
            if is_comment {
                println!("\nWith comment: {line}");
            } else {
                // is_data is true (not a variable, just communicating in terms of the pattern)

                // println!("\nWith data: {line}");
                let parts = line.split_whitespace().collect::<Vec<_>>();
                println!("Data (len: {}):\n{:?}", parts.len(), parts);

                data_line::parse_data_line(line.to_string(), &parsed_options.format);
            }
        }
    }
    println!("parsed options:\n{:?}", parsed_options);

    parsed_options
}

#[cfg(test)]
mod tests {

    use super::read_file;
    #[test]
    fn parse_2port() {
        let options = read_file("files/2port.s2p".to_string());

        assert_eq!(options.frequency_unit, "GHz");
        assert_eq!(options.parameter, "S");
        assert_eq!(options.format, "RI");
        assert_eq!(options.resistance_string, "R");
        assert_eq!(options.reference_resistance, "50");
    }
}
