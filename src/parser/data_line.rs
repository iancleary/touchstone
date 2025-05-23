// FROM docs/touchstone_ver2_1.pdf (Page 6)
//
// format    specifies the format of the network parameter data pairs.  Legal values are:
// DB for decibel-angle (decibel = 20 × log10|magnitude|)
// MA for magnitude-angle,
// RI for real-imaginary.
// Angles are given in degrees.  Note that this format does not apply to noise
// parameters (refer to the “Noise Parameter Data” section later in this
// specification).  The default value is MA.

#[derive(Debug)]
struct MagnitudeAngle(f32, f32);

#[derive(Clone, Copy, Debug)]
struct RealImaginary(f32, f32);

impl RealImaginary {
    pub fn decibel(self) -> f32 {
        // need to resolve file from 2010 on if this should be 20
        // TODO: I think the s2p file is old/bad in this regard
        // (look into this after I sleep and think about it)
        10.0 * (f32::powf(self.0, 2.0) + f32::powf(self.1, 2.0))
            .sqrt()
            .log10()
    }
    pub fn magnitude(self) -> f32 {
        (f32::powf(self.0, 2.0) + f32::powf(self.1, 2.0)).sqrt()
    }

    pub fn angle(self) -> f32 {
        (self.1 / self.0).atan()
    }
}

#[derive(Debug)]
struct DecibelAngle(f32, f32);
// As specified, this is dB20, not dB10

#[derive(Debug)]
struct RealImaginaryMatrix(RealImaginary, RealImaginary, RealImaginary, RealImaginary);

#[derive(Debug)]
struct MagnitudeAngleMatrix(
    MagnitudeAngle,
    MagnitudeAngle,
    MagnitudeAngle,
    MagnitudeAngle,
);

#[derive(Debug)]
struct DecibelAngleMatrix(DecibelAngle, DecibelAngle, DecibelAngle, DecibelAngle);

fn str_to_f32(x: &str) -> f32 {
    x.parse::<f32>().expect("Failed to parse {x} into f32")
}

pub fn parse_data_line(data_line: String, format: &String) {
    println!("\n");
    // println!("format:\n{:?}", *format);

    // println!("Data Line: {data_line}");
    let parts = data_line.split_whitespace().collect::<Vec<_>>();

    let f32_parts: Vec<_> = parts.clone().into_iter().map(str_to_f32).collect();

    let len_parts = f32_parts.len();

    // println!("{}", len_parts);
    // println!("f32_parts (len {}): {:?}", len_parts, f32_parts);

    let mut frequency = "";

    // FROM docs/touchstone_ver2_1.pdf (Page 16)
    //
    // 2-port data (line)
    // <frequency value>  <N11> <N12> <N21> <N22>

    // where
    // frequency value  frequency at which the network parameter data was taken or derived.

    // N11, N12, N21, N22   network parameter data points, where Nij represents a pair of data values

    // Assuming only two port right now
    match len_parts.to_string().as_str() {
        "9" => {
            frequency = parts[0];
            let real_imaginary_matrix = RealImaginaryMatrix(
                RealImaginary(f32_parts[1], f32_parts[2]),
                RealImaginary(f32_parts[3], f32_parts[4]),
                RealImaginary(f32_parts[5], f32_parts[6]),
                RealImaginary(f32_parts[7], f32_parts[8]),
            );

            let magnitude_angle_matrix = MagnitudeAngleMatrix(
                MagnitudeAngle(
                    real_imaginary_matrix.0.magnitude(),
                    real_imaginary_matrix.0.angle(),
                ),
                MagnitudeAngle(
                    real_imaginary_matrix.1.magnitude(),
                    real_imaginary_matrix.1.angle(),
                ),
                MagnitudeAngle(
                    real_imaginary_matrix.2.magnitude(),
                    real_imaginary_matrix.2.angle(),
                ),
                MagnitudeAngle(
                    real_imaginary_matrix.3.magnitude(),
                    real_imaginary_matrix.3.angle(),
                ),
            );

            let decibel_angle_matrix = DecibelAngleMatrix(
                DecibelAngle(
                    real_imaginary_matrix.0.decibel(),
                    real_imaginary_matrix.0.angle(),
                ),
                DecibelAngle(
                    real_imaginary_matrix.1.decibel(),
                    real_imaginary_matrix.1.angle(),
                ),
                DecibelAngle(
                    real_imaginary_matrix.2.decibel(),
                    real_imaginary_matrix.2.angle(),
                ),
                DecibelAngle(
                    real_imaginary_matrix.3.decibel(),
                    real_imaginary_matrix.3.angle(),
                ),
            );

            // println!(
            //     "mag/dB, angle, {}/{} dB, {} degrees",
            //     real_imaginary_matrix.0.magnitude(),
            //     real_imaginary_matrix.0.decibel(),
            //     real_imaginary_matrix.0.angle()
            // );

            println!("{}, {:?}", frequency, real_imaginary_matrix);
            println!("{}, {:?}", frequency, magnitude_angle_matrix);
            println!("{}, {:?}", frequency, decibel_angle_matrix);
        }
        _ => {} // Do nothing (should raise error on unsupported cases)
    }
}
