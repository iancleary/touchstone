use std::ops;
mod data_line;
mod data_pairs;
mod file_extension;
mod option_line;
mod parser;
mod utils;

#[derive(Debug)]
pub struct Network {
    pub name: String,
    pub rank: i32,
    pub frequency_unit: String,
    pub parameter: String,
    pub format: String,
    pub resistance_string: String, // "R"
    pub z0: f64,                   // If "R" is not present, this is 50
    pub comments: Vec<String>,
    pub comments_after_option_line: Vec<String>,

    // data
    pub f: Vec<f64>,
    pub s: Vec<data_line::ParsedDataLine>,
}

impl Network {
    pub fn new(file_path: String) -> Self {
        parser::read_file(file_path)
    }

    pub fn cascade(&self, other: &Network) -> Network {
        if self.rank != 2 || other.rank != 2 {
            panic!("Cascading is only implemented for 2-port networks.");
        }

        if self.z0 != other.z0 {
            panic!(
                "Cannot cascade networks with different reference impedances: {} and {}",
                self.z0, other.z0
            );
        }

        // can avoid this by converting other.f to use self.frequency_unit instead of other.frequency_unit
        if self.frequency_unit != other.frequency_unit {
            panic!(
                "Cannot cascade networks with different frequency units: {} and {}",
                self.frequency_unit, other.frequency_unit
            );
        }

        let mut comments = Vec::<String>::new();
        comments.push(format!(
            "! Cascaded network of {} and {}",
            self.name, other.name
        ));
        let comment_header_self = format!("! Comments from first network ({:?}):", self.name);
        comments.push(comment_header_self);
        for comment in &self.comments {
            comments.push(comment.clone());
        }
        let comment_header_other = format!("! Comments from second network ({:?}):", other.name);
        comments.push(comment_header_other);

        for comment in &other.comments {
            comments.push(comment.clone());
        }

        let mut comments_after_option_line = Vec::<String>::new();
        comments_after_option_line.push(format!(
            "! Cascaded network of {} and {}",
            self.name, other.name
        ));
        let comments_after_option_line_header_self = format!(
            "! Comments (after option line) from first network ({:?}):",
            self.name
        );
        comments_after_option_line.push(comments_after_option_line_header_self);
        for comment_after_option_line in &self.comments_after_option_line {
            comments_after_option_line.push(comment_after_option_line.clone());
        }
        let comments_after_option_line_header_other = format!(
            "! Comments (after option line) from second network ({:?}):",
            other.name
        );
        comments_after_option_line.push(comments_after_option_line_header_other);

        for comment_after_option_line in &other.comments_after_option_line {
            comments_after_option_line.push(comment_after_option_line.clone());
        }

        let new_name = format!("Cascaded({},{})", self.name, other.name);

        // needs to be reworked for ABCD cascade of 2 port networks
        Network {
            name: new_name,
            rank: self.rank,
            frequency_unit: self.frequency_unit.clone(),
            parameter: self.parameter.clone(),
            format: self.format.clone(),
            resistance_string: self.resistance_string.clone(),
            z0: self.z0,
            comments: comments,
            comments_after_option_line: comments_after_option_line,
            f: self.f.clone(),
            s: self.s.clone(), // TODO: implement proper cascading of S-parameters
        }
    }
}

// The `std::ops::Mul` trait is used to specify the functionality of `+`.
// Here, we make `Mul<Network>` - the trait for addition with a RHS of type `Network`.
// The following block implements the operation: Foo * Bar = FooBar
// This cascades Foo with Bar where in a gain lineup Foo comes before Bar
// using a device analogy -> [Foo] & [Bar] = [Foo Bar]
impl ops::Mul<Network> for Network {
    type Output = Network;

    fn mul(self, _rhs: Network) -> Network {
        println!("> Network.mul(Network) was called");

        self.cascade(&_rhs)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn cascade_2port_networks() {
        let network1 = Network::new("files/ntwk1.s2p".to_string());
        let network2 = Network::new("files/ntwk2.s2p".to_string());

        let cascaded_network = network1.cascade(&network2);

        assert_eq!(cascaded_network.f.len(), 91);
        assert!(true);
    }

    #[test]
    fn cascade_2port_networks_operator() {
        let network1 = Network::new("files/ntwk1.s2p".to_string());
        let network2 = Network::new("files/ntwk2.s2p".to_string());

        let _cascaded_network = network1 * network2;

        // let network3 = Network::new("files/ntwk3.s2p".to_string());

        // // assert_eq!(cascaded_network.comments.len(), 4);
        // // assert_eq!(cascaded_network.data_lines.len(), 91);

        // let num_data_lines = network3.s.len();
        // println!("Number of data lines in cascaded network: {}", num_data_lines);
        // // assert_eq!(num_data_lines, 42); // debug
        // for i in 0..num_data_lines {
        //     println!("Data line {}: {:?}", i + 1, _cascaded_network.s[i]);
        //     assert_eq!(_cascaded_network.s[i].frequency, network3.s[i].frequency);
        //     assert_eq!(_cascaded_network.s[i].s_ri, network3.s[i].s_ri);
        // }
    }
}
