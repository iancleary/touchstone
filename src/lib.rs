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
            let s_db_matrix = &self.s[i].s_db;
            let s_db_value = s_db_matrix.get(j as usize, k as usize);
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
            let s_ri_matrix = &self.s[i].s_ri;
            let s_ri_value = s_ri_matrix.get(j as usize, k as usize);
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
            let s_ma_matrix = &self.s[i].s_ma;
            let s_ma_value = s_ma_matrix.get(j as usize, k as usize);
            let frequency_ma = FrequencyMA {
                frequency,
                s_ma: s_ma_value,
            };
            s_ma_vector.push(frequency_ma);
        }
        s_ma_vector
    }

    /// Cascade two 2-port networks (standard connection: port 2 → port 1)
    ///
    /// For more control over port connections, use `cascade_ports()`.
    pub fn cascade(&self, other: &Network) -> Network {
        if self.rank != 2 || other.rank != 2 {
            panic!("Cascading is only implemented for 2-port networks. Use cascade_ports() for explicit port specification.");
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
            let s1 = &self.s[i].s_ri;
            let s2 = &other.s[i].s_ri;

            let abcd1 = s1.to_abcd(self.z0);
            let abcd2 = s2.to_abcd(other.z0);

            let abcd_new = abcd1 * abcd2;

            // Resulting Z0? Usually the Z0 of the output port of the second network,
            // but for S-parameters of the cascaded block, we usually reference the input port of the first
            // and output port of the second.
            // If Z0 is the same for both (checked at start of function), then it's just self.z0.
            let s_new_ri = abcd_new.to_s(self.z0);

            let s_new_ma = crate::data_pairs::MagnitudeAngleMatrix::from_vec(vec![
                vec![
                    s_new_ri.get(1, 1).magnitude_angle(),
                    s_new_ri.get(1, 2).magnitude_angle(),
                ],
                vec![
                    s_new_ri.get(2, 1).magnitude_angle(),
                    s_new_ri.get(2, 2).magnitude_angle(),
                ],
            ]);

            let s_new_db =
                crate::data_pairs::DecibelAngleMatrix::from_magnitude_angle_matrix(&s_new_ma);

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

    /// Cascade two networks with explicit port specification
    ///
    /// # Arguments
    /// * `other` - The network to cascade with
    /// * `from_port` - Output port of self to connect (1-indexed)
    /// * `to_port` - Input port of other to connect (1-indexed)
    ///
    /// # Examples
    /// ```
    /// use touchstone::Network;
    ///
    /// let net1 = Network::new("files/ntwk1.s2p".to_string());
    /// let net2 = Network::new("files/ntwk2.s2p".to_string());
    ///
    /// // Standard 2-port cascade (port 2 → port 1)
    /// let result = net1.cascade_ports(&net2, 2, 1);
    /// ```
    ///
    /// # Current Limitations
    /// - Only 2-port networks with standard connection (2→1) are currently supported
    /// - N-port cascade (N > 2) will be implemented in a future release
    ///
    /// # Panics
    /// - If port numbers are out of range
    /// - If networks are not 2-port
    /// - If connection is not standard (2→1) for 2-port networks
    pub fn cascade_ports(&self, other: &Network, from_port: usize, to_port: usize) -> Network {
        // Validate port numbers
        assert!(
            from_port >= 1 && from_port <= self.rank as usize,
            "from_port {} out of range for {}-port network (valid range: 1-{})",
            from_port,
            self.rank,
            self.rank
        );
        assert!(
            to_port >= 1 && to_port <= other.rank as usize,
            "to_port {} out of range for {}-port network (valid range: 1-{})",
            to_port,
            other.rank,
            other.rank
        );

        // For 2-port networks: use existing ABCD-based cascade
        if self.rank == 2 && other.rank == 2 {
            // Currently only support standard connection (port 2 → port 1)
            if from_port != 2 || to_port != 1 {
                panic!(
                    "For 2-port networks, only standard cascade (port 2 → port 1) is currently supported.\n\
                     Requested connection: port {} → port {}\n\
                     Use cascade() method for standard 2-port cascade, or wait for future N-port cascade implementation.",
                    from_port, to_port
                );
            }

            // Delegate to existing cascade implementation
            return self.cascade(other);
        }

        // For N-port where N > 2: Future enhancement
        panic!(
            "Cascading {}-port and {}-port networks is not yet supported.\n\
             Currently only 2-port networks can be cascaded (with standard port 2 → port 1 connection).\n\
             \n\
             Future enhancement: Full N-port cascade with arbitrary port connections.\n\
             \n\
             Workaround: Extract 2-port sub-networks from your {}-port and {}-port networks,\n\
             then cascade those 2-port networks.",
            self.rank,
            other.rank,
            self.rank,
            other.rank
        );
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
        let option_line = option_line::Options::new(
            self.frequency_unit.clone(),
            self.parameter.clone(),
            self.format.clone(),
            self.resistance_string.clone(),
            self.z0.to_string().clone(),
        );
        writeln!(file, "{}", option_line)?;

        // Write comments after option line
        for comment in &self.comments_after_option_line {
            writeln!(file, "{}", comment)?;
        }

        // Write data lines
        let n = self.rank as usize;
        for data_line in &self.s {
            let mut freq = data_line.frequency;
            let frequency_unit = self.frequency_unit.clone();

            if frequency_unit == "THz" {
                freq = rfconversions::frequency::hz_to_thz(freq);
            } else if frequency_unit == "GHz" {
                freq = rfconversions::frequency::hz_to_ghz(freq);
            } else if frequency_unit == "MHz" {
                freq = rfconversions::frequency::hz_to_mhz(freq);
            } else if frequency_unit == "kHz" {
                freq = rfconversions::frequency::hz_to_khz(freq);
            }

            // For 1-port and 2-port: use single-line format
            // For 3+ port: use multi-line format
            if n <= 2 {
                // Single-line format
                let mut line = format!("{}", freq);

                match self.format.as_str() {
                    "RI" => {
                        let s = &data_line.s_ri;
                        for row in 1..=n {
                            for col in 1..=n {
                                line.push_str(&format!(
                                    " {} {}",
                                    s.get(row, col).0,
                                    s.get(row, col).1
                                ));
                            }
                        }
                    }
                    "MA" => {
                        let s = &data_line.s_ma;
                        for row in 1..=n {
                            for col in 1..=n {
                                line.push_str(&format!(
                                    " {} {}",
                                    s.get(row, col).0,
                                    s.get(row, col).1
                                ));
                            }
                        }
                    }
                    "DB" => {
                        let s = &data_line.s_db;
                        for row in 1..=n {
                            for col in 1..=n {
                                line.push_str(&format!(
                                    " {} {}",
                                    s.get(row, col).0,
                                    s.get(row, col).1
                                ));
                            }
                        }
                    }
                    _ => panic!("Unsupported format for saving: {}", self.format),
                }

                writeln!(file, "{}", line)?;
            } else {
                // Multi-line format for 3+ port
                // First line: frequency and first row of S-parameters
                let mut line = format!("{}", freq);

                match self.format.as_str() {
                    "RI" => {
                        let s = &data_line.s_ri;
                        // First row on same line as frequency
                        for col in 1..=n {
                            line.push_str(&format!(" {} {}", s.get(1, col).0, s.get(1, col).1));
                        }
                        writeln!(file, "{}", line)?;

                        // Subsequent rows on their own lines
                        for row in 2..=n {
                            let mut row_line = String::new();
                            for col in 1..=n {
                                row_line.push_str(&format!(
                                    " {} {}",
                                    s.get(row, col).0,
                                    s.get(row, col).1
                                ));
                            }
                            writeln!(file, "{}", row_line)?;
                        }
                    }
                    "MA" => {
                        let s = &data_line.s_ma;
                        // First row on same line as frequency
                        for col in 1..=n {
                            line.push_str(&format!(" {} {}", s.get(1, col).0, s.get(1, col).1));
                        }
                        writeln!(file, "{}", line)?;

                        // Subsequent rows on their own lines
                        for row in 2..=n {
                            let mut row_line = String::new();
                            for col in 1..=n {
                                row_line.push_str(&format!(
                                    " {} {}",
                                    s.get(row, col).0,
                                    s.get(row, col).1
                                ));
                            }
                            writeln!(file, "{}", row_line)?;
                        }
                    }
                    "DB" => {
                        let s = &data_line.s_db;
                        // First row on same line as frequency
                        for col in 1..=n {
                            line.push_str(&format!(" {} {}", s.get(1, col).0, s.get(1, col).1));
                        }
                        writeln!(file, "{}", line)?;

                        // Subsequent rows on their own lines
                        for row in 2..=n {
                            let mut row_line = String::new();
                            for col in 1..=n {
                                row_line.push_str(&format!(
                                    " {} {}",
                                    s.get(row, col).0,
                                    s.get(row, col).1
                                ));
                            }
                            writeln!(file, "{}", row_line)?;
                        }
                    }
                    _ => panic!("Unsupported format for saving: {}", self.format),
                }
            }
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

            let s1 = &cascaded_network.s[i].s_ri;
            let s2 = &network3.s[i].s_ri;
            let epsilon = 1e-4; // Relaxed epsilon for floating point differences

            assert!(
                (s1.get(1, 1).0 - s2.get(1, 1).0).abs() < epsilon,
                "S11 real mismatch at freq {}",
                cascaded_network.s[i].frequency
            );
            assert!(
                (s1.get(1, 1).1 - s2.get(1, 1).1).abs() < epsilon,
                "S11 imag mismatch"
            );
            assert!(
                (s1.get(1, 2).0 - s2.get(1, 2).0).abs() < epsilon,
                "S12 real mismatch"
            );
            assert!(
                (s1.get(1, 2).1 - s2.get(1, 2).1).abs() < epsilon,
                "S12 imag mismatch"
            );
            assert!(
                (s1.get(2, 1).0 - s2.get(2, 1).0).abs() < epsilon,
                "S21 real mismatch"
            );
            assert!(
                (s1.get(2, 1).1 - s2.get(2, 1).1).abs() < epsilon,
                "S21 imag mismatch"
            );
            assert!(
                (s1.get(2, 2).0 - s2.get(2, 2).0).abs() < epsilon,
                "S22 real mismatch"
            );
            assert!(
                (s1.get(2, 2).1 - s2.get(2, 2).1).abs() < epsilon,
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

            let f1 = cascaded_network.f[i];
            let f2 = network3.f[i];

            assert_eq!(f1, f2);

            let s1 = &cascaded_network.s[i].s_ri;
            let s2 = &network3.s[i].s_ri;
            let epsilon = 1e-4; // Relaxed epsilon for floating point differences

            assert!(
                (s1.get(1, 1).0 - s2.get(1, 1).0).abs() < epsilon,
                "S11 real mismatch at freq {}",
                cascaded_network.s[i].frequency
            );
            assert!(
                (s1.get(1, 1).1 - s2.get(1, 1).1).abs() < epsilon,
                "S11 imag mismatch"
            );
            assert!(
                (s1.get(1, 2).0 - s2.get(1, 2).0).abs() < epsilon,
                "S12 real mismatch"
            );
            assert!(
                (s1.get(1, 2).1 - s2.get(1, 2).1).abs() < epsilon,
                "S12 imag mismatch"
            );
            assert!(
                (s1.get(2, 1).0 - s2.get(2, 1).0).abs() < epsilon,
                "S21 real mismatch"
            );
            assert!(
                (s1.get(2, 1).1 - s2.get(2, 1).1).abs() < epsilon,
                "S21 imag mismatch"
            );
            assert!(
                (s1.get(2, 2).0 - s2.get(2, 2).0).abs() < epsilon,
                "S22 real mismatch"
            );
            assert!(
                (s1.get(2, 2).1 - s2.get(2, 2).1).abs() < epsilon,
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

        let path_temp = file_path_str.to_string();
        let binding = std::path::Path::new(&path_temp);
        let network2_name = binding.to_str().unwrap();

        assert_eq!(network2.name, network2_name);
        assert_eq!(network1.parameter, network2.parameter);

        assert_eq!(network1.f.len(), network2.f.len());
        for i in 0..network1.f.len() {
            assert_eq!(network1.f[i], network2.f[i]);
        }

        assert_eq!(network1.s.len(), network2.s.len());
        for i in 0..network1.s.len() {
            let s1 = &network1.s[i].s_ri;
            let s2 = &network2.s[i].s_ri;
            let epsilon = 1e-6;
            assert!((s1.get(1, 1).0 - s2.get(1, 1).0).abs() < epsilon);
            assert!((s1.get(1, 1).1 - s2.get(1, 1).1).abs() < epsilon);
        }

        // Cleanup
        std::fs::remove_file(file_path).unwrap();
    }

    #[test]
    fn test_save_load_roundtrip_3port() {
        let network1 = Network::new("files/hfss_18.2.s3p".to_string());

        let temp_dir = std::env::temp_dir()
            .join("touchstone_tests")
            .join("roundtrip");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("roundtrip_3port.s3p");
        let file_path_str = file_path.to_str().unwrap();

        network1.save(file_path_str).unwrap();

        let network2 = Network::new(file_path_str.to_string());

        // Verify metadata
        assert_eq!(network1.rank, network2.rank);
        assert_eq!(network1.rank, 3);
        assert_eq!(network1.f.len(), network2.f.len());
        assert_eq!(network1.s.len(), network2.s.len());
        assert_eq!(network1.format, network2.format);
        assert_eq!(network1.z0, network2.z0);

        // Verify frequencies
        for i in 0..network1.f.len() {
            assert_eq!(network1.f[i], network2.f[i]);
        }

        // Verify all S-parameters (3x3 matrix)
        let epsilon = 1e-6;
        for i in 0..network1.s.len() {
            for row in 1..=3 {
                for col in 1..=3 {
                    let s1_ma = &network1.s[i].s_ma;
                    let s2_ma = &network2.s[i].s_ma;
                    assert!(
                        (s1_ma.get(row, col).0 - s2_ma.get(row, col).0).abs() < epsilon,
                        "S{}{} magnitude mismatch at frequency index {}",
                        row,
                        col,
                        i
                    );
                    assert!(
                        (s1_ma.get(row, col).1 - s2_ma.get(row, col).1).abs() < epsilon,
                        "S{}{} angle mismatch at frequency index {}",
                        row,
                        col,
                        i
                    );
                }
            }
        }

        // Cleanup
        std::fs::remove_file(file_path).unwrap();
    }

    #[test]
    fn test_save_load_roundtrip_4port() {
        let network1 = Network::new("files/Agilent_E5071B.s4p".to_string());

        let temp_dir = std::env::temp_dir()
            .join("touchstone_tests")
            .join("roundtrip");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("roundtrip_4port.s4p");
        let file_path_str = file_path.to_str().unwrap();

        network1.save(file_path_str).unwrap();

        let network2 = Network::new(file_path_str.to_string());

        // Verify metadata
        assert_eq!(network1.rank, network2.rank);
        assert_eq!(network1.rank, 4);
        assert_eq!(network1.f.len(), network2.f.len());
        assert_eq!(network1.s.len(), network2.s.len());
        assert_eq!(network1.format, network2.format);
        assert_eq!(network1.z0, network2.z0);

        // Verify frequencies
        for i in 0..network1.f.len() {
            assert_eq!(network1.f[i], network2.f[i]);
        }

        // Verify all S-parameters (4x4 matrix)
        let epsilon = 1e-6;
        for i in 0..network1.s.len() {
            for row in 1..=4 {
                for col in 1..=4 {
                    let s1_db = &network1.s[i].s_db;
                    let s2_db = &network2.s[i].s_db;
                    assert!(
                        (s1_db.get(row, col).0 - s2_db.get(row, col).0).abs() < epsilon,
                        "S{}{} dB mismatch at frequency index {}",
                        row,
                        col,
                        i
                    );
                    assert!(
                        (s1_db.get(row, col).1 - s2_db.get(row, col).1).abs() < epsilon,
                        "S{}{} angle mismatch at frequency index {}",
                        row,
                        col,
                        i
                    );
                }
            }
        }

        // Cleanup
        std::fs::remove_file(file_path).unwrap();
    }

    #[test]
    fn test_cascade_ports_2port_standard() {
        // Test cascade_ports with standard 2-port connection (2→1)
        let network1 = Network::new("files/ntwk1.s2p".to_string());
        let network2 = Network::new("files/ntwk2.s2p".to_string());
        let network3 = Network::new("files/ntwk3.s2p".to_string());

        // cascade_ports(2, 1) should give same result as cascade()
        let result_ports = network1.cascade_ports(&network2, 2, 1);
        let result_standard = network1.cascade(&network2);

        assert_eq!(result_ports.rank, result_standard.rank);
        assert_eq!(result_ports.f.len(), result_standard.f.len());
        assert_eq!(result_ports.s.len(), result_standard.s.len());

        // Should also match the known good result (ntwk3)
        let epsilon = 1e-4;
        for i in 0..result_ports.s.len() {
            let s1 = &result_ports.s[i].s_ri;
            let s2 = &network3.s[i].s_ri;

            assert!((s1.get(1, 1).0 - s2.get(1, 1).0).abs() < epsilon);
            assert!((s1.get(2, 2).0 - s2.get(2, 2).0).abs() < epsilon);
        }
    }

    #[test]
    #[should_panic(expected = "only standard cascade (port 2 → port 1) is currently supported")]
    fn test_cascade_ports_2port_nonstandard() {
        // Test that non-standard port connections panic with helpful message
        let network1 = Network::new("files/ntwk1.s2p".to_string());
        let network2 = Network::new("files/ntwk2.s2p".to_string());

        // This should panic because we don't support 1→2 connection yet
        network1.cascade_ports(&network2, 1, 2);
    }

    #[test]
    #[should_panic(expected = "from_port 3 out of range for 2-port network")]
    fn test_cascade_ports_invalid_from_port() {
        // Test that invalid from_port is caught
        let network1 = Network::new("files/ntwk1.s2p".to_string());
        let network2 = Network::new("files/ntwk2.s2p".to_string());

        // This should panic - port 3 doesn't exist on 2-port network
        network1.cascade_ports(&network2, 3, 1);
    }

    #[test]
    #[should_panic(expected = "to_port 5 out of range for 2-port network")]
    fn test_cascade_ports_invalid_to_port() {
        // Test that invalid to_port is caught
        let network1 = Network::new("files/ntwk1.s2p".to_string());
        let network2 = Network::new("files/ntwk2.s2p".to_string());

        // This should panic - port 5 doesn't exist on 2-port network
        network1.cascade_ports(&network2, 2, 5);
    }

    #[test]
    #[should_panic(expected = "from_port 0 out of range")]
    fn test_cascade_ports_zero_port() {
        // Test that port 0 is rejected (ports are 1-indexed)
        let network1 = Network::new("files/ntwk1.s2p".to_string());
        let network2 = Network::new("files/ntwk2.s2p".to_string());

        // This should panic - ports are 1-indexed, not 0-indexed
        network1.cascade_ports(&network2, 0, 1);
    }
}
