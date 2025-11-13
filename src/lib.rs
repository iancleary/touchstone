use std::fs;
use std::ops;
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

    pub fn cascade(&self, other: &Network) -> Network {
        
        if self.z0 != other.z0 {
            panic!("Cannot cascade networks with different reference impedances: {} and {}", self.z0, other.z0);
        }

        if self.frequency_unit != other.frequency_unit {
            panic!("Cannot cascade networks with different frequency units: {} and {}", self.frequency_unit, other.frequency_unit);
        }

        let new_name = format!("Cascaded({},{})", self.name, other.name);

        Network {
            z0: self.z0,
            frequency_unit: self.frequency_unit.clone(),
            name: new_name,
            options: self.options.clone(),
            comments: vec![],
            data_lines: vec![],
            s: vec![],
        }
    }
}

// The `std::ops::Mul` trait is used to specify the functionality of `+`.
// Here, we make `Mul<Network>` - the trait for addition with a RHS of type `Network`.
// The following block implements the operation: Foo * Bar = FooBar
impl ops::Mul<Network> for Network {
    type Output = Network;

    fn mul(self, _rhs: Network) -> Network {
        println!("> Network.multiply(Network) was called");

        self.cascade(&_rhs)
    }
}


#[cfg(test)]
mod tests {

    #[test]
    fn parse_2port_ntwk1() {
        let network = crate::read_file("files/ntwk1.s2p".to_string());

        assert_eq!(network.options.frequency_unit, "GHz");
        assert_eq!(network.options.parameter, "S");
        assert_eq!(network.options.format, "RI");
        assert_eq!(network.options.resistance_string, "R");
        assert_eq!(network.options.reference_resistance, "50");

        assert_eq!(network.z0, 50.0);
        assert_eq!(network.frequency_unit, "GHz");
        assert_eq!(network.name, "files/ntwk1.s2p".to_string());

        assert_eq!(network.comments.len(), 4);
        assert_eq!(network.data_lines.len(), 91);
    }

    #[test]
    fn parse_2port_ntwk2() {
        let network = crate::read_file("files/ntwk2.s2p".to_string());

        assert_eq!(network.options.frequency_unit, "GHz");
        assert_eq!(network.options.parameter, "S");
        assert_eq!(network.options.format, "RI");
        assert_eq!(network.options.resistance_string, "R");
        assert_eq!(network.options.reference_resistance, "50");

        assert_eq!(network.z0, 50.0);
        assert_eq!(network.frequency_unit, "GHz");
        assert_eq!(network.name, "files/ntwk2.s2p".to_string());

        assert_eq!(network.comments.len(), 4);
        assert_eq!(network.data_lines.len(), 91);
    }

    #[test]
    fn parse_2port_ntwk3() {
        let network = crate::read_file("files/ntwk3.s2p".to_string());

        assert_eq!(network.options.frequency_unit, "GHz");
        assert_eq!(network.options.parameter, "S");
        assert_eq!(network.options.format, "RI");
        assert_eq!(network.options.resistance_string, "R");
        assert_eq!(network.options.reference_resistance, "50");

        assert_eq!(network.z0, 50.0);
        assert_eq!(network.frequency_unit, "GHz");
        assert_eq!(network.name, "files/ntwk3.s2p".to_string());

        assert_eq!(network.comments.len(), 4);
        assert_eq!(network.data_lines.len(), 91);
    }

    #[test]
    fn cascade_2port_networks() {
        let network1 = crate::read_file("files/ntwk1.s2p".to_string());
        let network2 = crate::read_file("files/ntwk2.s2p".to_string());

        let _cascaded_network = network1.cascade(&network2);

        // assert_eq!(cascaded_network.comments.len(), 4);
        // assert_eq!(cascaded_network.data_lines.len(), 91);
        assert!(true);
    }

    #[test]
    fn cascade_2port_networks_multiple() {
        let network1 = crate::read_file("files/ntwk1.s2p".to_string());
        let network2 = crate::read_file("files/ntwk2.s2p".to_string());

        let _cascaded_network = network1 * network2;

        let network3 = crate::read_file("files/ntwk3.s2p".to_string());

        // assert_eq!(cascaded_network.comments.len(), 4);
        // assert_eq!(cascaded_network.data_lines.len(), 91);

        let num_data_lines = network3.s.len();
        println!("Number of data lines in cascaded network: {}", num_data_lines);
        // assert_eq!(num_data_lines, 42); // debug
        for i in 0..num_data_lines {
            println!("Data line {}: {:?}", i + 1, _cascaded_network.s[i]);
            assert_eq!(_cascaded_network.s[i].frequency, network3.s[i].frequency);
            assert_eq!(_cascaded_network.s[i].s_ri, network3.s[i].s_ri);
        }
    }
}
