use crate::utils::degrees_to_radians;
use std::ops;
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

// Add
impl ops::Add for RealImaginary {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        RealImaginary(self.0 + other.0, self.1 + other.1)
    }
}

// Sub
impl ops::Sub for RealImaginary {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        RealImaginary(self.0 - other.0, self.1 - other.1)
    }
}

// Mul
impl ops::Mul for RealImaginary {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        RealImaginary(
            self.0 * other.0 - self.1 * other.1,
            self.0 * other.1 + self.1 * other.0,
        )
    }
}

// Mul<f64> for RealImaginary
impl ops::Mul<f64> for RealImaginary {
    type Output = Self;

    fn mul(self, other: f64) -> Self {
        RealImaginary(self.0 * other, self.1 * other)
    }
}

// Mul<RealImaginary> for f64
impl ops::Mul<RealImaginary> for f64 {
    type Output = RealImaginary;

    fn mul(self, other: RealImaginary) -> RealImaginary {
        RealImaginary(self * other.0, self * other.1)
    }
}

// Div
impl ops::Div for RealImaginary {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        let denominator = other.0 * other.0 + other.1 * other.1;
        RealImaginary(
            (self.0 * other.0 + self.1 * other.1) / denominator,
            (self.1 * other.0 - self.0 * other.1) / denominator,
        )
    }
}

// Div<f64> for RealImaginary
impl ops::Div<f64> for RealImaginary {
    type Output = Self;

    fn div(self, other: f64) -> Self {
        RealImaginary(self.0 / other, self.1 / other)
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

#[derive(Clone, Debug)]
pub struct RealImaginaryMatrix {
    data: Vec<Vec<RealImaginary>>,
    n: usize,
}

impl PartialEq for RealImaginaryMatrix {
    fn eq(&self, other: &Self) -> bool {
        self.n == other.n && self.data == other.data
    }
}

impl RealImaginaryMatrix {
    /// Create a new NxN matrix filled with zeros
    pub fn new(n: usize) -> Self {
        assert!(n > 0, "Matrix size must be at least 1");
        let data = vec![vec![RealImaginary(0.0, 0.0); n]; n];
        RealImaginaryMatrix { data, n }
    }

    /// Create a matrix from a 2D vector (validates square matrix)
    pub fn from_vec(data: Vec<Vec<RealImaginary>>) -> Self {
        let n = data.len();
        assert!(n > 0, "Matrix must have at least 1 row");
        for row in &data {
            assert_eq!(row.len(), n, "Matrix must be square (NxN)");
        }
        RealImaginaryMatrix { data, n }
    }

    /// Get the size of the matrix (N for NxN matrix)
    pub fn size(&self) -> usize {
        self.n
    }

    /// Get element at position (j, k) using 1-based indexing (S11, S21, etc.)
    /// j: row (1 to N), k: column (1 to N)
    pub fn get(&self, j: usize, k: usize) -> RealImaginary {
        assert!(
            j >= 1 && j <= self.n,
            "Row index j={} out of range [1, {}]",
            j,
            self.n
        );
        assert!(
            k >= 1 && k <= self.n,
            "Column index k={} out of range [1, {}]",
            k,
            self.n
        );
        self.data[j - 1][k - 1]
    }

    /// Set element at position (j, k) using 1-based indexing
    /// j: row (1 to N), k: column (1 to N)
    pub fn set(&mut self, j: usize, k: usize, value: RealImaginary) {
        assert!(
            j >= 1 && j <= self.n,
            "Row index j={} out of range [1, {}]",
            j,
            self.n
        );
        assert!(
            k >= 1 && k <= self.n,
            "Column index k={} out of range [1, {}]",
            k,
            self.n
        );
        self.data[j - 1][k - 1] = value;
    }
}

impl ops::Mul for RealImaginaryMatrix {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        assert_eq!(
            self.n, other.n,
            "Matrix sizes must match for multiplication"
        );
        let n = self.n;
        let mut result = RealImaginaryMatrix::new(n);

        // Standard matrix multiplication: C[i][j] = sum(A[i][k] * B[k][j])
        for i in 0..n {
            for j in 0..n {
                let mut sum = RealImaginary(0.0, 0.0);
                for k in 0..n {
                    sum = sum + self.data[i][k] * other.data[k][j];
                }
                result.data[i][j] = sum;
            }
        }

        result
    }
}
impl RealImaginaryMatrix {
    // Convert S-parameters to ABCD parameters (2x2 matrices only)
    // https://en.wikipedia.org/wiki/Scattering_parameters#Scattering_transfer_parameters
    // But we want ABCD (Transmission Matrix), not T-parameters (Scattering Transfer Matrix)
    // https://en.wikipedia.org/wiki/ABCD_parameters#S_parameters
    // A = ((1+S11)(1-S22) + S12S21) / (2S21)
    // B = Z0 * ((1+S11)(1+S22) - S12S21) / (2S21)
    // C = (1/Z0) * ((1-S11)(1-S22) - S12S21) / (2S21)
    // D = ((1-S11)(1+S22) + S12S21) / (2S21)
    pub fn to_abcd(&self, z0: f64) -> RealImaginaryMatrix {
        assert_eq!(
            self.n, 2,
            "ABCD conversion only supported for 2x2 S-parameter matrices"
        );

        let s11 = self.data[0][0];
        let s12 = self.data[0][1];
        let s21 = self.data[1][0];
        let s22 = self.data[1][1];

        let one = RealImaginary(1.0, 0.0);
        let two_s21 = s21 * 2.0;

        // A = ((1+S11)(1-S22) + S12S21) / (2S21)
        let a = ((one + s11) * (one - s22) + s12 * s21) / two_s21;

        // B = Z0 * ((1+S11)(1+S22) - S12S21) / (2S21)
        let b = ((one + s11) * (one + s22) - s12 * s21) * z0 / two_s21;

        // C = (1/Z0) * ((1-S11)(1-S22) - S12S21) / (2S21)
        let c = ((one - s11) * (one - s22) - s12 * s21) / z0 / two_s21;

        // D = ((1-S11)(1+S22) + S12S21) / (2S21)
        let d = ((one - s11) * (one + s22) + s12 * s21) / two_s21;

        RealImaginaryMatrix::from_vec(vec![vec![a, b], vec![c, d]])
    }

    // Convert ABCD parameters to S-parameters (2x2 matrices only)
    // https://en.wikipedia.org/wiki/ABCD_parameters#S_parameters
    // Denom = A + B/Z0 + C*Z0 + D
    // S11 = (A + B/Z0 - C*Z0 - D) / Denom
    // S12 = 2(AD - BC) / Denom  <-- Note: AD-BC is determinant, usually 1 for reciprocal networks
    // S21 = 2 / Denom
    // S22 = (-A + B/Z0 - C*Z0 + D) / Denom
    pub fn to_s(&self, z0: f64) -> RealImaginaryMatrix {
        assert_eq!(
            self.n, 2,
            "S-parameter conversion from ABCD only supported for 2x2 matrices"
        );

        let a = self.data[0][0];
        let b = self.data[0][1];
        let c = self.data[1][0];
        let d = self.data[1][1];

        let denom = a + b / z0 + c * z0 + d;

        // S11 = (A + B/Z0 - C*Z0 - D) / Denom
        let s11 = (a + b / z0 - c * z0 - d) / denom;

        // S12 = 2(AD - BC) / Denom
        let s12 = (a * d - b * c) * 2.0 / denom;

        // S21 = 2 / Denom
        let s21 = RealImaginary(2.0, 0.0) / denom;

        // S22 = (-A + B/Z0 - C*Z0 + D) / Denom
        let s22 = (RealImaginary(0.0, 0.0) - a + b / z0 - c * z0 + d) / denom;

        RealImaginaryMatrix::from_vec(vec![vec![s11, s12], vec![s21, s22]])
    }
}

#[derive(Clone, Debug)]
pub struct MagnitudeAngleMatrix {
    data: Vec<Vec<MagnitudeAngle>>,
    n: usize,
}

impl PartialEq for MagnitudeAngleMatrix {
    fn eq(&self, other: &Self) -> bool {
        self.n == other.n && self.data == other.data
    }
}

impl MagnitudeAngleMatrix {
    /// Create a new NxN matrix filled with zeros
    pub fn new(n: usize) -> Self {
        assert!(n > 0, "Matrix size must be at least 1");
        let data = vec![vec![MagnitudeAngle(0.0, 0.0); n]; n];
        MagnitudeAngleMatrix { data, n }
    }

    /// Create a matrix from a 2D vector (validates square matrix)
    pub fn from_vec(data: Vec<Vec<MagnitudeAngle>>) -> Self {
        let n = data.len();
        assert!(n > 0, "Matrix must have at least 1 row");
        for row in &data {
            assert_eq!(row.len(), n, "Matrix must be square (NxN)");
        }
        MagnitudeAngleMatrix { data, n }
    }

    /// Get the size of the matrix (N for NxN matrix)
    pub fn size(&self) -> usize {
        self.n
    }

    /// Get element at position (j, k) using 1-based indexing
    pub fn get(&self, j: usize, k: usize) -> MagnitudeAngle {
        assert!(
            j >= 1 && j <= self.n,
            "Row index j={} out of range [1, {}]",
            j,
            self.n
        );
        assert!(
            k >= 1 && k <= self.n,
            "Column index k={} out of range [1, {}]",
            k,
            self.n
        );
        self.data[j - 1][k - 1]
    }

    /// Set element at position (j, k) using 1-based indexing
    pub fn set(&mut self, j: usize, k: usize, value: MagnitudeAngle) {
        assert!(
            j >= 1 && j <= self.n,
            "Row index j={} out of range [1, {}]",
            j,
            self.n
        );
        assert!(
            k >= 1 && k <= self.n,
            "Column index k={} out of range [1, {}]",
            k,
            self.n
        );
        self.data[j - 1][k - 1] = value;
    }
}

#[derive(Clone, Debug)]
pub struct DecibelAngleMatrix {
    data: Vec<Vec<DecibelAngle>>,
    n: usize,
}

impl PartialEq for DecibelAngleMatrix {
    fn eq(&self, other: &Self) -> bool {
        self.n == other.n && self.data == other.data
    }
}

impl DecibelAngleMatrix {
    /// Create a new NxN matrix filled with zeros
    pub fn new(n: usize) -> Self {
        assert!(n > 0, "Matrix size must be at least 1");
        let data = vec![vec![DecibelAngle(0.0, 0.0); n]; n];
        DecibelAngleMatrix { data, n }
    }

    /// Create a matrix from a 2D vector (validates square matrix)
    pub fn from_vec(data: Vec<Vec<DecibelAngle>>) -> Self {
        let n = data.len();
        assert!(n > 0, "Matrix must have at least 1 row");
        for row in &data {
            assert_eq!(row.len(), n, "Matrix must be square (NxN)");
        }
        DecibelAngleMatrix { data, n }
    }

    /// Get the size of the matrix (N for NxN matrix)
    pub fn size(&self) -> usize {
        self.n
    }

    /// Get element at position (j, k) using 1-based indexing
    pub fn get(&self, j: usize, k: usize) -> DecibelAngle {
        assert!(
            j >= 1 && j <= self.n,
            "Row index j={} out of range [1, {}]",
            j,
            self.n
        );
        assert!(
            k >= 1 && k <= self.n,
            "Column index k={} out of range [1, {}]",
            k,
            self.n
        );
        self.data[j - 1][k - 1]
    }

    /// Set element at position (j, k) using 1-based indexing
    pub fn set(&mut self, j: usize, k: usize, value: DecibelAngle) {
        assert!(
            j >= 1 && j <= self.n,
            "Row index j={} out of range [1, {}]",
            j,
            self.n
        );
        assert!(
            k >= 1 && k <= self.n,
            "Column index k={} out of range [1, {}]",
            k,
            self.n
        );
        self.data[j - 1][k - 1] = value;
    }

    /// Convert from MagnitudeAngleMatrix
    pub fn from_magnitude_angle_matrix(ma_matrix: &MagnitudeAngleMatrix) -> Self {
        let n = ma_matrix.size();
        let mut result = DecibelAngleMatrix::new(n);
        for i in 0..n {
            for j in 0..n {
                result.data[i][j] = DecibelAngle::from_magnitude_angle(ma_matrix.data[i][j]);
            }
        }
        result
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

    #[test]
    fn test_real_imaginary_arithmetic() {
        let a = RealImaginary(1.0, 2.0);
        let b = RealImaginary(3.0, 4.0);

        // Add
        let c = a + b;
        assert_eq!(c, RealImaginary(4.0, 6.0));

        // Sub
        let d = b - a;
        assert_eq!(d, RealImaginary(2.0, 2.0));

        // Mul
        // (1+2i)(3+4i) = 3 + 4i + 6i - 8 = -5 + 10i
        let e = a * b;
        assert_eq!(e, RealImaginary(-5.0, 10.0));

        // Div
        // (1+2i)/(3+4i) = (1+2i)(3-4i)/(9+16) = (3 - 4i + 6i + 8)/25 = (11 + 2i)/25 = 0.44 + 0.08i
        let f = a / b;
        assert_eq!(f, RealImaginary(0.44, 0.08));
    }

    #[test]
    fn test_matrix_creation_and_access() {
        // Test 2x2 matrix
        let mut m2 = RealImaginaryMatrix::new(2);
        assert_eq!(m2.size(), 2);
        assert_eq!(m2.get(1, 1), RealImaginary(0.0, 0.0));

        // Set and get
        m2.set(1, 1, RealImaginary(1.0, 2.0));
        m2.set(2, 2, RealImaginary(3.0, 4.0));
        assert_eq!(m2.get(1, 1), RealImaginary(1.0, 2.0));
        assert_eq!(m2.get(2, 2), RealImaginary(3.0, 4.0));

        // Test 3x3 matrix
        let mut m3 = RealImaginaryMatrix::new(3);
        assert_eq!(m3.size(), 3);
        for i in 1..=3 {
            for j in 1..=3 {
                assert_eq!(m3.get(i, j), RealImaginary(0.0, 0.0));
            }
        }

        // Set elements
        m3.set(1, 1, RealImaginary(1.0, 0.0));
        m3.set(2, 2, RealImaginary(2.0, 0.0));
        m3.set(3, 3, RealImaginary(3.0, 0.0));
        assert_eq!(m3.get(1, 1), RealImaginary(1.0, 0.0));
        assert_eq!(m3.get(2, 2), RealImaginary(2.0, 0.0));
        assert_eq!(m3.get(3, 3), RealImaginary(3.0, 0.0));

        // Test 4x4 matrix
        let m4 = RealImaginaryMatrix::new(4);
        assert_eq!(m4.size(), 4);
    }

    #[test]
    fn test_matrix_from_vec() {
        // Create a 2x2 matrix from vector
        let m = RealImaginaryMatrix::from_vec(vec![
            vec![RealImaginary(1.0, 0.0), RealImaginary(2.0, 0.0)],
            vec![RealImaginary(3.0, 0.0), RealImaginary(4.0, 0.0)],
        ]);
        assert_eq!(m.size(), 2);
        assert_eq!(m.get(1, 1), RealImaginary(1.0, 0.0));
        assert_eq!(m.get(1, 2), RealImaginary(2.0, 0.0));
        assert_eq!(m.get(2, 1), RealImaginary(3.0, 0.0));
        assert_eq!(m.get(2, 2), RealImaginary(4.0, 0.0));

        // Create a 3x3 identity-like matrix
        let m3 = RealImaginaryMatrix::from_vec(vec![
            vec![
                RealImaginary(1.0, 0.0),
                RealImaginary(0.0, 0.0),
                RealImaginary(0.0, 0.0),
            ],
            vec![
                RealImaginary(0.0, 0.0),
                RealImaginary(1.0, 0.0),
                RealImaginary(0.0, 0.0),
            ],
            vec![
                RealImaginary(0.0, 0.0),
                RealImaginary(0.0, 0.0),
                RealImaginary(1.0, 0.0),
            ],
        ]);
        assert_eq!(m3.size(), 3);
        assert_eq!(m3.get(1, 1), RealImaginary(1.0, 0.0));
        assert_eq!(m3.get(2, 2), RealImaginary(1.0, 0.0));
        assert_eq!(m3.get(3, 3), RealImaginary(1.0, 0.0));
    }

    #[test]
    #[should_panic(expected = "out of range")]
    fn test_matrix_bounds_checking() {
        let m = RealImaginaryMatrix::new(2);
        m.get(3, 1); // Should panic
    }

    #[test]
    fn test_matrix_multiplication_2x2() {
        // Test 2x2 matrix multiplication
        let a = RealImaginaryMatrix::from_vec(vec![
            vec![RealImaginary(1.0, 0.0), RealImaginary(2.0, 0.0)],
            vec![RealImaginary(3.0, 0.0), RealImaginary(4.0, 0.0)],
        ]);

        let b = RealImaginaryMatrix::from_vec(vec![
            vec![RealImaginary(5.0, 0.0), RealImaginary(6.0, 0.0)],
            vec![RealImaginary(7.0, 0.0), RealImaginary(8.0, 0.0)],
        ]);

        let c = a * b;
        // [[1,2],[3,4]] * [[5,6],[7,8]] = [[19,22],[43,50]]
        assert_eq!(c.get(1, 1), RealImaginary(19.0, 0.0));
        assert_eq!(c.get(1, 2), RealImaginary(22.0, 0.0));
        assert_eq!(c.get(2, 1), RealImaginary(43.0, 0.0));
        assert_eq!(c.get(2, 2), RealImaginary(50.0, 0.0));
    }

    #[test]
    fn test_matrix_multiplication_3x3() {
        // Test 3x3 identity matrix multiplication
        let identity = RealImaginaryMatrix::from_vec(vec![
            vec![
                RealImaginary(1.0, 0.0),
                RealImaginary(0.0, 0.0),
                RealImaginary(0.0, 0.0),
            ],
            vec![
                RealImaginary(0.0, 0.0),
                RealImaginary(1.0, 0.0),
                RealImaginary(0.0, 0.0),
            ],
            vec![
                RealImaginary(0.0, 0.0),
                RealImaginary(0.0, 0.0),
                RealImaginary(1.0, 0.0),
            ],
        ]);

        let a = RealImaginaryMatrix::from_vec(vec![
            vec![
                RealImaginary(1.0, 0.0),
                RealImaginary(2.0, 0.0),
                RealImaginary(3.0, 0.0),
            ],
            vec![
                RealImaginary(4.0, 0.0),
                RealImaginary(5.0, 0.0),
                RealImaginary(6.0, 0.0),
            ],
            vec![
                RealImaginary(7.0, 0.0),
                RealImaginary(8.0, 0.0),
                RealImaginary(9.0, 0.0),
            ],
        ]);

        let result = identity.clone() * a.clone();
        assert_eq!(result, a); // Identity * A = A
    }

    #[test]
    fn test_magnitude_angle_matrix() {
        let mut m = MagnitudeAngleMatrix::new(2);
        m.set(1, 1, MagnitudeAngle(10.0, 45.0));
        m.set(2, 2, MagnitudeAngle(5.0, 90.0));

        assert_eq!(m.get(1, 1), MagnitudeAngle(10.0, 45.0));
        assert_eq!(m.get(2, 2), MagnitudeAngle(5.0, 90.0));
        assert_eq!(m.size(), 2);
    }

    #[test]
    fn test_decibel_angle_matrix() {
        let mut m = DecibelAngleMatrix::new(3);
        m.set(1, 1, DecibelAngle(20.0, 0.0));
        m.set(2, 2, DecibelAngle(15.0, 45.0));
        m.set(3, 3, DecibelAngle(10.0, 90.0));

        assert_eq!(m.get(1, 1), DecibelAngle(20.0, 0.0));
        assert_eq!(m.get(2, 2), DecibelAngle(15.0, 45.0));
        assert_eq!(m.get(3, 3), DecibelAngle(10.0, 90.0));
        assert_eq!(m.size(), 3);
    }

    #[test]
    fn test_matrix_format_conversion() {
        // Create a 2x2 MagnitudeAngle matrix
        let ma = MagnitudeAngleMatrix::from_vec(vec![
            vec![MagnitudeAngle(10.0, 45.0), MagnitudeAngle(5.0, 90.0)],
            vec![MagnitudeAngle(2.0, 0.0), MagnitudeAngle(8.0, 180.0)],
        ]);

        // Convert to DecibelAngle
        let db = DecibelAngleMatrix::from_magnitude_angle_matrix(&ma);
        assert_eq!(db.size(), 2);

        // Verify conversion of first element: 10.0 mag -> 20.0 dB
        assert_eq!(round_to_nine_decimal_places(db.get(1, 1).0), 20.0);
        assert_eq!(db.get(1, 1).1, 45.0);
    }

    #[test]
    fn test_s_to_abcd_conversion() {
        // Identity S-matrix (matched, no transmission) -> ABCD?
        // S = [[0, 0], [0, 0]] -> Z0 * [[1, 0], [0, 1]] ? No, that's not right.
        // Let's use a known conversion.
        // S to ABCD
        // A = ((1+S11)(1-S22) + S12S21) / (2S21)
        // B = Z0 * ((1+S11)(1+S22) - S12S21) / (2S21)
        // C = (1/Z0) * ((1-S11)(1-S22) - S12S21) / (2S21)
        // D = ((1-S11)(1+S22) + S12S21) / (2S21)

        // Let's just verify the functions exist and compile for now in the main code,
        // and maybe add a simple test case if I can calculate one easily.
        // S = [[0, 1], [1, 0]] (Through connection)
        // A = ((1)(1) + 1) / 2 = 1
        // B = 50 * ((1)(1) - 1) / 2 = 0
        // C = (1/50) * ((1)(1) - 1) / 2 = 0
        // D = ((1)(1) + 1) / 2 = 1
        // So S=[[0,1],[1,0]] -> ABCD=[[1,0],[0,1]]

        let s = RealImaginaryMatrix::from_vec(vec![
            vec![RealImaginary(0.0, 0.0), RealImaginary(1.0, 0.0)],
            vec![RealImaginary(1.0, 0.0), RealImaginary(0.0, 0.0)],
        ]);

        let abcd = s.to_abcd(50.0);
        // A=1, B=0, C=0, D=1
        assert_eq!(abcd.get(1, 1).0, 1.0); // A real
        assert_eq!(abcd.get(1, 1).1, 0.0); // A imag
        assert_eq!(abcd.get(1, 2).0, 0.0); // B real
        assert_eq!(abcd.get(2, 1).0, 0.0); // C real
        assert_eq!(abcd.get(2, 2).0, 1.0); // D real

        let s_back = abcd.to_s(50.0);
        assert_eq!(s_back, s);
    }
}
