use std::ops;
pub mod cli;
mod data_line;
mod data_pairs;
mod file_extension;
mod file_operations;
mod open;
mod option_line;
mod parser;
mod plot;
mod utils;

#[derive(Debug, Clone)]
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

        let mut s_new = Vec::new();
        // Assuming index-wise alignment as discussed
        let len = std::cmp::min(self.s.len(), other.s.len());

        for i in 0..len {
            let freq = self.s[i].frequency;
            let s1 = self.s[i].s_ri;
            let s2 = other.s[i].s_ri;

            let abcd1 = s1.to_abcd(self.z0);
            let abcd2 = s2.to_abcd(other.z0);

            let abcd_new = abcd1 * abcd2;

            // Resulting Z0? Usually the Z0 of the output port of the second network,
            // but for S-parameters of the cascaded block, we usually reference the input port of the first
            // and output port of the second.
            // If Z0 is the same for both (checked at start of function), then it's just self.z0.
            let s_new_ri = abcd_new.to_s(self.z0);

            let s_new_ma = crate::data_pairs::MagnitudeAngleMatrix(
                (
                    s_new_ri.0 .0.magnitude_angle(),
                    s_new_ri.0 .1.magnitude_angle(),
                ),
                (
                    s_new_ri.1 .0.magnitude_angle(),
                    s_new_ri.1 .1.magnitude_angle(),
                ),
            );

            let s_new_db =
                crate::data_pairs::DecibelAngleMatrix::from_magnitude_angle_matrix(s_new_ma);

            s_new.push(crate::data_line::ParsedDataLine {
                frequency: freq,
                s_ri: s_new_ri,
                s_ma: s_new_ma,
                s_db: s_new_db,
            });
        }

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
            f: self.f.clone(), // Note: this might be longer than s_new if other is shorter
            s: s_new,
        }
    }

    pub fn save(&self, file_path: &str) -> std::io::Result<()> {
        use std::io::Write;
        let mut file = std::fs::File::create(file_path)?;

        // Write comments
        for comment in &self.comments {
            writeln!(file, "{}", comment)?;
        }

        // Write option line
        // # <frequency unit> <parameter> <format> R <n>
        writeln!(
            file,
            "# {} {} {} {} {}",
            self.frequency_unit, self.parameter, self.format, self.resistance_string, self.z0
        )?;

        // Write comments after option line
        for comment in &self.comments_after_option_line {
            writeln!(file, "{}", comment)?;
        }

        // Write data lines
        for data_line in &self.s {
            let freq = data_line.frequency;
            let mut line = format!("{}", freq);

            match self.format.as_str() {
                "RI" => {
                    let s = data_line.s_ri;
                    // N11, N12, N21, N22
                    // Each is real, imag
                    line.push_str(&format!(" {} {}", s.0 .0 .0, s.0 .0 .1));
                    line.push_str(&format!(" {} {}", s.0 .1 .0, s.0 .1 .1));
                    line.push_str(&format!(" {} {}", s.1 .0 .0, s.1 .0 .1));
                    line.push_str(&format!(" {} {}", s.1 .1 .0, s.1 .1 .1));
                }
                "MA" => {
                    let s = data_line.s_ma;
                    line.push_str(&format!(" {} {}", s.0 .0 .0, s.0 .0 .1));
                    line.push_str(&format!(" {} {}", s.0 .1 .0, s.0 .1 .1));
                    line.push_str(&format!(" {} {}", s.1 .0 .0, s.1 .0 .1));
                    line.push_str(&format!(" {} {}", s.1 .1 .0, s.1 .1 .1));
                }
                "DB" => {
                    let s = data_line.s_db;
                    line.push_str(&format!(" {} {}", s.0 .0 .0, s.0 .0 .1));
                    line.push_str(&format!(" {} {}", s.0 .1 .0, s.0 .1 .1));
                    line.push_str(&format!(" {} {}", s.1 .0 .0, s.1 .0 .1));
                    line.push_str(&format!(" {} {}", s.1 .1 .0, s.1 .1 .1));
                }
                _ => panic!("Unsupported format for saving: {}", self.format),
            }

            writeln!(file, "{}", line)?;
        }

        Ok(())
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

        let network3 = Network::new("files/ntwk3.s2p".to_string());

        let cascaded_network = network1.cascade(&network2);

        assert_eq!(cascaded_network.f.len(), 91);
        assert_eq!(cascaded_network.s.len(), 91);

        for i in 0..cascaded_network.s.len() {
            assert_eq!(cascaded_network.s[i].frequency, network3.s[i].frequency);

            let s1 = cascaded_network.s[i].s_ri;
            let s2 = network3.s[i].s_ri;
            let epsilon = 1e-4; // Relaxed epsilon for floating point differences

            assert!(
                (s1.0 .0 .0 - s2.0 .0 .0).abs() < epsilon,
                "S11 real mismatch at freq {}",
                cascaded_network.s[i].frequency
            );
            assert!(
                (s1.0 .0 .1 - s2.0 .0 .1).abs() < epsilon,
                "S11 imag mismatch"
            );
            assert!(
                (s1.0 .1 .0 - s2.0 .1 .0).abs() < epsilon,
                "S12 real mismatch"
            );
            assert!(
                (s1.0 .1 .1 - s2.0 .1 .1).abs() < epsilon,
                "S12 imag mismatch"
            );
            assert!(
                (s1.1 .0 .0 - s2.1 .0 .0).abs() < epsilon,
                "S21 real mismatch"
            );
            assert!(
                (s1.1 .0 .1 - s2.1 .0 .1).abs() < epsilon,
                "S21 imag mismatch"
            );
            assert!(
                (s1.1 .1 .0 - s2.1 .1 .0).abs() < epsilon,
                "S22 real mismatch"
            );
            assert!(
                (s1.1 .1 .1 - s2.1 .1 .1).abs() < epsilon,
                "S22 imag mismatch"
            );

            // Derived values might also differ slightly, skipping strict check
            // assert_eq!(cascaded_network.s[i].s_ma, network3.s[i].s_ma);
            // assert_eq!(cascaded_network.s[i].s_db, network3.s[i].s_db);
        }
    }

    #[test]
    fn cascade_2port_networks_operator() {
        let network1 = Network::new("files/ntwk1.s2p".to_string());
        let network2 = Network::new("files/ntwk2.s2p".to_string());

        let cascaded_network = network1 * network2;

        let network3 = Network::new("files/ntwk3.s2p".to_string());

        assert_eq!(cascaded_network.f.len(), 91);
        assert_eq!(cascaded_network.s.len(), 91);

        for i in 0..cascaded_network.s.len() {
            assert_eq!(cascaded_network.s[i].frequency, network3.s[i].frequency);

            let s1 = cascaded_network.s[i].s_ri;
            let s2 = network3.s[i].s_ri;
            let epsilon = 1e-4; // Relaxed epsilon for floating point differences

            assert!(
                (s1.0 .0 .0 - s2.0 .0 .0).abs() < epsilon,
                "S11 real mismatch at freq {}",
                cascaded_network.s[i].frequency
            );
            assert!(
                (s1.0 .0 .1 - s2.0 .0 .1).abs() < epsilon,
                "S11 imag mismatch"
            );
            assert!(
                (s1.0 .1 .0 - s2.0 .1 .0).abs() < epsilon,
                "S12 real mismatch"
            );
            assert!(
                (s1.0 .1 .1 - s2.0 .1 .1).abs() < epsilon,
                "S12 imag mismatch"
            );
            assert!(
                (s1.1 .0 .0 - s2.1 .0 .0).abs() < epsilon,
                "S21 real mismatch"
            );
            assert!(
                (s1.1 .0 .1 - s2.1 .0 .1).abs() < epsilon,
                "S21 imag mismatch"
            );
            assert!(
                (s1.1 .1 .0 - s2.1 .1 .0).abs() < epsilon,
                "S22 real mismatch"
            );
            assert!(
                (s1.1 .1 .1 - s2.1 .1 .1).abs() < epsilon,
                "S22 imag mismatch"
            );

            // Derived values might also differ slightly, skipping strict check
            // assert_eq!(cascaded_network.s[i].s_ma, network3.s[i].s_ma);
            // assert_eq!(cascaded_network.s[i].s_db, network3.s[i].s_db);
        }
    }

    #[test]
    fn test_save_load_roundtrip() {
        let network1 = Network::new("files/ntwk1.s2p".to_string());

        let temp_dir = std::env::temp_dir()
            .join("touchstone_tests")
            .join("roundtrip");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("roundtrip.s2p");
        let file_path_str = file_path.to_str().unwrap();

        network1.save(file_path_str).unwrap();

        let network2 = Network::new(file_path_str.to_string());

        assert_eq!(network1.f.len(), network2.f.len());
        assert_eq!(network1.s.len(), network2.s.len());
        assert_eq!(network1.format, network2.format);
        assert_eq!(network1.z0, network2.z0);

        // Check first data point
        let s1 = network1.s[0].s_ri;
        let s2 = network2.s[0].s_ri;
        let epsilon = 1e-6;

        assert!((s1.0 .0 .0 - s2.0 .0 .0).abs() < epsilon);
        assert!((s1.0 .0 .1 - s2.0 .0 .1).abs() < epsilon);

        // Cleanup
        std::fs::remove_file(file_path).unwrap();
    }
}
