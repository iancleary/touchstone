//! Bounded f64 experiment for two-port cascades. Run with
//! `cargo run --example abcd_numerics` and `cargo test --example abcd_numerics`.
//! This is an investigation, not a proposed production implementation.

use touchstone::{Complex as C, NetworkBuilder, SMatrix as S};

const Z0: f64 = 50.0;

fn c(re: f64, im: f64) -> C {
    C { re, im }
}

fn one() -> C {
    c(1.0, 0.0)
}

fn abs(x: C) -> f64 {
    x.re.hypot(x.im)
}

fn sm(s11: C, s12: C, s21: C, s22: C) -> S {
    S {
        rank: 2,
        data: vec![vec![s11, s12], vec![s21, s22]],
    }
}

fn matched(db: f64) -> S {
    let t = 10f64.powf(-db / 20.0);
    sm(c(0.0, 0.0), c(t, 0.0), c(t, 0.0), c(0.0, 0.0))
}

fn entry(s: &S, row: usize, col: usize) -> C {
    s.data[row][col]
}

// The public ABCD path under examination. Each adjacent pair uses the public
// conversion on both operands and the published ABCD->S conversion.
fn public_pair(a: &S, b: &S) -> Result<S, String> {
    let a = a.to_abcd(Z0).map_err(|e| e.to_string())?;
    let b = b.to_abcd(Z0).map_err(|e| e.to_string())?;
    let product = touchstone::ABCDMatrix {
        a: a.a * b.a + a.b * b.c,
        b: a.a * b.b + a.b * b.d,
        c: a.c * b.a + a.d * b.c,
        d: a.c * b.b + a.d * b.d,
    };
    product.to_s_matrix(Z0).map_err(|e| e.to_string())
}

fn public_roundtrip(s: &S) -> Result<S, String> {
    s.to_abcd(Z0)
        .and_then(|abcd| abcd.to_s_matrix(Z0))
        .map_err(|e| e.to_string())
}

// Network::cascade uses the crate's older internal matrix path, which differs
// from the public SMatrix conversion above.
fn network_pair(a: &S, b: &S) -> Result<S, String> {
    let a = NetworkBuilder::new("a.s2p", 2)
        .point(1.0e9, a.clone())
        .build()
        .map_err(|e| e.to_string())?;
    let b = NetworkBuilder::new("b.s2p", 2)
        .point(1.0e9, b.clone())
        .build()
        .map_err(|e| e.to_string())?;
    a.cascade(&b).s_matrix_at(0).map_err(|e| e.to_string())
}

// ABCD numerators over a shared denominator. This defers division during
// conversion and multiplication. The determinant is carried from S12/S21,
// then multiplied for cascades, so S12 never subtracts A*D - B*C. There is
// no scaling or normalization of the shared denominator; the tested range is
// bounded. This candidate requires nonzero S21 and finite intermediates.
#[derive(Clone, Copy)]
struct Tracked {
    a: C,
    b: C,
    c: C,
    d: C,
    q: C,
    det: C,
}

impl Tracked {
    fn from_s(s: &S) -> Option<Self> {
        let (s11, s12, s21, s22) = (
            entry(s, 0, 0),
            entry(s, 0, 1),
            entry(s, 1, 0),
            entry(s, 1, 1),
        );
        if abs(s21) == 0.0 {
            return None;
        }
        let o = one();
        Some(Self {
            a: (o + s11) * (o - s22) + s12 * s21,
            b: ((o + s11) * (o + s22) - s12 * s21) * Z0,
            c: ((o - s11) * (o - s22) - s12 * s21) / Z0,
            d: (o - s11) * (o + s22) + s12 * s21,
            q: s21 * 2.0,
            det: s12 / s21,
        })
    }

    fn then(self, rhs: Self) -> Self {
        Self {
            a: self.a * rhs.a + self.b * rhs.c,
            b: self.a * rhs.b + self.b * rhs.d,
            c: self.c * rhs.a + self.d * rhs.c,
            d: self.c * rhs.b + self.d * rhs.d,
            q: self.q * rhs.q,
            det: self.det * rhs.det,
        }
    }

    fn to_s(self) -> Option<S> {
        let den = self.a + self.b / Z0 + self.c * Z0 + self.d;
        if abs(den) == 0.0 {
            return None;
        }
        let s21 = self.q * 2.0 / den;
        let result = sm(
            (self.a + self.b / Z0 - self.c * Z0 - self.d) / den,
            self.det * s21,
            s21,
            (-self.a + self.b / Z0 - self.c * Z0 + self.d) / den,
        );
        result
            .data
            .iter()
            .flatten()
            .all(|x| x.re.is_finite() && x.im.is_finite())
            .then_some(result)
    }
}

// Redheffer two-port composition directly in scattering variables.
fn scattering_pair(a: &S, b: &S) -> Option<S> {
    let d = one() - entry(a, 1, 1) * entry(b, 0, 0);
    if abs(d) == 0.0 {
        return None;
    }
    let result = sm(
        entry(a, 0, 0) + entry(a, 0, 1) * entry(b, 0, 0) * entry(a, 1, 0) / d,
        entry(a, 0, 1) * entry(b, 0, 1) / d,
        entry(b, 1, 0) * entry(a, 1, 0) / d,
        entry(b, 1, 1) + entry(b, 1, 0) * entry(a, 1, 1) * entry(b, 0, 1) / d,
    );
    result
        .data
        .iter()
        .flatten()
        .all(|x| x.re.is_finite() && x.im.is_finite())
        .then_some(result)
}

// Independent internal-wave reference: solve the two connection equations
//   x - B11*y = B12*v,  -A22*x + y = A21*u
// by Gaussian elimination with partial pivoting for each incident basis wave.
fn internal_wave_pair(a: &S, b: &S) -> Option<S> {
    fn solve(a: &S, b: &S, u: C, v: C) -> Option<(C, C)> {
        let mut m = [
            [one(), -entry(b, 0, 0), entry(b, 0, 1) * v],
            [-entry(a, 1, 1), one(), entry(a, 1, 0) * u],
        ];
        if abs(m[1][0]) > abs(m[0][0]) {
            m.swap(0, 1);
        }
        if abs(m[0][0]) == 0.0 {
            return None;
        }
        let factor = m[1][0] / m[0][0];
        m[1][1] = m[1][1] - factor * m[0][1];
        m[1][2] = m[1][2] - factor * m[0][2];
        if abs(m[1][1]) == 0.0 {
            return None;
        }
        let y = m[1][2] / m[1][1];
        let x = (m[0][2] - m[0][1] * y) / m[0][0];
        Some((x, y))
    }
    let (x1, y1) = solve(a, b, one(), c(0.0, 0.0))?;
    let (x2, y2) = solve(a, b, c(0.0, 0.0), one())?;
    Some(sm(
        entry(a, 0, 0) + entry(a, 0, 1) * x1,
        entry(a, 0, 1) * x2,
        entry(b, 1, 0) * y1,
        entry(b, 1, 0) * y2 + entry(b, 1, 1),
    ))
}

fn relative(actual: C, expected: C) -> f64 {
    abs(actual - expected) / abs(expected).max(f64::MIN_POSITIVE)
}

fn max_error(actual: &S, expected: &S) -> (f64, f64) {
    if actual
        .data
        .iter()
        .chain(expected.data.iter())
        .flatten()
        .any(|x| !x.re.is_finite() || !x.im.is_finite())
    {
        return (f64::INFINITY, f64::INFINITY);
    }
    let reflection = [(0, 0), (1, 1)]
        .into_iter()
        .map(|(i, j)| abs(entry(actual, i, j) - entry(expected, i, j)))
        .fold(0.0, f64::max);
    let transmission = [(0, 1), (1, 0)]
        .into_iter()
        .map(|(i, j)| relative(entry(actual, i, j), entry(expected, i, j)))
        .fold(0.0, f64::max);
    (reflection, transmission)
}

#[cfg(test)]
fn assert_close(
    label: &str,
    actual: &S,
    expected: &S,
    reflection_limit: f64,
    transmission_limit: f64,
) {
    let (r, t) = max_error(actual, expected);
    assert!(
        r <= reflection_limit && t <= transmission_limit,
        "{label}: reflection abs={r:e}, transmission rel={t:e}"
    );
}

fn report(label: &str, candidate: Option<&S>, expected: &S) {
    match candidate {
        Some(s) => {
            let (r, t) = max_error(s, expected);
            println!(
                "{label:<21} reflection abs={r:.3e}  transmission rel={t:.3e}  S12={:.3e}{:+.3e}i",
                entry(s, 0, 1).re,
                entry(s, 0, 1).im
            );
        }
        None => println!("{label:<21} unavailable"),
    }
}

fn complex_cases() -> [(S, S); 2] {
    [
        (
            sm(c(0.12, 0.08), c(0.31, -0.07), c(0.42, 0.09), c(-0.16, 0.04)),
            sm(
                c(-0.21, 0.11),
                c(0.22, 0.18),
                c(0.53, -0.13),
                c(0.09, -0.17),
            ),
        ),
        // Internal loop gain is 0.999999; the cascade is close to singular.
        (
            sm(
                c(0.02, 0.0),
                c(0.001, 0.002),
                c(0.003, -0.001),
                c(0.999999, 0.0),
            ),
            sm(
                c(1.0, 0.0),
                c(0.004, 0.001),
                c(0.002, -0.003),
                c(-0.03, 0.0),
            ),
        ),
    ]
}

fn run() {
    println!("f64 two-port numerical probe; errors: reflection absolute, S12/S21 relative");
    for db in [0.0, 20.0, 60.0, 90.0, 120.0, 180.0, 240.0] {
        let expected = matched(db);
        println!(
            "\nmatched {db:.0} dB roundtrip, expected |S12|={:.3e}",
            abs(entry(&expected, 0, 1))
        );
        report(
            "public S->ABCD->S",
            public_roundtrip(&expected).as_ref().ok(),
            &expected,
        );
        report(
            "tracked determinant",
            Tracked::from_s(&expected).and_then(Tracked::to_s).as_ref(),
            &expected,
        );
        let half = matched(db / 2.0);
        println!(
            "matched {db:.0} dB as two {half_db:.0} dB stages",
            half_db = db / 2.0
        );
        report(
            "public ABCD pair",
            public_pair(&half, &half).as_ref().ok(),
            &expected,
        );
        report(
            "tracked ABCD pair",
            Tracked::from_s(&half)
                .zip(Tracked::from_s(&half))
                .and_then(|(a, b)| a.then(b).to_s())
                .as_ref(),
            &expected,
        );
        report(
            "scattering pair",
            scattering_pair(&half, &half).as_ref(),
            &expected,
        );
        if db == 180.0 || db == 240.0 {
            report(
                "Network::cascade",
                network_pair(&half, &half).as_ref().ok(),
                &expected,
            );
        }
    }

    println!("\n260 dB boundary observation (outside the 0..240 dB target)");
    let boundary = matched(260.0);
    let boundary_result = public_roundtrip(&boundary);
    report(
        "public S->ABCD->S",
        boundary_result.as_ref().ok(),
        &boundary,
    );
    if let Err(error) = boundary_result {
        println!("  public error: {error}");
    }

    let stage = matched(20.0);
    let expected = matched(240.0);
    let mut public = stage.clone();
    let mut tracked = Tracked::from_s(&stage).unwrap();
    let mut scattering = stage.clone();
    let mut public_error = None;
    for _ in 1..12 {
        if public_error.is_none() {
            match public_pair(&public, &stage) {
                Ok(next) => public = next,
                Err(e) => public_error = Some(e),
            }
        }
        tracked = tracked.then(Tracked::from_s(&stage).unwrap());
        scattering = scattering_pair(&scattering, &stage).unwrap();
    }
    println!("\n12 x 20 dB matched chain (240 dB total)");
    report(
        "public ABCD chain",
        public_error.is_none().then_some(&public),
        &expected,
    );
    if let Some(e) = public_error {
        println!("  public error: {e}");
    }
    report("tracked ABCD chain", tracked.to_s().as_ref(), &expected);
    report("scattering chain", Some(&scattering), &expected);

    for (index, (a, b)) in complex_cases().into_iter().enumerate() {
        let reference = internal_wave_pair(&a, &b).unwrap();
        println!(
            "\ncomplex case {}: independent internal-wave f64 solve",
            index + 1
        );
        report(
            "public ABCD pair",
            public_pair(&a, &b).as_ref().ok(),
            &reference,
        );
        report(
            "tracked ABCD pair",
            Tracked::from_s(&a)
                .zip(Tracked::from_s(&b))
                .and_then(|(x, y)| x.then(y).to_s())
                .as_ref(),
            &reference,
        );
        report(
            "scattering pair",
            scattering_pair(&a, &b).as_ref(),
            &reference,
        );
    }

    let blocked = sm(c(0.2, 0.0), c(0.3, 0.1), c(0.0, 0.0), c(0.1, 0.0));
    let b = complex_cases()[0].1.clone();
    let reference = internal_wave_pair(&blocked, &b).unwrap();
    println!("\nexact S21=0 in first stage");
    report(
        "public ABCD pair",
        public_pair(&blocked, &b).as_ref().ok(),
        &reference,
    );
    report(
        "tracked ABCD pair",
        Tracked::from_s(&blocked)
            .zip(Tracked::from_s(&b))
            .and_then(|(x, y)| x.then(y).to_s())
            .as_ref(),
        &reference,
    );
    report(
        "scattering pair",
        scattering_pair(&blocked, &b).as_ref(),
        &reference,
    );
    let legacy_zero = network_pair(&blocked, &b);
    report("Network::cascade", legacy_zero.as_ref().ok(), &reference);
    if let Err(error) = legacy_zero {
        println!("  Network::cascade readback error: {error}");
    }
    println!("\nLimitations: all references and candidates use f64; no high-precision oracle or performance measurement. Tracked ABCD needs nonzero S21 and finite intermediates. Direct scattering needs a nonsingular internal connection.");
}

fn main() {
    run();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matched_attenuation_uses_relative_transmission_error() {
        for db in [0.0, 20.0, 60.0, 90.0, 120.0, 180.0, 240.0] {
            let expected = matched(db);
            let half = matched(db / 2.0);
            let tracked = Tracked::from_s(&half)
                .unwrap()
                .then(Tracked::from_s(&half).unwrap())
                .to_s()
                .unwrap();
            let direct = scattering_pair(&half, &half).unwrap();
            assert_close("tracked pair", &tracked, &expected, 1e-12, 2e-12);
            assert_close("scattering pair", &direct, &expected, 1e-12, 2e-12);
            assert_close(
                "tracked roundtrip",
                &Tracked::from_s(&expected).unwrap().to_s().unwrap(),
                &expected,
                1e-12,
                2e-12,
            );
        }
    }

    #[test]
    fn long_chain_retains_small_transmission() {
        let stage = matched(20.0);
        let mut tracked = Tracked::from_s(&stage).unwrap();
        let mut direct = stage.clone();
        for _ in 1..12 {
            tracked = tracked.then(Tracked::from_s(&stage).unwrap());
            direct = scattering_pair(&direct, &stage).unwrap();
        }
        assert_close(
            "tracked chain",
            &tracked.to_s().unwrap(),
            &matched(240.0),
            1e-12,
            5e-12,
        );
        assert_close("scattering chain", &direct, &matched(240.0), 1e-12, 5e-12);
    }

    #[test]
    fn complex_and_near_singular_against_internal_wave_solve() {
        for (a, b) in complex_cases() {
            let reference = internal_wave_pair(&a, &b).unwrap();
            assert_close(
                "tracked",
                &Tracked::from_s(&a)
                    .unwrap()
                    .then(Tracked::from_s(&b).unwrap())
                    .to_s()
                    .unwrap(),
                &reference,
                5e-10,
                5e-10,
            );
            assert_close(
                "scattering",
                &scattering_pair(&a, &b).unwrap(),
                &reference,
                5e-10,
                5e-10,
            );
        }
    }

    #[test]
    fn zero_forward_transmission_uses_scattering() {
        let a = sm(c(0.2, 0.0), c(0.3, 0.1), c(0.0, 0.0), c(0.1, 0.0));
        let b = complex_cases()[0].1.clone();
        assert!(Tracked::from_s(&a).is_none());
        assert!(a.to_abcd(Z0).is_err());
        assert_close(
            "zero S21",
            &scattering_pair(&a, &b).unwrap(),
            &internal_wave_pair(&a, &b).unwrap(),
            1e-12,
            1e-12,
        );
    }
}
