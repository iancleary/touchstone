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

}

fn read_file(file_path: String) -> Network {
    let contents = fs::read_to_string(&file_path).expect("Should have been able to read the file");

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

                println!("\nWith data: {line}");
                let parts = line.split_whitespace().collect::<Vec<_>>();
                println!("Data (len: {}):\n{:?}", parts.len(), parts);

                data_line::parse_data_line(line.to_string(), &parsed_options.format);
            }
        }
    }
    println!("parsed options:\n{:?}", parsed_options);

    Network {
        // s: MagnitudeAngleMatrix(
        //     (crate::data_line::MagnitudeAngle(0.0, 0.0), crate::data_line::MagnitudeAngle(0.0, 0.0)),
        //     (crate::data_line::MagnitudeAngle(0.0, 0.0), crate::data_line::MagnitudeAngle(0.0, 0.0)),
        // ),
        z0: parsed_options.reference_resistance.parse::<f64>().unwrap_or(50.0),
        frequency_unit: parsed_options.frequency_unit.clone(),
        name: String::from(file_path),
        options: parsed_options,
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
    }
}