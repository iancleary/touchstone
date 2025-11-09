// FROM docs/touchstone_ver2_1.pdf (Page 6)
//
// format    specifies the format of the network parameter data pairs.  Legal values are:
// DB for decibel-angle (decibel = 20 × log10|magnitude|)
// MA for magnitude-angle,
// RI for real-imaginary.
// Angles are given in degrees.  Note that this format does not apply to noise
// parameters (refer to the “Noise Parameter Data” section later in this
// specification).  The default value is MA.

#[derive(Clone, Copy, Debug)]
pub(super) struct MagnitudeAngle(pub f32, pub f32);

#[derive(Clone, Copy, Debug)]
pub(crate) struct RealImaginary(pub f32, pub f32);

#[derive(Clone, Copy, Debug)]
pub(crate) struct DecibelAngle(pub f32, pub f32);
// As specified, this is dB20, not dB10

impl RealImaginary {
    pub fn real(self) -> f32 {
        self.0
    }
    pub fn imaginary(self) -> f32 {
        self.1
    }

    pub fn magnitude(self) -> f32 {
        (f32::powf(self.0, 2.0) + f32::powf(self.1, 2.0)).sqrt()
    }

    pub fn decibel(self) -> f32 {
        // need to resolve file from 2010 on if this should be 20
        // TODO: I think the s2p file is old/bad in this regard
        // (look into this after I sleep and think about it)
        20.0 * (f32::powf(self.0, 2.0) + f32::powf(self.1, 2.0))
            .sqrt()
            .log10()
    }

    pub fn angle(self) -> f32 {
        (self.1 / self.0).atan()
    }
}

impl MagnitudeAngle {
    pub fn real(self) -> f32 {
        self.0 * self.1.cos()
    }

    pub fn real_imaginary(self) -> RealImaginary {
        RealImaginary(self.0 * self.1.cos(), self.0 * self.1.sin())
    }

    pub fn from_real_imaginary(ri: RealImaginary) -> Self {
        MagnitudeAngle(ri.magnitude(), ri.angle())
    }

    pub fn decibel(self) -> f32 {
        self.0.log10() * 20.0
    }

    pub fn magnitude(self) -> f32 {
        self.0
    }

    pub fn angle(self) -> f32 {
        self.1
    }
}

impl DecibelAngle {
    pub fn magnitude(self) -> f32 {
        10f32.powf(self.0 / 20.0)
    }

    pub fn angle(self) -> f32 {
        self.1
    }

    pub fn real_imaginary(self) -> RealImaginary {
        RealImaginary(
            self.magnitude() * self.angle().cos(),
            self.magnitude() * self.angle().sin(),
        )
    }

    pub fn real(self) -> f32 {
        self.magnitude() * self.angle().cos()
    }

    pub fn imaginary(self) -> f32 {
        self.magnitude() * self.angle().sin()
    }
}

#[derive(Debug)]
pub(crate) struct RealImaginaryMatrix(
    pub (RealImaginary, RealImaginary),
    pub (RealImaginary, RealImaginary),
);

#[derive(Debug)]
pub(crate) struct MagnitudeAngleMatrix(
    pub (MagnitudeAngle, MagnitudeAngle),
    pub (MagnitudeAngle, MagnitudeAngle),
);

#[derive(Debug)]
pub(crate) struct DecibelAngleMatrix(
    pub (DecibelAngle, DecibelAngle),
    pub (DecibelAngle, DecibelAngle),
);

fn str_to_f32(x: &str) -> f32 {
    x.parse::<f32>().expect("Failed to parse {x} into f32")
}

pub(crate) struct ParsedDataLine {
    frequency: String,
    s_ri: RealImaginaryMatrix,
    s_db: DecibelAngleMatrix,
    s_ma: MagnitudeAngleMatrix,
}

pub(crate) fn parse_data_line(data_line: String, format: &String, n: &i32) -> ParsedDataLine {
    println!("\n");
    println!("format:\n{:?}", *format);
    println!("n (number of ports): {:?}", *n);
    
    // FROM docs/touchstone_ver2_1.pdf (Page 16)
    //
    // 2-port data (line)
    // <frequency value>  <N11> <N12> <N21> <N22>

    // where
    // frequency value  frequency at which the network parameter data was taken or derived.

    // N11, N12, N21, N22   network parameter data points, where Nij represents a pair of data values
    //
    // for the network parameter from port i to port j. Each Nij consists of two numeric
    // values, whose meaning is determined by the format option specified in the option line.
    // therefore, the total number of numeric values on a 2-port data line is 1 + (2 × (2^2)) = 9.
    // generally, for an n-port data line, the total number of numeric values is 1 + (2 × (n^2)).
    let expect_number_of_parts = 1 + (2 * (n * n));
    println!("expected number of parts: {:?}", expect_number_of_parts);

    println!("Data Line: {data_line}");
    let parts = data_line.split_whitespace().collect::<Vec<_>>();

    let len_parts = parts.len();
    println!("Data Line Parts (len {}): {:?}", len_parts, parts);

    if len_parts != expect_number_of_parts as usize {
        panic!(
            "Data line has unexpected number of parts. Expected {}, got {}",
            expect_number_of_parts, len_parts
        );
    }

    // split into f32 parts, after checking the expected length
    let f32_parts: Vec<_> = parts.clone().into_iter().map(str_to_f32).collect();

    // println!("{}", len_parts);
    // println!("f32_parts (len {}): {:?}", len_parts, f32_parts);

    if n != &2 {
        panic!("Only 2-port data lines are currently supported. Found {}-port data line.", n);
    }

    let frequency = parts[0];
    if format == "RI" {
        // Real-Imaginary format
        let s_ri = RealImaginaryMatrix(
            (
                RealImaginary(f32_parts[1], f32_parts[2]),
                RealImaginary(f32_parts[3], f32_parts[4]),
            ),
            (
                RealImaginary(f32_parts[5], f32_parts[6]),
                RealImaginary(f32_parts[7], f32_parts[8]),
            ),
        );

        let s_db = DecibelAngleMatrix(
            (
                DecibelAngle(s_ri.0 .0.decibel(), s_ri.0 .0.angle()),
                DecibelAngle(s_ri.0 .1.decibel(), s_ri.0 .1.angle()),
            ),
            (
                DecibelAngle(s_ri.1 .0.decibel(), s_ri.1 .0.angle()),
                DecibelAngle(s_ri.1 .1.decibel(), s_ri.1 .1.angle()),
            ),
        );

        let s_ma = MagnitudeAngleMatrix(
            (
                MagnitudeAngle(s_ri.0 .0.magnitude(), s_ri.0 .0.angle()),
                MagnitudeAngle(s_ri.0 .1.magnitude(), s_ri.0 .1.angle()),
            ),
            (
                MagnitudeAngle(s_ri.1 .0.magnitude(), s_ri.1 .0.angle()),
                MagnitudeAngle(s_ri.1 .1.magnitude(), s_ri.1 .1.angle()),
            ),
        );

        return ParsedDataLine {
            frequency: frequency.to_string(),
            s_ri,
            s_db,
            s_ma,
        };
    } else if format == "MA" {
        // Magnitude-Angle format
        let s_ma = MagnitudeAngleMatrix(
            (
                MagnitudeAngle(f32_parts[1], f32_parts[2]),
                MagnitudeAngle(f32_parts[3], f32_parts[4]),
            ),
            (
                MagnitudeAngle(f32_parts[5], f32_parts[6]),
                MagnitudeAngle(f32_parts[7], f32_parts[8]),
            ),
        );

        let s_ri = RealImaginaryMatrix(
            (
                RealImaginary(
                    s_ma.0 .0 .0 * s_ma.0 .0 .0.cos(),
                    s_ma.0 .0 .0 * s_ma.0 .0 .0.sin(),
                ),
                RealImaginary(
                    s_ma.0 .1 .0 * s_ma.0 .1 .0.cos(),
                    s_ma.0 .1 .0 * s_ma.0 .1 .0.sin(),
                ),
            ),
            (
                RealImaginary(
                    s_ma.1 .0 .0 * s_ma.1 .0 .0.cos(),
                    s_ma.1 .0 .0 * s_ma.1 .0 .0.sin(),
                ),
                RealImaginary(
                    s_ma.1 .1 .0 * s_ma.1 .1 .0.cos(),
                    s_ma.1 .1 .0 * s_ma.1 .1 .0.sin(),
                ),
            ),
        );

        let s_db = DecibelAngleMatrix(
            (
                DecibelAngle(s_ma.0 .0.decibel(), s_ma.0 .0.angle()),
                DecibelAngle(s_ma.0 .1.decibel(), s_ma.0 .1.angle()),
            ),
            (
                DecibelAngle(s_ma.1 .0.decibel(), s_ma.1 .0.angle()),
                DecibelAngle(s_ma.1 .1.decibel(), s_ma.1 .1.angle()),
            ),
        );
        return ParsedDataLine {
            frequency: frequency.to_string(),
            s_ri,
            s_db,
            s_ma,
        };
    } else if format == "DB" {
        // Decibel-Angle format
        let s_db = DecibelAngleMatrix(
            (
                DecibelAngle(f32_parts[1], f32_parts[2]),
                DecibelAngle(f32_parts[3], f32_parts[4]),
            ),
            (
                DecibelAngle(f32_parts[5], f32_parts[6]),
                DecibelAngle(f32_parts[7], f32_parts[8]),
            ),
        );

        let s_ri = RealImaginaryMatrix(
            (
                RealImaginary(
                    10f32.powf(s_db.0 .0 .0 / 20.0) * s_db.0 .0 .1.cos(),
                    10f32.powf(s_db.0 .0 .0 / 20.0) * s_db.0 .0 .1.sin(),
                ),
                RealImaginary(
                    10f32.powf(s_db.0 .1 .0 / 20.0) * s_db.0 .1 .1.cos(),
                    10f32.powf(s_db.0 .1 .0 / 20.0) * s_db.0 .1 .1.sin(),
                ),
            ),
            (
                RealImaginary(
                    10f32.powf(s_db.1 .0 .0 / 20.0) * s_db.1 .0 .1.cos(),
                    10f32.powf(s_db.1 .0 .0 / 20.0) * s_db.1 .0 .1.sin(),
                ),
                RealImaginary(
                    10f32.powf(s_db.1 .1 .0 / 20.0) * s_db.1 .1 .1.cos(),
                    10f32.powf(s_db.1 .1 .0 / 20.0) * s_db.1 .1 .1.sin(),
                ),
            ),
        );

        let s_ma = MagnitudeAngleMatrix(
            (
                MagnitudeAngle(s_db.0 .0.magnitude(), s_db.0 .0.angle()),
                MagnitudeAngle(s_db.0 .1.magnitude(), s_db.0 .1.angle()),
            ),
            (
                MagnitudeAngle(s_db.1 .0.magnitude(), s_db.1 .0.angle()),
                MagnitudeAngle(s_db.1 .1.magnitude(), s_db.1 .1.angle()),
            ),
        );
        return ParsedDataLine {
            frequency: frequency.to_string(),
            s_ri,
            s_db,
            s_ma,
        };
    }else {
        panic!("Unsupported format: {}", format);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_str_to_f32() {
        let x = "3.14";
        let y = super::str_to_f32(x);
        assert_eq!(y, 3.14);
    }

    #[test]
    fn test_str_to_f32_invalid() {
        let x = "abc";
        let result = std::panic::catch_unwind(|| {
            super::str_to_f32(x);
        });
        assert!(result.is_err());
    }
}
