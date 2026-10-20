//! Run with `cargo run --offline --release --example nport_connection`.
//! Scalar denominator baseline is experimental only; production uses shared LU.
use std::{hint::black_box, time::Instant};
use touchstone::{Complex, Network, NetworkBuilder, SMatrix};

fn c(re: f64, im: f64) -> Complex {
    Complex { re, im }
}
fn network(name: &str, data: Vec<Vec<Complex>>) -> Network {
    NetworkBuilder::new(name, data.len())
        .point(1e9, SMatrix::try_new(data).unwrap())
        .build()
        .unwrap()
}
fn scalar(a: &SMatrix, b: &SMatrix) -> SMatrix {
    // Connect last port of a to first port of b; independent scalar composition.
    let la = a.rank - 1;
    let n = a.rank + b.rank - 2;
    let d = c(1.0, 0.0) - a.data[la][la] * b.data[0][0];
    let mut data = vec![vec![c(0.0, 0.0); n]; n];
    for (i, row) in data.iter_mut().enumerate() {
        for (j, value) in row.iter_mut().enumerate() {
            *value = match (i < la, j < la) {
                (true, true) => a.data[i][j] + a.data[i][la] * b.data[0][0] * a.data[la][j] / d,
                (true, false) => a.data[i][la] * b.data[0][j - la + 1] / d,
                (false, true) => b.data[i - la + 1][0] * a.data[la][j] / d,
                (false, false) => {
                    b.data[i - la + 1][j - la + 1]
                        + b.data[i - la + 1][0] * a.data[la][la] * b.data[0][j - la + 1] / d
                }
            };
        }
    }
    SMatrix::try_new(data).unwrap()
}
fn compare(a: &SMatrix, b: &SMatrix) -> f64 {
    let mut worst: f64 = 0.0;
    for (x, y) in a.data.iter().flatten().zip(b.data.iter().flatten()) {
        let error = (x.re - y.re).hypot(x.im - y.im);
        let scale = y.re.hypot(y.im).max(1e-300);
        assert!(error <= 3e-14 * scale, "{x:?} != {y:?}");
        worst = worst.max(error);
    }
    worst
}
fn main() {
    println!("2-port shared block solve vs existing direct cascade:");
    for t in [0.0, 0.8, 1e-100, 1e-150] {
        let a = network(
            "left",
            vec![
                vec![c(0.2, 0.1), c(t, 0.0)],
                vec![c(0.3, -0.1), c(-0.3, 0.2)],
            ],
        );
        let b = network(
            "right",
            vec![
                vec![c(0.4, -0.2), c(0.2, 0.1)],
                vec![c(t, 0.0), c(0.1, 0.2)],
            ],
        );
        let block = a.connect(&b, 2, 1).unwrap().network.s_matrix_at(0).unwrap();
        let direct = a.try_cascade(&b).unwrap().s_matrix_at(0).unwrap();
        println!(
            "  transmission {t:.1e}: max absolute error {:.3e}",
            compare(&block, &direct)
        );
    }
    println!("3-port analytic splitter termination:");
    let splitter = network(
        "splitter",
        (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| c(if i == j { -1.0 / 3.0 } else { 2.0 / 3.0 }, 0.0))
                    .collect()
            })
            .collect(),
    );
    for gamma in [c(0.0, 0.0), c(1.0, 0.0), c(0.2, 0.4)] {
        let load = network("load", vec![vec![gamma]]);
        let effective = splitter
            .connect(&load, 3, 1)
            .unwrap()
            .network
            .s_matrix_at(0)
            .unwrap();
        let correction = gamma * (4.0 / 9.0) / (c(1.0, 0.0) + gamma / 3.0);
        for i in 0..2 {
            for j in 0..2 {
                let expected = c(if i == j { -1.0 / 3.0 } else { 2.0 / 3.0 }, 0.0) + correction;
                let difference = effective.data[i][j] - expected;
                assert!(difference.re.hypot(difference.im) < 2e-15);
            }
        }
        println!("  Gamma {gamma:?}: analytic check passed");
    }
    println!("100 repeated single-frequency operations; means, no statistical controls:");
    for rank in [2, 8, 32] {
        let make = |name| {
            network(
                name,
                (0..rank)
                    .map(|i| {
                        (0..rank)
                            .map(|j| {
                                c(
                                    if i == j {
                                        0.1
                                    } else {
                                        0.02 / (1 + i + j) as f64
                                    },
                                    0.01 / (1 + i + 2 * j) as f64,
                                )
                            })
                            .collect()
                    })
                    .collect(),
            )
        };
        let a = make("a");
        let b = make("b");
        let am = a.s_matrix_at(0).unwrap();
        let bm = b.s_matrix_at(0).unwrap();
        let actual = a
            .connect(&b, rank, 1)
            .unwrap()
            .network
            .s_matrix_at(0)
            .unwrap();
        let error = compare(&actual, &scalar(&am, &bm));
        let start = Instant::now();
        for _ in 0..100 {
            black_box(a.connect(&b, rank, 1).unwrap());
        }
        let public = start.elapsed() / 100;
        let start = Instant::now();
        for _ in 0..100 {
            black_box(scalar(&am, &bm));
        }
        let baseline = start.elapsed() / 100;
        println!("  rank {rank}: public block {public:?}, matrix-only scalar {baseline:?}, max abs error {error:.3e}");
    }
    let resonant = network(
        "resonant",
        vec![
            vec![c(0.0, 0.0), c(1.0, 0.0)],
            vec![c(1.0, 0.0), c(1.0, 0.0)],
        ],
    );
    println!(
        "Singular open termination: {}",
        resonant
            .connect(&network("open", vec![vec![c(1.0, 0.0)]]), 2, 1)
            .unwrap_err()
    );
}
