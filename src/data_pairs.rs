use crate::utils::degrees_to_radians;
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
        self.0 == other.0 && self.1 == other.1
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RealImaginary(pub f64, pub f64);

impl PartialEq for RealImaginary {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1 == other.1
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DecibelAngle(pub f64, pub f64);
// As specified, this is dB20, not dB10

impl PartialEq for DecibelAngle {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1 == other.1
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
        // format specifies the format of the network parameter data pairs. Legal values are:
        // DB for decibel-angle (decibel = 20 × log10|magnitude|)
        20.0 * (f64::powf(self.0, 2.0) + f64::powf(self.1, 2.0))
            .sqrt()
            .log10()
    }

    pub fn angle(self) -> f64 {
        // https://docs.rs/libm/latest/libm/fn.atan2.html
        // Arctangent of y/x (f64)
        //
        // Computes the inverse tangent (arc tangent) of y/x.
        // Produces the correct result even for angles near pi/2 or -pi/2
        // (that is, when x is near 0). Returns a value in radians, in the range of -pi to pi.
        f64::atan2(self.1, self.0) * 180.0 / std::f64::consts::PI
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

    // Rule 5. All angles are measured in degrees.
    pub fn angle(self) -> f64 {
        self.1
    }

    // Rule 5. All angles are measured in degrees.
    pub fn real(self) -> f64 {
        self.0 * degrees_to_radians(self.1).cos()
    }

    // Rule 5. All angles are measured in degrees.
    pub fn imaginary(self) -> f64 {
        self.0 * degrees_to_radians(self.1).sin()
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
    pub fn decibel(self) -> f64 {
        self.0
    }

    pub fn magnitude(self) -> f64 {
        10f64.powf(self.0 / 20.0)
    }

    pub fn angle(self) -> f64 {
        self.1
    }

    // Rule 5. All angles are measured in degrees.
    pub fn real(self) -> f64 {
        self.magnitude() * degrees_to_radians(self.angle()).cos()
    }

    // Rule 5. All angles are measured in degrees.
    pub fn imaginary(self) -> f64 {
        self.magnitude() * degrees_to_radians(self.angle()).sin()
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

impl RealImaginaryMatrix {
    pub fn get(&self, j: i8, k: i8) -> RealImaginary {
        match (j, k) {
            (1, 1) => self.0 .0,
            (1, 2) => self.0 .1,
            (2, 1) => self.1 .0,
            (2, 2) => self.1 .1,
            _ => panic!("Invalid port numbers: {}, {}", j, k),
        }
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

impl MagnitudeAngleMatrix {
    pub fn get(&self, j: i8, k: i8) -> MagnitudeAngle {
        match (j, k) {
            (1, 1) => self.0 .0,
            (1, 2) => self.0 .1,
            (2, 1) => self.1 .0,
            (2, 2) => self.1 .1,
            _ => panic!("Invalid port numbers: {}, {}", j, k),
        }
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

impl DecibelAngleMatrix {
    pub fn get(&self, j: i8, k: i8) -> DecibelAngle {
        match (j, k) {
            (1, 1) => self.0 .0,
            (1, 2) => self.0 .1,
            (2, 1) => self.1 .0,
            (2, 2) => self.1 .1,
            _ => panic!("Invalid port numbers: {}, {}", j, k),
        }
    }

    pub fn from_magnitude_angle_matrix(ma_matrix: MagnitudeAngleMatrix) -> Self {
        DecibelAngleMatrix(
            (
                DecibelAngle::from_magnitude_angle(ma_matrix.0 .0),
                DecibelAngle::from_magnitude_angle(ma_matrix.0 .1),
            ),
            (
                DecibelAngle::from_magnitude_angle(ma_matrix.1 .0),
                DecibelAngle::from_magnitude_angle(ma_matrix.1 .1),
            ),
        )
    }
}

#[cfg(test)]
mod tests {

    fn round_to_nine_decimal_places(value: f64) -> f64 {
        f64::round(value * 1e9) / 1e9
    }

    use super::*;
    #[test]
    fn decibel_angle() {
        // tuple struct, so need to use 0 and 1
        let da = DecibelAngle(20.0, 45.0);

        assert_eq!(da.magnitude(), 10.0);
        assert_eq!(da.angle(), 45.0);
        assert_eq!(da.decibel(), 20.0);

        // keeps 9 decimal places
        assert_eq!(round_to_nine_decimal_places(da.real()), 7.071067812);

        // keeps 9 decimal places
        assert_eq!(round_to_nine_decimal_places(da.imaginary()), 7.071067812);

        let da_from_ri = DecibelAngle::from_real_imaginary(RealImaginary(7.071067812, 7.071067812));
        assert_eq!(round_to_nine_decimal_places(da_from_ri.magnitude()), 10.0);
        assert_eq!(round_to_nine_decimal_places(da_from_ri.angle()), 45.0);

        let da_from_ma = DecibelAngle::from_magnitude_angle(MagnitudeAngle(10.0, 45.0));
        assert_eq!(round_to_nine_decimal_places(da_from_ma.magnitude()), 10.0);
        assert_eq!(round_to_nine_decimal_places(da_from_ma.angle()), 45.0);
    }

    #[test]
    fn magnitude_angle() {
        // tuple struct, so need to use 0 and 1
        let ma = MagnitudeAngle(10.0, 45.0);

        assert_eq!(ma.magnitude(), 10.0);
        assert_eq!(ma.angle(), 45.0);
        assert_eq!(ma.decibel(), 20.0);

        // keeps 9 decimal places
        assert_eq!(round_to_nine_decimal_places(ma.real()), 7.071067812);

        // keeps 9 decimal places
        assert_eq!(round_to_nine_decimal_places(ma.imaginary()), 7.071067812);

        let ma_from_ri =
            MagnitudeAngle::from_real_imaginary(RealImaginary(7.071067812, 7.071067812));
        assert_eq!(round_to_nine_decimal_places(ma_from_ri.magnitude()), 10.0);
        assert_eq!(round_to_nine_decimal_places(ma_from_ri.angle()), 45.0);

        let ma_from_da = MagnitudeAngle::from_decibel_angle(DecibelAngle(20.0, 45.0));
        assert_eq!(round_to_nine_decimal_places(ma_from_da.magnitude()), 10.0);
        assert_eq!(round_to_nine_decimal_places(ma_from_da.angle()), 45.0);
    }

    #[test]
    fn real_imaginary() {
        // tuple struct, so need to use 0 and 1
        let ma = RealImaginary(20.0, 0.0);

        assert_eq!(ma.magnitude(), 20.0);
        assert_eq!(ma.angle(), 0.0);
        assert_eq!(round_to_nine_decimal_places(ma.decibel()), 26.020599913);

        // keeps 9 decimal places
        assert_eq!(round_to_nine_decimal_places(ma.real()), 20.0);

        // keeps 9 decimal places
        assert_eq!(round_to_nine_decimal_places(ma.imaginary()), 0.0);
    }

    #[test]
    fn real_imaginary2() {
        // tuple struct, so need to use 0 and 1
        let ri = RealImaginary(0.0, 10.0);

        assert_eq!(round_to_nine_decimal_places(ri.magnitude()), 10.0);
        assert_eq!(ri.angle(), 90.0);
        assert_eq!(round_to_nine_decimal_places(ri.decibel()), 20.0);

        // keeps 9 decimal places
        assert_eq!(round_to_nine_decimal_places(ri.real()), 0.0);

        // keeps 9 decimal places
        assert_eq!(round_to_nine_decimal_places(ri.imaginary()), 10.0);
    }

    #[test]
    fn real_imaginary3() {
        // tuple struct, so need to use 0 and 1
        let ri = RealImaginary(10.0, 10.0);

        assert_eq!(round_to_nine_decimal_places(ri.magnitude()), 14.142135624);
        assert_eq!(ri.angle(), 45.0);
        assert_eq!(round_to_nine_decimal_places(ri.decibel()), 23.010299957);

        // keeps 9 decimal places
        assert_eq!(round_to_nine_decimal_places(ri.real()), 10.0);

        // keeps 9 decimal places
        assert_eq!(round_to_nine_decimal_places(ri.imaginary()), 10.0);
    }

    #[test]
    fn real_imaginary4() {
        // tuple struct, so need to use 0 and 1
        let ri = RealImaginary(-20.0, -20.0);

        assert_eq!(round_to_nine_decimal_places(ri.magnitude()), 28.284271247);
        assert_eq!(ri.angle(), -135.0);
        assert_eq!(round_to_nine_decimal_places(ri.decibel()), 29.03089987);

        // keeps 9 decimal places
        assert_eq!(round_to_nine_decimal_places(ri.real()), -20.0);

        // keeps 9 decimal places
        assert_eq!(round_to_nine_decimal_places(ri.imaginary()), -20.0);
    }
}
