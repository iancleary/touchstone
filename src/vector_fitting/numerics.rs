//! Small private real least-squares and sigma-zero helpers; unrelated to matrix LU.
use super::FitError;
use crate::Complex;

pub(super) struct Solution {
    pub coefficients: Vec<f64>,
    pub pivot_ratio: f64,
}

fn norm(values: impl Iterator<Item = f64>) -> f64 {
    values.fold(0.0, f64::hypot)
}

/// Column equilibration followed by Householder QR with recomputed column pivots.
/// The diagonal ratio is an indicator, not an estimate of the condition number.
pub(super) fn least_squares(
    mut a: Vec<f64>,
    mut b: Vec<f64>,
    n: usize,
) -> Result<Solution, FitError> {
    let m = b.len();
    let mut scales = vec![0.0; n];
    for j in 0..n {
        scales[j] = norm((0..m).map(|i| a[i * n + j]));
        if !scales[j].is_finite() || scales[j] == 0.0 {
            return Err(FitError::RankDeficient {
                rank: 0,
                columns: n,
                pivot: 0.0,
            });
        }
        for i in 0..m {
            a[i * n + j] /= scales[j];
        }
    }
    let mut permutation: Vec<_> = (0..n).collect();
    let mut min_pivot: f64 = 1.0;
    let mut max_pivot: f64 = 0.0;
    for k in 0..n {
        let mut pivot = k;
        let mut length = 0.0;
        for j in k..n {
            let candidate = norm((k..m).map(|i| a[i * n + j]));
            if candidate > length {
                length = candidate;
                pivot = j;
            }
        }
        if !length.is_finite() || length <= 1e-11 {
            return Err(FitError::RankDeficient {
                rank: k,
                columns: n,
                pivot: length,
            });
        }
        min_pivot = min_pivot.min(length);
        max_pivot = max_pivot.max(length);
        permutation.swap(k, pivot);
        for i in 0..m {
            a.swap(i * n + k, i * n + pivot);
        }
        let alpha = -length.copysign(a[k * n + k]);
        let mut v: Vec<_> = (k..m).map(|i| a[i * n + k]).collect();
        v[0] -= alpha;
        let vnorm = norm(v.iter().copied());
        for value in &mut v {
            *value /= vnorm;
        }
        for j in k..n {
            let projection = 2.0 * (k..m).map(|i| v[i - k] * a[i * n + j]).sum::<f64>();
            for i in k..m {
                a[i * n + j] -= projection * v[i - k];
            }
        }
        let projection = 2.0 * (k..m).map(|i| v[i - k] * b[i]).sum::<f64>();
        for i in k..m {
            b[i] -= projection * v[i - k];
        }
        a[k * n + k] = alpha;
        for i in k + 1..m {
            a[i * n + k] = 0.0;
        }
    }
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let tail: f64 = (i + 1..n).map(|j| a[i * n + j] * x[j]).sum();
        x[i] = (b[i] - tail) / a[i * n + i];
    }
    let mut coefficients = vec![0.0; n];
    for i in 0..n {
        coefficients[permutation[i]] = x[i] / scales[permutation[i]];
    }
    if coefficients.iter().any(|v| !v.is_finite()) {
        return Err(FitError::Numerical);
    }
    Ok(Solution {
        coefficients,
        pivot_ratio: min_pivot / max_pivot,
    })
}

/// Real monic quadratic, scaled before the discriminant; q/C avoids small-root cancellation.
pub(super) fn quadratic(b: f64, c: f64) -> Result<Vec<Complex>, FitError> {
    if !b.is_finite() || !c.is_finite() {
        return Err(FitError::Numerical);
    }
    let scale = b.abs().max(c.abs().sqrt()).max(1e-150);
    let b = b / scale;
    let c = (c / scale) / scale;
    let discriminant = b * b - 4.0 * c;
    if discriminant < 0.0 {
        let re = -0.5 * b * scale;
        let im = 0.5 * (-discriminant).sqrt() * scale;
        Ok(vec![Complex { re, im }, Complex { re, im: -im }])
    } else {
        let q = -0.5 * (b + discriminant.sqrt().copysign(b));
        if q == 0.0 {
            return Err(FitError::InvalidPoles);
        }
        Ok(vec![
            Complex {
                re: q * scale,
                im: 0.0,
            },
            Complex {
                re: (c / q) * scale,
                im: 0.0,
            },
        ])
    }
}
