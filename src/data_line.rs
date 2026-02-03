use crate::utils::str_to_f64;

use crate::data_pairs::DecibelAngle;
use crate::data_pairs::DecibelAngleMatrix;
use crate::data_pairs::MagnitudeAngle;
use crate::data_pairs::MagnitudeAngleMatrix;
use crate::data_pairs::RealImaginary;
use crate::data_pairs::RealImaginaryMatrix;

#[derive(Clone, Debug)]
pub struct ParsedDataLine {
    pub frequency: f64,
    pub s_ri: RealImaginaryMatrix,
    pub s_db: DecibelAngleMatrix,
    pub s_ma: MagnitudeAngleMatrix,
}

pub(crate) fn parse_data_line(
    data_lines: Vec<String>,
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

    // Combine all lines into a single vector of parts
    // This handles both single-line and multi-line format
    let mut parts = Vec::new();
    for line in &data_lines {
        // println!("Data Line: {}", line);
        let line_parts: Vec<_> = line
            .split_whitespace()
            .filter(|s| !s.starts_with('!')) // Skip inline comments
            .collect();
        parts.extend(line_parts);
    }

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

    let n_usize = *n as usize;

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
        // Build NxN matrix from f64_parts in row-major order
        let mut s_ri_data = Vec::new();
        for row in 0..n_usize {
            let mut row_vec = Vec::new();
            for col in 0..n_usize {
                let idx = 1 + 2 * (row * n_usize + col);
                row_vec.push(RealImaginary(f64_parts[idx], f64_parts[idx + 1]));
            }
            s_ri_data.push(row_vec);
        }
        let s_ri = RealImaginaryMatrix::from_vec(s_ri_data);

        // Convert to DecibelAngle format
        let mut s_db_data = Vec::new();
        for row in 0..n_usize {
            let mut row_vec = Vec::new();
            for col in 0..n_usize {
                row_vec.push(DecibelAngle::from_real_imaginary(
                    s_ri.get(row + 1, col + 1),
                ));
            }
            s_db_data.push(row_vec);
        }
        let s_db = DecibelAngleMatrix::from_vec(s_db_data);

        // Convert to MagnitudeAngle format
        let mut s_ma_data = Vec::new();
        for row in 0..n_usize {
            let mut row_vec = Vec::new();
            for col in 0..n_usize {
                row_vec.push(MagnitudeAngle::from_real_imaginary(
                    s_ri.get(row + 1, col + 1),
                ));
            }
            s_ma_data.push(row_vec);
        }
        let s_ma = MagnitudeAngleMatrix::from_vec(s_ma_data);

        ParsedDataLine {
            frequency,
            s_ri,
            s_db,
            s_ma,
        }
    } else if format == "MA" {
        // Magnitude-Angle format
        // Build NxN matrix from f64_parts in row-major order
        let mut s_ma_data = Vec::new();
        for row in 0..n_usize {
            let mut row_vec = Vec::new();
            for col in 0..n_usize {
                let idx = 1 + 2 * (row * n_usize + col);
                row_vec.push(MagnitudeAngle(f64_parts[idx], f64_parts[idx + 1]));
            }
            s_ma_data.push(row_vec);
        }
        let s_ma = MagnitudeAngleMatrix::from_vec(s_ma_data);

        // Convert to RealImaginary format
        let mut s_ri_data = Vec::new();
        for row in 0..n_usize {
            let mut row_vec = Vec::new();
            for col in 0..n_usize {
                row_vec.push(RealImaginary::from_magnitude_angle(
                    s_ma.get(row + 1, col + 1),
                ));
            }
            s_ri_data.push(row_vec);
        }
        let s_ri = RealImaginaryMatrix::from_vec(s_ri_data);

        // Convert to DecibelAngle format
        let mut s_db_data = Vec::new();
        for row in 0..n_usize {
            let mut row_vec = Vec::new();
            for col in 0..n_usize {
                row_vec.push(DecibelAngle::from_magnitude_angle(
                    s_ma.get(row + 1, col + 1),
                ));
            }
            s_db_data.push(row_vec);
        }
        let s_db = DecibelAngleMatrix::from_vec(s_db_data);

        ParsedDataLine {
            frequency,
            s_ri,
            s_db,
            s_ma,
        }
    } else if format == "DB" {
        // Decibel-Angle format
        // Build NxN matrix from f64_parts in row-major order
        let mut s_db_data = Vec::new();
        for row in 0..n_usize {
            let mut row_vec = Vec::new();
            for col in 0..n_usize {
                let idx = 1 + 2 * (row * n_usize + col);
                row_vec.push(DecibelAngle(f64_parts[idx], f64_parts[idx + 1]));
            }
            s_db_data.push(row_vec);
        }
        let s_db = DecibelAngleMatrix::from_vec(s_db_data);

        // Convert to RealImaginary format
        let mut s_ri_data = Vec::new();
        for row in 0..n_usize {
            let mut row_vec = Vec::new();
            for col in 0..n_usize {
                row_vec.push(RealImaginary::from_decibel_angle(
                    s_db.get(row + 1, col + 1),
                ));
            }
            s_ri_data.push(row_vec);
        }
        let s_ri = RealImaginaryMatrix::from_vec(s_ri_data);

        // Convert to MagnitudeAngle format
        let mut s_ma_data = Vec::new();
        for row in 0..n_usize {
            let mut row_vec = Vec::new();
            for col in 0..n_usize {
                row_vec.push(MagnitudeAngle::from_decibel_angle(
                    s_db.get(row + 1, col + 1),
                ));
            }
            s_ma_data.push(row_vec);
        }
        let s_ma = MagnitudeAngleMatrix::from_vec(s_ma_data);

        ParsedDataLine {
            frequency,
            s_ri,
            s_db,
            s_ma,
        }
    } else {
        panic!("Unsupported format: {}", format);
    }
}
