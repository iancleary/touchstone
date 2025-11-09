use std::fs;
mod data_line;
mod file_extension;
mod option_line;
mod utils;

#[derive(Debug)]
pub struct Network {
    // pub s: MagnitudeAngleMatrix,
    pub z0: f64,
    pub frequency_unit: String,
    pub name: String,
    pub options: option_line::Options,
    pub comments: Vec<String>,
    pub data_lines: Vec<String>,
    pub s: Vec<data_line::ParsedDataLine>,
}

fn read_file(file_path: String) -> Network {
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

    let mut found_first_option_line = false;

    let mut comment_lines: Vec<String> = Vec::new();
    let mut data_lines: Vec<String> = Vec::new();
    let mut s: Vec<data_line::ParsedDataLine> = Vec::new();

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
                // let parts = line.split_whitespace().collect::<Vec<_>>();
                // println!("Data (len: {}):\n{:?}", parts.len(), parts);

                let line_matrix_data = data_line::parse_data_line(
                    line.to_string(),
                    &parsed_options.format,
                    &n_ports,
                    &parsed_options.frequency_unit,
                );
                data_lines.push(line.to_string());
                s.push(line_matrix_data);
            }
        }
    }
    println!("parsed options:\n{:?}", parsed_options);

    Network {
        z0: parsed_options
            .reference_resistance
            .parse::<f64>()
            .unwrap_or(50.0),
        frequency_unit: parsed_options.frequency_unit.clone(),
        name: String::from(file_path),
        options: parsed_options,
        comments: comment_lines,
        data_lines: data_lines,
        s: s,
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
