//! Accuracy/compatibility experiment, not a statistically controlled benchmark.
use std::time::Instant;
use touchstone::{Complex as C, ParameterMatrix, SMatrix};

type Matrix = Vec<Vec<C>>;
fn c(re: f64, im: f64) -> C {
    C { re, im }
}
fn abs(z: C) -> f64 {
    z.re.hypot(z.im)
}
fn identity(n: usize) -> Matrix {
    let mut a = vec![vec![c(0.0, 0.0); n]; n];
    for (i, row) in a.iter_mut().enumerate() {
        row[i] = c(1.0, 0.0);
    }
    a
}
fn mul(a: &Matrix, b: &Matrix) -> Matrix {
    let n = a.len();
    let mut x = vec![vec![c(0.0, 0.0); n]; n];
    for i in 0..n {
        for j in 0..n {
            for (k, row) in b.iter().enumerate() {
                x[i][j] = x[i][j] + a[i][k] * row[j];
            }
        }
    }
    x
}
// Historical production Gauss-Jordan, retained only for this experiment.
fn baseline(mut a: Matrix, b: &Matrix) -> Option<Matrix> {
    let n = a.len();
    let mut inverse = identity(n);
    for k in 0..n {
        let mut pivot = k;
        for (i, row) in a.iter().enumerate().skip(k + 1) {
            if abs(row[k]) > abs(a[pivot][k]) {
                pivot = i;
            }
        }
        a.swap(k, pivot);
        inverse.swap(k, pivot);
        let p = a[k][k];
        if !abs(p).is_finite() || abs(p) <= 1e-12 {
            return None;
        }
        for j in 0..n {
            a[k][j] = a[k][j] / p;
            inverse[k][j] = inverse[k][j] / p;
        }
        let ar = a[k].clone();
        let ir = inverse[k].clone();
        for i in 0..n {
            if i == k {
                continue;
            }
            let factor = a[i][k];
            if abs(factor) == 0.0 {
                continue;
            }
            for j in 0..n {
                a[i][j] = a[i][j] - factor * ar[j];
                inverse[i][j] = inverse[i][j] - factor * ir[j];
            }
        }
    }
    let x = mul(b, &inverse);
    x.iter()
        .flatten()
        .all(|z| z.re.is_finite() && z.im.is_finite())
        .then_some(x)
}
fn affine(a: &Matrix, scalar: f64, diagonal: f64) -> Matrix {
    a.iter()
        .enumerate()
        .map(|(i, row)| {
            row.iter()
                .enumerate()
                .map(|(j, z)| *z * scalar + c(if i == j { diagonal } else { 0.0 }, 0.0))
                .collect()
        })
        .collect()
}
fn error(a: &Matrix, b: &Matrix) -> f64 {
    a.iter()
        .flatten()
        .zip(b.iter().flatten())
        .map(|(x, y)| abs(*x - *y))
        .fold(0.0, f64::max)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Dense rank-one perturbation: S=dI+c*u*v^T. Analytic Sherman-Morrison references; max absolute entry error.");
    for n in [2, 8, 32] {
        let d = 0.2;
        let coefficient = 0.15 / n as f64;
        let z0 = 50.0;
        let u: Vec<_> = (0..n).map(|i| c(1.0 + i as f64 / n as f64, 0.25)).collect();
        let v: Vec<_> = (0..n).map(|i| c(0.5 + i as f64 / n as f64, 0.0)).collect();
        let dot = u.iter().zip(&v).fold(c(0.0, 0.0), |s, (u, v)| s + *u * *v);
        let make = |diagonal: f64, coefficient: C| -> Matrix {
            (0..n)
                .map(|i| {
                    (0..n)
                        .map(|j| {
                            u[i] * v[j] * coefficient + c(if i == j { diagonal } else { 0.0 }, 0.0)
                        })
                        .collect()
                })
                .collect()
        };
        let s = make(d, c(coefficient, 0.0));
        let z = make(
            z0 * (1.0 + d) / (1.0 - d),
            c(2.0 * z0 * coefficient / (1.0 - d), 0.0) / (c(1.0 - d, 0.0) - dot * coefficient),
        );
        let y = make(
            (1.0 - d) / (1.0 + d) / z0,
            c(-2.0 * coefficient / (1.0 + d) / z0, 0.0) / (c(1.0 + d, 0.0) + dot * coefficient),
        );
        for mode in 0..4 {
            let (name, denominator, numerator, scale, expected) = match mode {
                0 => ("S->Z", affine(&s, -1.0, 1.0), affine(&s, 1.0, 1.0), z0, &z),
                1 => (
                    "S->Y",
                    affine(&s, 1.0, 1.0),
                    affine(&s, -1.0, 1.0),
                    1.0 / z0,
                    &y,
                ),
                2 => ("Y->S", affine(&y, z0, 1.0), affine(&y, -z0, 1.0), 1.0, &s),
                _ => ("Z->S", affine(&z, 1.0, z0), affine(&z, 1.0, -z0), 1.0, &s),
            };
            let start = Instant::now();
            let old = affine(
                &baseline(denominator.clone(), &numerator).expect("regular fixture"),
                scale,
                0.0,
            );
            let old_time = start.elapsed();
            let sm = SMatrix::try_new(s.clone())?;
            let ym = ParameterMatrix::try_new(y.clone())?;
            let zm = ParameterMatrix::try_new(z.clone())?;
            let plain = || -> Result<Matrix, touchstone::TouchstoneError> {
                Ok(match mode {
                    0 => sm.to_z_matrix(z0)?.data,
                    1 => sm.to_y_matrix(z0)?.data,
                    2 => ym.to_s_matrix_from_y(z0)?.data,
                    _ => zm.to_s_matrix_from_z(z0)?.data,
                })
            };
            let start = Instant::now();
            let new = plain()?;
            let new_time = start.elapsed();
            let start = Instant::now();
            let report = match mode {
                0 => sm.to_z_matrix_with_diagnostics(z0)?.1,
                1 => sm.to_y_matrix_with_diagnostics(z0)?.1,
                2 => ym.to_s_matrix_from_y_with_diagnostics(z0)?.1,
                _ => zm.to_s_matrix_from_z_with_diagnostics(z0)?.1,
            };
            let report_time = start.elapsed();
            println!("n={n:2} {name}: GJerr={:.3e} LUerr={:.3e} rcond={:.3e} berr={:.3e}; GJ={old_time:?} LU={new_time:?} LU+report={report_time:?}",error(&old,expected),error(&new,expected),report.reciprocal_condition_inf,report.backward_error_inf);
            assert!(error(&new, expected) < 1e-11);
        }
    }
    println!("Scale/cutoff probes: Z+z0I = scale*[[1,1],[2,1]], z0=scale/8; analytic S=[[1.25,-.25],[-.5,1.25]].");
    let expected = vec![
        vec![c(1.25, 0.0), c(-0.25, 0.0)],
        vec![c(-0.5, 0.0), c(1.25, 0.0)],
    ];
    for scale in [1e-13, 1e-12, 2e-12, 1.0, 1e150, 1e300] {
        let d = vec![
            vec![c(scale, 0.0), c(scale, 0.0)],
            vec![c(2.0 * scale, 0.0), c(scale, 0.0)],
        ];
        let z0 = scale / 8.0;
        let z = affine(&d, 1.0, -z0);
        let n = affine(&z, 1.0, -z0);
        let old = baseline(d, &n);
        let new = ParameterMatrix::try_new(z)?.to_s_matrix_from_z_with_diagnostics(z0);
        println!(
            "scale={scale:.1e}: GJ={:?}, LU={:?}",
            old.as_ref().map(|m| error(m, &expected)),
            new.as_ref().map(|(m, r)| (error(&m.data, &expected), r))
        );
    }
    let z0 = 1e-15;
    let d = vec![
        vec![c(1e-12, 0.0), c(1.0, 0.0)],
        vec![c(2e-12, 0.0), c(1.0, 0.0)],
    ];
    let z = affine(&d, 1.0, -z0);
    println!(
        "Threshold/order probe: GJ accepted={}, LU={:?}",
        baseline(d, &affine(&z, 1.0, -z0)).is_some(),
        ParameterMatrix::try_new(z)?.to_s_matrix_from_z(z0)
    );
    let s = SMatrix::try_new(vec![
        vec![c(1.0 - 1e-10, 0.0), c(0.0, 0.0)],
        vec![c(0.0, 0.0), c(0.0, 0.0)],
    ])?;
    println!(
        "Ill-conditioned S->Z: {:?}",
        s.to_z_matrix_with_diagnostics(50.0)?.1
    );
    Ok(())
}
