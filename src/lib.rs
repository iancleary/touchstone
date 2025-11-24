use std::ops;
mod data_line;
mod data_pairs;
mod file_extension;
mod option_line;
mod parser;
pub mod plot;
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

#[derive(Debug, Clone)]
pub struct FrequencyRI {
    pub frequency: f64,
    pub s_ri: data_pairs::RealImaginary,
}

#[derive(Debug, Clone)]
pub struct FrequencyDB {
    pub frequency: f64,
    pub s_db: data_pairs::DecibelAngle,
}

#[derive(Debug, Clone)]
pub struct FrequencyMA {
    pub frequency: f64,
    pub s_ma: data_pairs::MagnitudeAngle,
}

impl Network {
    pub fn new(file_path: String) -> Self {
        parser::read_file(file_path)
    }

    pub fn print_summary(&self) {
        println!("Network Summary:");
        println!("Name: {}", self.name);
        println!("Rank (number of ports): {}", self.rank);
        println!("Frequency Unit: {}", self.frequency_unit);
        println!("Parameter: {}", self.parameter);
        println!("Format: {}", self.format);
        println!("Reference Impedance (Z0): {}", self.z0);
        println!("Number of Data Lines: {}", self.f.len());
        println!("Comments:");
        for comment in &self.comments {
            println!("{}", comment);
        }
    }

    pub fn f(&self) -> Vec<f64> {
        self.f.clone()
    }

    pub fn s_db(&self, j: i8, k: i8) -> Vec<FrequencyDB> {
        let mut s_db_vector: Vec<FrequencyDB> = Vec::new();
        for i in 0..self.s.len() {
            let frequency = self.s[i].frequency;
            let s_db_matrix = self.s[i].s_db;
            let s_db_value = s_db_matrix.get(j, k);
            let frequency_db = FrequencyDB {
                frequency,
                s_db: s_db_value,
            };
            s_db_vector.push(frequency_db);
        }
        s_db_vector
    }

    pub fn s_ri(&self, j: i8, k: i8) -> Vec<FrequencyRI> {
        let mut s_ri_vector: Vec<FrequencyRI> = Vec::new();
        for i in 0..self.s.len() {
            let frequency = self.s[i].frequency;
            let s_ri_matrix = self.s[i].s_ri;
            let s_ri_value = s_ri_matrix.get(j, k);
            let frequency_ri = FrequencyRI {
                frequency,
                s_ri: s_ri_value,
            };
            s_ri_vector.push(frequency_ri);
        }
        s_ri_vector
    }

    pub fn s_ma(&self, j: i8, k: i8) -> Vec<FrequencyMA> {
        let mut s_ma_vector: Vec<FrequencyMA> = Vec::new();
        for i in 0..self.s.len() {
            let frequency = self.s[i].frequency;
            let s_ma_matrix = self.s[i].s_ma;
            let s_ma_value = s_ma_matrix.get(j, k);
            let frequency_ma = FrequencyMA {
                frequency,
                s_ma: s_ma_value,
            };
            s_ma_vector.push(frequency_ma);
        }
        s_ma_vector
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
            comments,
            comments_after_option_line,
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
    fn f() {
        let network1 = Network::new("files/ntwk1.s2p".to_string());
        let f = network1.f();

        assert_eq!(f.len(), network1.f.len());
    }

    #[test]
    fn s_db() {
        let network1 = Network::new("files/ntwk1.s2p".to_string());

        let s11 = network1.s_db(1, 1);
        let s12 = network1.s_db(1, 2);
        let s21 = network1.s_db(2, 1);
        let s22 = network1.s_db(2, 2);

        assert_eq!(s11.len(), s12.len());
        assert_eq!(s11.len(), s21.len());
        assert_eq!(s11.len(), s22.len());
    }

    #[test]
    fn s_ri() {
        let network1 = Network::new("files/ntwk1.s2p".to_string());

        let s11 = network1.s_ri(1, 1);
        let s12 = network1.s_ri(1, 2);
        let s21 = network1.s_ri(2, 1);
        let s22 = network1.s_ri(2, 2);

        assert_eq!(s11.len(), s12.len());
        assert_eq!(s11.len(), s21.len());
        assert_eq!(s11.len(), s22.len());
    }

    #[test]
    fn s_ma() {
        let network1 = Network::new("files/ntwk1.s2p".to_string());

        let s11 = network1.s_ma(1, 1);
        let s12 = network1.s_ma(1, 2);
        let s21 = network1.s_ma(2, 1);
        let s22 = network1.s_ma(2, 2);

        assert_eq!(s11.len(), s12.len());
        assert_eq!(s11.len(), s21.len());
        assert_eq!(s11.len(), s22.len());
    }

    #[test]
    fn cascade_2port_networks() {
        let network1 = Network::new("files/ntwk1.s2p".to_string());
        let network2 = Network::new("files/ntwk2.s2p".to_string());

        let cascaded_network = network1.cascade(&network2);

        assert_eq!(cascaded_network.f.len(), 91);
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
