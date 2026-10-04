use crate::data_line::ParsedDataLine;
use crate::data_pairs::RealImaginaryMatrix;
use crate::{
    ABCDMatrix, Complex, Network, NetworkPoint, ParameterMatrix, SMatrix, TouchstoneError,
};

impl SMatrix {
    /// Construct a nonempty square S matrix with finite complex values.
    ///
    /// The rank is inferred from the number of rows. Rows are destination ports;
    /// columns are source ports. Both port indexes are 1-based in [`Self::get`].
    ///
    /// ```
    /// use touchstone::{Complex, SMatrix};
    /// let matrix = SMatrix::try_new(vec![vec![Complex { re: 0.5, im: 0.0 }]])?;
    /// assert_eq!(matrix.rank, 1);
    /// # Ok::<(), touchstone::TouchstoneError>(())
    /// ```
    pub fn try_new(data: Vec<Vec<Complex>>) -> Result<Self, TouchstoneError> {
        let rank = data.len();
        crate::validate_matrix_data("S", rank, &data)?;
        Ok(Self { rank, data })
    }
}

impl ParameterMatrix {
    /// Construct a nonempty square parameter matrix with finite complex values.
    ///
    /// The rank is inferred from the rows. Select Y or Z interpretation with
    /// the corresponding conversion method.
    pub fn try_new(data: Vec<Vec<Complex>>) -> Result<Self, TouchstoneError> {
        let rank = data.len();
        crate::validate_matrix_data("parameter", rank, &data)?;
        Ok(Self { rank, data })
    }
}

impl ABCDMatrix {
    /// Construct an ABCD matrix with finite complex entries.
    ///
    /// Conversion to S parameters can still fail for a singular denominator.
    pub fn try_new(
        a: Complex,
        b: Complex,
        c: Complex,
        d: Complex,
    ) -> Result<Self, TouchstoneError> {
        let matrix = Self { a, b, c, d };
        crate::validate_abcd_values(&matrix)?;
        Ok(matrix)
    }
}

impl NetworkPoint {
    /// Construct a finite S matrix at a finite, nonnegative frequency in Hz.
    ///
    /// A [`crate::NetworkBuilder`] also checks the order of the complete grid.
    pub fn try_new(frequency: f64, s: SMatrix) -> Result<Self, TouchstoneError> {
        crate::validate_sample_frequency(0, frequency)?;
        crate::validate_matrix_data("S", s.rank, &s.data)?;
        Ok(Self { frequency, s })
    }
}

/// A borrowed, read-only S matrix stored in a [`Network`].
///
/// Reading this view does not allocate or copy the matrix. Use
/// [`Network::s_matrix_at`] when an owned, editable matrix is needed.
#[derive(Debug, Clone, Copy)]
pub struct SMatrixRef<'a> {
    matrix: &'a RealImaginaryMatrix,
}

impl SMatrixRef<'_> {
    /// Return the number of ports.
    #[must_use]
    pub fn rank(&self) -> usize {
        self.matrix.size()
    }

    /// Read S(to_port, from_port) with checked 1-based RF port indexes.
    pub fn get(&self, to_port: usize, from_port: usize) -> Result<Complex, TouchstoneError> {
        crate::validate_port_indexes(to_port, from_port, self.rank())?;
        Ok(crate::complex_from_real_imaginary(
            self.matrix.get(to_port, from_port),
        ))
    }
}

/// A borrowed, read-only frequency point stored in a [`Network`].
///
/// The frequency is in Hz. This view borrows the cached S matrix without
/// exposing the parser's storage or allowing independent edits to RI/MA/DB.
#[derive(Debug, Clone, Copy)]
pub struct NetworkPointRef<'a> {
    point: &'a ParsedDataLine,
}

impl<'a> NetworkPointRef<'a> {
    /// Return the frequency in Hz.
    #[must_use]
    pub fn frequency(&self) -> f64 {
        self.point.frequency
    }

    /// Borrow the S matrix at this frequency.
    #[must_use]
    pub fn s(&self) -> SMatrixRef<'a> {
        SMatrixRef {
            matrix: &self.point.s_ri,
        }
    }
}

impl Network {
    /// Borrow the frequency grid in Hz without allocating.
    ///
    /// Use [`Self::f`] for an owned copy. The stored grid cannot be edited
    /// independently of the network's frequency points.
    ///
    /// ```compile_fail,E0616
    /// let mut network = touchstone::Network::from_str("dc.s1p", "# Hz S RI R 50\n0 0 0\n").unwrap();
    /// network.f.clear();
    /// ```
    #[must_use]
    pub fn frequencies(&self) -> &[f64] {
        &self.f
    }

    /// Borrow a frequency point by its 0-based index without copying matrices.
    pub fn point_ref(&self, point_index: usize) -> Result<NetworkPointRef<'_>, TouchstoneError> {
        Ok(NetworkPointRef {
            point: self.data_line_at(point_index)?,
        })
    }

    /// Iterate over borrowed frequency points without allocating.
    ///
    /// ```
    /// let network = touchstone::Network::from_str("example.s1p", "# Hz S RI R 50\n1 0.5 0\n")?;
    /// for (frequency, point) in network.frequencies().iter().zip(network.iter_points()) {
    ///     assert_eq!(*frequency, point.frequency());
    ///     assert_eq!(point.s().get(1, 1)?.re, 0.5);
    /// }
    /// # Ok::<(), touchstone::TouchstoneError>(())
    /// ```
    pub fn iter_points(
        &self,
    ) -> impl ExactSizeIterator<Item = NetworkPointRef<'_>> + DoubleEndedIterator + '_ {
        self.s.iter().map(|point| NetworkPointRef { point })
    }
}
