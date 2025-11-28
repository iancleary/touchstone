use std::fmt;

#[derive(Debug, Clone)]
pub struct Options {
    pub frequency_unit: String,
    pub parameter: String,
    pub format: String,
    pub resistance_string: String,    // "R"
    pub reference_resistance: String, // If "R" is not present, this is 50
}

// FROM docs/touchstone_ver2_1.pdf (Page 6)
//
// Option Line
// For Version 1.0, Version 1.1, Version 2.0 and Version 2.1 files:
// Each Touchstone data file shall contain an option line (additional option lines after the first one shall be
// ignored).  The option line is formatted as follows:

// # <frequency unit> <parameter> <format> R <n>

// where
// #   marks the beginning of the option line.

// frequency unit  specifies the unit of frequency.  Legal values are Hz, kHz, MHz, and GHz.  The
// default value is GHz.

// parameter  specifies what kind of network parameter data is contained in the file.  Legal
// values are:
// S for Scattering parameters,
// Y for Admittance parameters,
// Z for Impedance parameters,
// H for Hybrid-h parameters,
// G for Hybrid-g parameters.
// The default value is S.

// format    specifies the format of the network parameter data pairs.  Legal values are:
// DB for decibel-angle (decibel = 20 × log10|magnitude|)
// MA for magnitude-angle,
// RI for real-imaginary.
// Angles are given in degrees.  Note that this format does not apply to noise
// parameters (refer to the “Noise Parameter Data” section later in this
// specification).  The default value is MA.
//
// R n  specifies the reference resistance in ohms, where n is a real, positive number.  If
// R is omitted, the default reference resistance is 50 ohms for all ports.

// For Version 1.1 files:
// R n1 ... np specifies the reference resistances in ohms, where the character “R” is followed
// by p real, positive numbers, where p is equal to the number of ports represented
// in the file.  For Version 1.1 files, the character “R” shall be followed by as many
// values as the number of ports to serve as the per port reference resistance for
// each port, matched by order.  In addition, for Version 1.1 files, “R” and the p
// values following it shall be placed at the end of the option line.

// For Version 2.0 and Version 2.1 files:
// R n  specifies the reference resistance in ohms, where n is a real, positive number.
// For Version 2.0 and Version 2.1 files, this is overridden by the [Reference]
// keyword, if it exists, as described below.  Note that, for Version 2.0 and Version
// 2.1 files, independent references per port are only supported through the
// [Reference] keyword.

impl Options {
    pub fn default() -> Self {
        Self {
            frequency_unit: "GHz".to_string(),
            parameter: "S".to_string(),
            format: "MA".to_string(),
            resistance_string: "R".to_string(),
            reference_resistance: "50".to_string(),
        }
    }

    pub fn new(
        frequency_unit: String,
        parameter: String,
        format: String,
        resistance_string: String,
        reference_resistance: String,
    ) -> Self {
        Self {
            frequency_unit,
            parameter,
            format,
            resistance_string,
            reference_resistance,
        }
    }
}

impl fmt::Display for Options {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let line = format!(
            "# {} {} {} {} {}",
            self.frequency_unit,
            self.parameter,
            self.format,
            self.resistance_string,
            self.reference_resistance
        );
        write!(f, "{}", line)
    }
}

pub(crate) fn parse_option_line(option_line: String, options: &mut Options) {
    // println!("\n\n");
    // println!("Default options:\n{:?}", options);

    // println!("Option Line: {option_line}");
    let parts = option_line.split_whitespace().collect::<Vec<_>>();

    // println!("{}", parts.len());
    // println!("{:?}", parts);

    for option in parts {
        //
        // FROM docs/touchstone_ver2_1.pdf (Page 8)
        // Touchstone files are case-insensitive
        let lowercase_option = option.to_string().to_lowercase();

        match lowercase_option.as_str() {
            "#" => {}

            "hz" => options.frequency_unit = "Hz".to_string(),
            "khz" => options.frequency_unit = "kHz".to_string(),
            "mhz" => options.frequency_unit = "MHz".to_string(),
            "ghz" => options.frequency_unit = "GHz".to_string(),

            "s" => options.parameter = "S".to_string(),
            "y" => options.parameter = "Y".to_string(),
            "z" => options.parameter = "Z".to_string(),
            "h" => options.parameter = "H".to_string(),
            "g" => options.parameter = "G".to_string(),

            "db" => options.format = "DB".to_string(),
            "ma" => options.format = "MA".to_string(),
            "ri" => options.format = "RI".to_string(),

            "r" => options.resistance_string = "R".to_string(),

            _ => options.reference_resistance = lowercase_option.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn parse_s_ri_r_50() {
        let mut options = Options::default();
        parse_option_line("# ghz S ri R 50".to_string(), &mut options);

        assert_eq!(options.frequency_unit, "GHz");
        assert_eq!(options.parameter, "S");
        assert_eq!(options.format, "RI");
        assert_eq!(options.resistance_string, "R");
        assert_eq!(options.reference_resistance, "50");
    }

    #[test]
    fn parse_y_ri_r_50() {
        let mut options = Options::default();
        parse_option_line("# ghz Y ri R 50".to_string(), &mut options);

        assert_eq!(options.frequency_unit, "GHz");
        assert_eq!(options.parameter, "Y");
        assert_eq!(options.format, "RI");
        assert_eq!(options.resistance_string, "R");
        assert_eq!(options.reference_resistance, "50");
    }

    #[test]
    fn parse_z_ri_r_50() {
        let mut options = Options::default();
        parse_option_line("# ghz Z ri R 50".to_string(), &mut options);

        assert_eq!(options.frequency_unit, "GHz");
        assert_eq!(options.parameter, "Z");
        assert_eq!(options.format, "RI");
        assert_eq!(options.resistance_string, "R");
        assert_eq!(options.reference_resistance, "50");
    }

    #[test]
    fn parse_h_ri_r_50() {
        let mut options = Options::default();
        parse_option_line("# ghz H ri R 50".to_string(), &mut options);

        assert_eq!(options.frequency_unit, "GHz");
        assert_eq!(options.parameter, "H");
        assert_eq!(options.format, "RI");
        assert_eq!(options.resistance_string, "R");
        assert_eq!(options.reference_resistance, "50");
    }

    #[test]
    fn parse_g_ri_r_50() {
        let mut options = Options::default();
        parse_option_line("# ghz G ri R 50".to_string(), &mut options);

        assert_eq!(options.frequency_unit, "GHz");
        assert_eq!(options.parameter, "G");
        assert_eq!(options.format, "RI");
        assert_eq!(options.resistance_string, "R");
        assert_eq!(options.reference_resistance, "50");
    }

    #[test]
    fn parse_s_ri_r_100() {
        let mut options = Options::default();
        parse_option_line("# ghz S ri R 100".to_string(), &mut options);

        assert_eq!(options.frequency_unit, "GHz");
        assert_eq!(options.parameter, "S");
        assert_eq!(options.format, "RI");
        assert_eq!(options.resistance_string, "R");
        assert_eq!(options.reference_resistance, "100");
    }

    #[test]
    fn parse_s_ri() {
        let mut options = Options::default();
        parse_option_line("# ghz S ri".to_string(), &mut options);

        assert_eq!(options.frequency_unit, "GHz");
        assert_eq!(options.parameter, "S");
        assert_eq!(options.format, "RI");
        assert_eq!(options.resistance_string, "R");
        assert_eq!(options.reference_resistance, "50");
    }

    #[test]
    fn parse_s_ri_mixed_case() {
        let mut options = Options::default();
        parse_option_line("# GHZ s RI".to_string(), &mut options);

        assert_eq!(options.frequency_unit, "GHz");
        assert_eq!(options.parameter, "S");
        assert_eq!(options.format, "RI");
        assert_eq!(options.resistance_string, "R");
        assert_eq!(options.reference_resistance, "50");
    }

    #[test]
    fn parse_s_ri_r_50_mixed_case() {
        let mut options = Options::default();
        parse_option_line("# GHZ s RI r 50".to_string(), &mut options);

        assert_eq!(options.frequency_unit, "GHz");
        assert_eq!(options.parameter, "S");
        assert_eq!(options.format, "RI");
        assert_eq!(options.resistance_string, "R");
        assert_eq!(options.reference_resistance, "50");
    }

    #[test]
    fn parse_s_ri_r_50_mixed_case_out_of_order() {
        let mut options = Options::default();
        parse_option_line("# s r 50 RI GHZ".to_string(), &mut options);

        assert_eq!(options.frequency_unit, "GHz");
        assert_eq!(options.parameter, "S");
        assert_eq!(options.format, "RI");
        assert_eq!(options.resistance_string, "R");
        assert_eq!(options.reference_resistance, "50");
    }

    #[test]
    fn parse_s_ri_r_100_mixed_case_out_of_order() {
        let mut options = Options::default();
        parse_option_line("# s r 100 RI GHZ".to_string(), &mut options);

        assert_eq!(options.frequency_unit, "GHz");
        assert_eq!(options.parameter, "S");
        assert_eq!(options.format, "RI");
        assert_eq!(options.resistance_string, "R");
        assert_eq!(options.reference_resistance, "100");
    }

    #[test]
    fn parse_s_ri_mixed_case_out_of_order() {
        let mut options = Options::default();
        parse_option_line("# s RI GHZ".to_string(), &mut options);

        assert_eq!(options.frequency_unit, "GHz");
        assert_eq!(options.parameter, "S");
        assert_eq!(options.format, "RI");
        assert_eq!(options.resistance_string, "R");
        assert_eq!(options.reference_resistance, "50");
    }

    #[test]
    fn parse_y_ri_r_100_mixed_case_out_of_order() {
        let mut options = Options::default();
        parse_option_line("# Y r 100 RI GHZ".to_string(), &mut options);

        assert_eq!(options.frequency_unit, "GHz");
        assert_eq!(options.parameter, "Y");
        assert_eq!(options.format, "RI");
        assert_eq!(options.resistance_string, "R");
        assert_eq!(options.reference_resistance, "100");
    }

    #[test]
    fn parse_z_ri_r_100_mixed_case_out_of_order() {
        let mut options = Options::default();
        parse_option_line("# Y r 100 RI GHZ".to_string(), &mut options);

        assert_eq!(options.frequency_unit, "GHz");
        assert_eq!(options.parameter, "Y");
        assert_eq!(options.format, "RI");
        assert_eq!(options.resistance_string, "R");
        assert_eq!(options.reference_resistance, "100");
    }

    #[test]
    fn parse_z_ma_r_100_mixed_case_out_of_order() {
        let mut options = Options::default();
        parse_option_line("# Y r 100 MA gHz".to_string(), &mut options);

        assert_eq!(options.frequency_unit, "GHz");
        assert_eq!(options.parameter, "Y");
        assert_eq!(options.format, "MA");
        assert_eq!(options.resistance_string, "R");
        assert_eq!(options.reference_resistance, "100");
    }

    #[test]
    fn parse_z_ma_r_100_mixed_case_out_of_order2() {
        let mut options = Options::default();
        parse_option_line("# Y r 100 ma GHZ".to_string(), &mut options);

        assert_eq!(options.frequency_unit, "GHz");
        assert_eq!(options.parameter, "Y");
        assert_eq!(options.format, "MA");
        assert_eq!(options.resistance_string, "R");
        assert_eq!(options.reference_resistance, "100");
    }

    #[test]
    fn parse_z_db_r_100_mixed_case_out_of_order() {
        let mut options = Options::default();
        parse_option_line("# Y r 100 DB GHZ".to_string(), &mut options);

        assert_eq!(options.frequency_unit, "GHz");
        assert_eq!(options.parameter, "Y");
        assert_eq!(options.format, "DB");
        assert_eq!(options.resistance_string, "R");
        assert_eq!(options.reference_resistance, "100");
    }

    #[test]
    fn to_string() {
        let options = Options::default();
        assert_eq!(options.to_string(), "# GHz S MA R 50");
    }
}
