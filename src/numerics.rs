//! Dense complex solves shared by parameter conversions.

use crate::{ensure_non_singular_value, validate_matrix_data, Complex, TouchstoneError};

/// Opt-in diagnostics for the unscaled conversion solve `X D = N`.
///
/// These describe the solve before any final scalar unit conversion. They are
/// computed in f64, not certified bounds or a forward-accuracy guarantee.
/// Diagnostic variants return an error if a required norm or auxiliary solve is
/// nonfinite, even when the ordinary conversion succeeds. Underflow can report
/// zero reciprocal condition or backward error; zero is not a certificate.
///
/// ```
/// use touchstone::{Complex, SMatrix};
/// let s = SMatrix::try_new(vec![vec![Complex { re: 0.2, im: 0.0 }]])?;
/// let (z, report) = s.to_z_matrix_with_diagnostics(50.0)?;
/// assert!((z.get(1, 1)?.re - 75.0).abs() < 1e-12);
/// assert_eq!(report.reciprocal_condition_inf, 1.0);
/// # Ok::<(), touchstone::TouchstoneError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolveDiagnostics {
    /// `1 / (||D||_inf * ||D^-1||_inf)`, clamped to at most one.
    /// The inverse norm is accumulated by solving identity columns, costing
    /// another O(n^3) operations; it is not inferred from pivot magnitudes.
    pub reciprocal_condition_inf: f64,
    /// `||X D - N||_inf / (||X||_inf ||D||_inf + ||N||_inf)`.
    /// A zero numerator and denominator is reported as zero. Small backward
    /// error does not ensure small forward error when D is ill-conditioned.
    pub backward_error_inf: f64,
}

pub(crate) struct ComplexLu {
    factors: Vec<Vec<Complex>>,
    swaps: Vec<usize>,
}

impl ComplexLu {
    pub(crate) fn factor(
        mut matrix: Vec<Vec<Complex>>,
        operation: &str,
        tolerance: f64,
    ) -> Result<Self, TouchstoneError> {
        let rank = matrix.len();
        validate_matrix_data(operation, rank, &matrix)?;
        let mut swaps = Vec::with_capacity(rank);
        for k in 0..rank {
            let mut pivot = k;
            for i in k + 1..rank {
                if matrix[i][k].magnitude() > matrix[pivot][k].magnitude() {
                    pivot = i;
                }
            }
            matrix.swap(k, pivot);
            swaps.push(pivot);
            ensure_non_singular_value(operation, k, matrix[k][k], tolerance)?;
            for i in k + 1..rank {
                matrix[i][k] = matrix[i][k] / matrix[k][k];
                for j in k + 1..rank {
                    matrix[i][j] = matrix[i][j] - matrix[i][k] * matrix[k][j];
                }
            }
        }
        validate_matrix_data(operation, rank, &matrix)?;
        Ok(Self {
            factors: matrix,
            swaps,
        })
    }

    /// Solve A X = B for one or more RHS columns. A was factored as P A = L U.
    /// Crate-private callers supply rank rows with equal, nonzero column counts.
    pub(crate) fn solve(
        &self,
        mut rhs: Vec<Vec<Complex>>,
        operation: &str,
    ) -> Result<Vec<Vec<Complex>>, TouchstoneError> {
        let rank = self.factors.len();
        for (k, &pivot) in self.swaps.iter().enumerate() {
            rhs.swap(k, pivot);
        }
        for i in 0..rank {
            for k in 0..i {
                let (before, current) = rhs.split_at_mut(i);
                for (value, source) in current[0].iter_mut().zip(&before[k]) {
                    *value = *value - self.factors[i][k] * *source;
                }
            }
        }
        for i in (0..rank).rev() {
            for k in i + 1..rank {
                let (before, source) = rhs.split_at_mut(k);
                for (value, source) in before[i].iter_mut().zip(&source[0]) {
                    *value = *value - self.factors[i][k] * *source;
                }
            }
            for value in &mut rhs[i] {
                *value = *value / self.factors[i][i];
            }
        }
        // RHS may be rectangular (including a single identity column).
        for (i, row) in rhs.iter().enumerate() {
            for (j, value) in row.iter().enumerate() {
                if !value.is_finite() {
                    return Err(TouchstoneError::InvalidParameterMatrixValue {
                        matrix: operation.to_string(),
                        row: i + 1,
                        column: j + 1,
                        re: value.re,
                        im: value.im,
                    });
                }
            }
        }
        Ok(rhs)
    }
}

fn transpose(matrix: &[Vec<Complex>]) -> Vec<Vec<Complex>> {
    (0..matrix[0].len())
        .map(|j| matrix.iter().map(|row| row[j]).collect())
        .collect()
}

/// Solve X D = N via D^T X^T = N^T, never a conjugate transpose.
pub(crate) fn solve_right(
    denominator: Vec<Vec<Complex>>,
    numerator: Vec<Vec<Complex>>,
    operation: &str,
    tolerance: f64,
    diagnostics: bool,
) -> Result<(Vec<Vec<Complex>>, Option<SolveDiagnostics>), TouchstoneError> {
    let rank = denominator.len();
    validate_matrix_data(operation, rank, &denominator)?;
    validate_matrix_data(operation, rank, &numerator)?;
    let lu = ComplexLu::factor(transpose(&denominator), operation, tolerance)?;
    let x = lu.solve(transpose(&numerator), operation)?;
    let report = if diagnostics {
        Some(diagnose(
            &lu,
            &transpose(&denominator),
            &transpose(&numerator),
            &x,
            operation,
        )?)
    } else {
        None
    };
    Ok((transpose(&x), report))
}

fn norm_one(matrix: &[Vec<Complex>]) -> f64 {
    (0..matrix[0].len())
        .map(|j| matrix.iter().map(|row| row[j].magnitude()).sum::<f64>())
        .fold(0.0, f64::max)
}

fn checked_scalar(value: f64, operation: &str) -> Result<f64, TouchstoneError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(TouchstoneError::InvalidParameterMatrixValue {
            matrix: format!("{operation} diagnostics"),
            row: 1,
            column: 1,
            re: value,
            im: 0.0,
        })
    }
}

fn diagnose(
    lu: &ComplexLu,
    a: &[Vec<Complex>],
    b: &[Vec<Complex>],
    x: &[Vec<Complex>],
    operation: &str,
) -> Result<SolveDiagnostics, TouchstoneError> {
    let rank = a.len();
    let a_norm = checked_scalar(norm_one(a), operation)?;
    let mut inverse_norm: f64 = 0.0;
    for j in 0..rank {
        let mut column = vec![vec![Complex::zero()]; rank];
        column[j][0] = Complex::one();
        inverse_norm = inverse_norm.max(checked_scalar(
            norm_one(&lu.solve(column, operation)?),
            operation,
        )?);
    }
    let reciprocal_condition_inf = checked_scalar(1.0 / a_norm / inverse_norm, operation)?.min(1.0);
    // Normalize before multiplication, including the denominator of the residual,
    // so large but finite norms need not overflow when multiplied together.
    let x_norm = checked_scalar(norm_one(x), operation)?;
    let b_norm = checked_scalar(norm_one(b), operation)?;
    let backward_error_inf = if x_norm == 0.0 {
        if b_norm == 0.0 {
            0.0
        } else {
            1.0
        }
    } else {
        // Avoid forming the norm product. Recover via logs only if the first
        // division loses exponent range; the usual path avoids log rounding.
        let first = b_norm / a_norm.max(x_norm);
        let ratio = if b_norm != 0.0 && (first == 0.0 || !first.is_finite()) {
            (b_norm.ln() - a_norm.ln() - x_norm.ln()).exp()
        } else {
            first / a_norm.min(x_norm)
        };
        let (ax_weight, b_weight) = if ratio <= 1.0 {
            (1.0 / (1.0 + ratio), ratio / (1.0 + ratio))
        } else {
            let reciprocal = 1.0 / ratio;
            (reciprocal / (1.0 + reciprocal), 1.0 / (1.0 + reciprocal))
        };
        let mut residual = vec![vec![Complex::zero(); rank]; rank];
        for i in 0..rank {
            for j in 0..rank {
                let mut sum = Complex::zero();
                for k in 0..rank {
                    sum = sum + (a[i][k] / a_norm) * (x[k][j] / x_norm);
                }
                residual[i][j] = sum * ax_weight;
                if b_norm != 0.0 {
                    residual[i][j] = residual[i][j] - (b[i][j] / b_norm) * b_weight;
                }
            }
        }
        checked_scalar(norm_one(&residual) / (ax_weight + b_weight), operation)?
    };
    Ok(SolveDiagnostics {
        reciprocal_condition_inf,
        backward_error_inf,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noncommuting_complex_right_solve_and_multiple_left_rhs() {
        // Conversion polynomials commute, so public conversion cases alone
        // cannot detect an accidental left solve. This fixture also needs swaps.
        let c = |re, im| Complex { re, im };
        let d = vec![
            vec![c(0.0, 0.0), c(0.0, 1.0)],
            vec![c(2.0, 0.0), c(1.0, 0.0)],
        ];
        let expected = vec![
            vec![c(1.0, 0.0), c(0.0, 2.0)],
            vec![c(3.0, 1.0), c(4.0, 0.0)],
        ];
        let right_rhs = vec![
            vec![c(0.0, 4.0), c(0.0, 3.0)],
            vec![c(8.0, 0.0), c(3.0, 3.0)],
        ];
        let (actual, report) =
            solve_right(d.clone(), right_rhs, "right solve", 1e-12, true).unwrap();
        assert_eq!(actual, expected);
        assert!(report.unwrap().backward_error_inf < 1e-15);
        let left_rhs = vec![
            vec![c(-1.0, 3.0), c(0.0, 4.0)],
            vec![c(5.0, 1.0), c(4.0, 4.0)],
        ];
        let lu = ComplexLu::factor(d, "left solve", 1e-12).unwrap();
        assert_eq!(lu.solve(left_rhs, "left solve").unwrap(), expected);
    }
}
