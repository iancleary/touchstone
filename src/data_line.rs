use crate::utils::str_to_f64;

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
pub struct MagnitudeAngle(pub f64, pub f64);

impl PartialEq for MagnitudeAngle {
    fn eq(&self, other: &Self) -> bool {
        self.0  == other.0 && self.1 == other.1
    }
}


#[derive(Clone, Copy, Debug)]
pub struct RealImaginary(pub f64, pub f64);

impl PartialEq for RealImaginary {
    fn eq(&self, other: &Self) -> bool {
        self.0  == other.0 && self.1 == other.1
    }
}


#[derive(Clone, Copy, Debug)]
pub struct DecibelAngle(pub f64, pub f64);
// As specified, this is dB20, not dB10

impl PartialEq for DecibelAngle {
    fn eq(&self, other: &Self) -> bool {
        self.0  == other.0 && self.1 == other.1
    }
}


#[allow(dead_code)]
impl RealImaginary {
    pub fn real(self) -> f64 {
        self.0
    }
    pub fn imaginary(self) -> f64 {
        self.1
    }

    pub fn magnitude(self) -> f64 {
        (f64::powf(self.0, 2.0) + f64::powf(self.1, 2.0)).sqrt()
    }

    pub fn decibel(self) -> f64 {
        // need to resolve file from 2010 on if this should be 20
        // TODO: I think the s2p file is old/bad in this regard
        // (look into this after I sleep and think about it)
        20.0 * (f64::powf(self.0, 2.0) + f64::powf(self.1, 2.0))
            .sqrt()
            .log10()
    }

    pub fn angle(self) -> f64 {
        (self.1 / self.0).atan()
    }

    pub fn magnitude_angle(self) -> MagnitudeAngle {
        MagnitudeAngle(self.magnitude(), self.angle())
    }

    pub fn from_magnitude_angle(ma: MagnitudeAngle) -> Self {
        RealImaginary(ma.real(), ma.imaginary())
    }

    pub fn decibel_angle(self) -> DecibelAngle {
        DecibelAngle(self.decibel(), self.angle())
    }

    pub fn from_decibel_angle(da: DecibelAngle) -> Self {
        RealImaginary(da.real(), da.imaginary())
    }
}

#[allow(dead_code)]
impl MagnitudeAngle {
    pub fn decibel(self) -> f64 {
        self.0.log10() * 20.0
    }

    pub fn magnitude(self) -> f64 {
        self.0
    }

    pub fn angle(self) -> f64 {
        self.1
    }
    pub fn real(self) -> f64 {
        self.0 * self.1.cos()
    }

    pub fn imaginary(self) -> f64 {
        self.0 * self.1.sin()
    }

    pub fn real_imaginary(self) -> RealImaginary {
        RealImaginary(self.real(), self.imaginary())
    }

    pub fn from_real_imaginary(ri: RealImaginary) -> Self {
        MagnitudeAngle(ri.magnitude(), ri.angle())
    }

    pub fn decible_angle(self) -> DecibelAngle {
        DecibelAngle(self.decibel(), self.angle())
    }

    pub fn from_decibel_angle(da: DecibelAngle) -> Self {
        MagnitudeAngle(da.magnitude(), da.angle())
    }
}

#[allow(dead_code)]
impl DecibelAngle {
    pub fn magnitude(self) -> f64 {
        10f64.powf(self.0 / 20.0)
    }

    pub fn angle(self) -> f64 {
        self.1
    }

    pub fn real(self) -> f64 {
        self.magnitude() * self.angle().cos()
    }

    pub fn imaginary(self) -> f64 {
        self.magnitude() * self.angle().sin()
    }

    pub fn real_imaginary(self) -> RealImaginary {
        RealImaginary(self.real(), self.imaginary())
    }

    pub fn from_real_imaginary(ri: RealImaginary) -> Self {
        DecibelAngle(ri.decibel(), ri.angle())
    }

    pub fn magnitude_angle(self) -> MagnitudeAngle {
        MagnitudeAngle(self.magnitude(), self.angle())
    }

    pub fn from_magnitude_angle(ma: MagnitudeAngle) -> Self {
        DecibelAngle(ma.decibel(), ma.angle())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RealImaginaryMatrix(
    pub (RealImaginary, RealImaginary),
    pub (RealImaginary, RealImaginary),
);

impl PartialEq for RealImaginaryMatrix {
    fn eq(&self, other: &Self) -> bool {
        self.0 .0 .0 == other.0 .0 .0
            && self.0 .0 .1 == other.0 .0 .1
            && self.0 .1 .0 == other.0 .1 .0
            && self.0 .1 .1 == other.0 .1 .1
            && self.1 .0 .0 == other.1 .0 .0
            && self.1 .0 .1 == other.1 .0 .1
            && self.1 .1 .0 == other.1 .1 .0
            && self.1 .1 .1 == other.1 .1 .1
    }
}


#[derive(Clone, Copy, Debug)]
pub struct MagnitudeAngleMatrix(
    pub (MagnitudeAngle, MagnitudeAngle),
    pub (MagnitudeAngle, MagnitudeAngle),
);

impl PartialEq for MagnitudeAngleMatrix {
    fn eq(&self, other: &Self) -> bool {
        self.0 .0 .0 == other.0 .0 .0
            && self.0 .0 .1 == other.0 .0 .1
            && self.0 .1 .0 == other.0 .1 .0
            && self.0 .1 .1 == other.0 .1 .1
            && self.1 .0 .0 == other.1 .0 .0
            && self.1 .0 .1 == other.1 .0 .1
            && self.1 .1 .0 == other.1 .1 .0
            && self.1 .1 .1 == other.1 .1 .1
    }
}




#[derive(Clone, Copy, Debug)]
pub struct DecibelAngleMatrix(
    pub (DecibelAngle, DecibelAngle),
    pub (DecibelAngle, DecibelAngle),
);

impl PartialEq for DecibelAngleMatrix {
    fn eq(&self, other: &Self) -> bool {
        self.0 .0 .0 == other.0 .0 .0
            && self.0 .0 .1 == other.0 .0 .1
            && self.0 .1 .0 == other.0 .1 .0
            && self.0 .1 .1 == other.0 .1 .1
            && self.1 .0 .0 == other.1 .0 .0
            && self.1 .0 .1 == other.1 .0 .1
            && self.1 .1 .0 == other.1 .1 .0
            && self.1 .1 .1 == other.1 .1 .1
    }
}


#[derive(Clone, Copy, Debug)]
pub struct ParsedDataLine {
    pub frequency: f64,
    pub s_ri: RealImaginaryMatrix,
    pub s_db: DecibelAngleMatrix,
    pub s_ma: MagnitudeAngleMatrix,
}

pub(crate) fn parse_data_line(
    data_line: String,
    format: &String,
    n: &i32,
    frequency_unit: &String,
) -> ParsedDataLine {
    // println!("\n");
    // println!("format:\n{:?}", *format);
    // println!("n (number of ports): {:?}", *n);

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
    // println!("expected number of parts: {:?}", expect_number_of_parts);

    // println!("Data Line: {data_line}");
    let parts = data_line.split_whitespace().collect::<Vec<_>>();

    let len_parts = parts.len();
    // println!("Data Line Parts (len {}): {:?}", len_parts, parts);

    if len_parts != expect_number_of_parts as usize {
        panic!(
            "Data line has unexpected number of parts. Expected {}, got {}",
            expect_number_of_parts, len_parts
        );
    }

    // split into f64 parts, after checking the expected length
    let f64_parts: Vec<_> = parts.clone().into_iter().map(str_to_f64).collect();

    // println!("{}", len_parts);
    // println!("f64_parts (len {}): {:?}", len_parts, f64_parts);

    if n != &2 {
        panic!(
            "Only 2-port data lines are currently supported. Found {}-port data line.",
            n
        );
    }

    let mut frequency = str_to_f64(parts[0]);

    if frequency_unit == "THz" {
        // convert to Hz
        // println!("Converting frequency from THz to Hz");
        // println!("Original frequency: {} THz", frequency);
        frequency = rfconversions::frequency::thz_to_hz(frequency);
        println!("Converted frequency: {} Hz", frequency);
    } else if frequency_unit == "GHz" {
        // convert to Hz
        // println!("Converting frequency from GHz to Hz");
        // println!("Original frequency: {} GHz", frequency);
        frequency = rfconversions::frequency::ghz_to_hz(frequency);
        // println!("Converted frequency: {} Hz", frequency);
    } else if frequency_unit == "MHz" {
        // convert to Hz
        // println!("Converting frequency from MHz to Hz");
        // println!("Original frequency: {} MHz", frequency);
        frequency = rfconversions::frequency::mhz_to_hz(frequency);
        // println!("Converted frequency: {} Hz", frequency);
    } else if frequency_unit == "kHz" {
        // convert to Hz
        // println!("Converting frequency from kHz to Hz");
        // println!("Original frequency: {} kHz", frequency);
        frequency = rfconversions::frequency::khz_to_hz(frequency);
        // println!("Converted frequency: {} Hz", frequency);
    } else if frequency_unit == "Hz" {
        // no conversion needed
        // println!("Frequency is already in Hz: {} Hz", frequency);
    } else {
        panic!("Unsupported frequency unit: {}", frequency_unit);
    }

    if format == "RI" {
        // Real-Imaginary format
        let s_ri = RealImaginaryMatrix(
            (
                RealImaginary(f64_parts[1], f64_parts[2]),
                RealImaginary(f64_parts[3], f64_parts[4]),
            ),
            (
                RealImaginary(f64_parts[5], f64_parts[6]),
                RealImaginary(f64_parts[7], f64_parts[8]),
            ),
        );

        let s_db = DecibelAngleMatrix(
            (
                DecibelAngle::from_real_imaginary(s_ri.0 .0),
                DecibelAngle::from_real_imaginary(s_ri.0 .1),
            ),
            (
                DecibelAngle::from_real_imaginary(s_ri.1 .0),
                DecibelAngle::from_real_imaginary(s_ri.1 .1),
            ),
        );

        let s_ma = MagnitudeAngleMatrix(
            (
                MagnitudeAngle::from_real_imaginary(s_ri.0 .0),
                MagnitudeAngle::from_real_imaginary(s_ri.0 .1),
            ),
            (
                MagnitudeAngle::from_real_imaginary(s_ri.1 .0),
                MagnitudeAngle::from_real_imaginary(s_ri.1 .1),
            ),
        );

        return ParsedDataLine {
            frequency: frequency,
            s_ri,
            s_db,
            s_ma,
        };
    } else if format == "MA" {
        // Magnitude-Angle format
        let s_ma = MagnitudeAngleMatrix(
            (
                MagnitudeAngle(f64_parts[1], f64_parts[2]),
                MagnitudeAngle(f64_parts[3], f64_parts[4]),
            ),
            (
                MagnitudeAngle(f64_parts[5], f64_parts[6]),
                MagnitudeAngle(f64_parts[7], f64_parts[8]),
            ),
        );

        let s_ri = RealImaginaryMatrix(
            (
                RealImaginary::from_magnitude_angle(s_ma.0 .0),
                RealImaginary::from_magnitude_angle(s_ma.0 .1),
            ),
            (
                RealImaginary::from_magnitude_angle(s_ma.1 .0),
                RealImaginary::from_magnitude_angle(s_ma.1 .1),
            ),
        );

        let s_db = DecibelAngleMatrix(
            (
                DecibelAngle::from_magnitude_angle(s_ma.0 .0),
                DecibelAngle::from_magnitude_angle(s_ma.0 .1),
            ),
            (
                DecibelAngle::from_magnitude_angle(s_ma.1 .0),
                DecibelAngle::from_magnitude_angle(s_ma.1 .1),
            ),
        );
        return ParsedDataLine {
            frequency: frequency,
            s_ri,
            s_db,
            s_ma,
        };
    } else if format == "DB" {
        // Decibel-Angle format
        let s_db = DecibelAngleMatrix(
            (
                DecibelAngle(f64_parts[1], f64_parts[2]),
                DecibelAngle(f64_parts[3], f64_parts[4]),
            ),
            (
                DecibelAngle(f64_parts[5], f64_parts[6]),
                DecibelAngle(f64_parts[7], f64_parts[8]),
            ),
        );

        let s_ri = RealImaginaryMatrix(
            (
                RealImaginary::from_decibel_angle(s_db.0 .0),
                RealImaginary::from_decibel_angle(s_db.0 .1),
            ),
            (
                RealImaginary::from_decibel_angle(s_db.1 .0),
                RealImaginary::from_decibel_angle(s_db.1 .1),
            ),
        );

        let s_ma = MagnitudeAngleMatrix(
            (
                MagnitudeAngle::from_decibel_angle(s_db.0 .0),
                MagnitudeAngle::from_decibel_angle(s_db.0 .1),
            ),
            (
                MagnitudeAngle::from_decibel_angle(s_db.1 .0),
                MagnitudeAngle::from_decibel_angle(s_db.1 .1),
            ),
        );
        return ParsedDataLine {
            frequency: frequency,
            s_ri,
            s_db,
            s_ma,
        };
    } else {
        panic!("Unsupported format: {}", format);
    }
}
