//! Compare normalized summation accuracy and lazy traversal with caller-owned caching.
//! Run: cargo run --offline --release --example mixed_mode
use std::{error::Error, hint::black_box, time::Instant};
use touchstone::{
    Complex, MixedModeLayout, MixedModeView, ModePort, NetworkBuilder, PortPair, SMatrix,
};

fn main() -> Result<(), Box<dyn Error>> {
    compare_summation()?;
    const POINTS: usize = 2_000;
    const PASSES: usize = 20;
    let zero = Complex { re: 0.0, im: 0.0 };
    let a = PortPair {
        positive: 1,
        negative: 2,
    };
    let b = PortPair {
        positive: 3,
        negative: 4,
    };
    let layout = MixedModeLayout::new(
        4,
        vec![
            ModePort::Differential(a),
            ModePort::Differential(b),
            ModePort::Common(a),
            ModePort::Common(b),
        ],
    )?;
    let mut builder = NetworkBuilder::new("example.s4p", 4);
    for index in 0..POINTS {
        let phase = index as f64 / POINTS as f64;
        let mut data = vec![vec![zero; 4]; 4];
        data[2][0] = Complex {
            re: 0.8 * phase.cos(),
            im: 0.8 * phase.sin(),
        };
        data[3][1] = Complex { re: 0.4, im: -0.6 };
        data[0][2] = data[2][0];
        data[1][3] = data[3][1];
        builder.push_point(index as f64 * 1e6, SMatrix::try_new(data)?);
    }
    let source = builder.build()?;
    let start = Instant::now();
    let view = MixedModeView::new(&source, layout)?;
    let validation = start.elapsed();

    // Instrument transform counts at the actual public read boundary, not in
    // production. Payload estimates below exclude Vec headers/allocator overhead.
    let mut cache_transforms = 0;
    let start = Instant::now();
    let cache = view
        .iter_matrices()
        .inspect(|_| {
            cache_transforms += 1;
        })
        .collect::<Result<Vec<_>, _>>()?;
    let cache_build = start.elapsed();
    let mut lazy_transforms = 0;
    let mut lazy_sum = 0.0;
    let start = Instant::now();
    for _ in 0..PASSES {
        for matrix in view.iter_matrices() {
            lazy_transforms += 1;
            lazy_sum += checksum(black_box(&matrix?));
        }
    }
    let lazy_time = start.elapsed();
    let mut cached_sum = 0.0;
    let start = Instant::now();
    for _ in 0..PASSES {
        for matrix in &cache {
            cached_sum += checksum(black_box(matrix));
        }
    }
    let cached_time = start.elapsed();
    assert_eq!(lazy_sum, cached_sum);
    // Compare entire actual public matrices, not just a checksum or a round trip.
    for _ in 0..3 {
        for (actual, expected) in view.iter_matrices().zip(&cache) {
            assert_eq!(&actual?, expected);
        }
    }
    println!("{POINTS} points, rank 4, {PASSES} measured traversals");
    println!("source validation: {validation:?}");
    println!("lazy: {lazy_time:?}, {lazy_transforms} transforms");
    println!("cache build: {cache_build:?}, {cache_transforms} transforms; cached reads: {cached_time:?}");
    println!(
        "retained transformed complex payload: lazy 0 bytes; cache {} bytes",
        POINTS * 4 * 4 * std::mem::size_of::<Complex>()
    );
    println!("full matrix equality: 3 additional traversals passed (outside timings/counts)");
    println!(
        "workload-specific comparison, not a universal performance claim; checksum={lazy_sum}"
    );
    Ok(())
}

// Experiment-only baseline and candidate: no alternative production view type.
fn normalized_sum(values: &[f64], weight: f64, compensated: bool) -> f64 {
    let scale = values.iter().map(|x| x.abs()).fold(0.0, f64::max);
    if scale == 0.0 {
        return 0.0;
    }
    let mut sum: f64 = 0.0;
    let mut correction = 0.0;
    for value in values {
        let term = (value / scale) * weight;
        let next = sum + term;
        if compensated {
            correction += if sum.abs() >= term.abs() {
                (sum - next) + term
            } else {
                (term - next) + sum
            };
        }
        sum = next;
    }
    (sum + correction) * scale
}

fn compare_summation() -> Result<(), Box<dyn Error>> {
    // Analytic expectations, including the known limitation, not an f64 oracle.
    let cases: &[(&str, &[f64], f64, f64)] = &[
        ("one term", &[0.75], 1.0, 0.75),
        ("two terms", &[0.8, 0.4], 0.5, 0.6),
        ("three terms", &[0.5, -0.25, 0.75], 0.5, 0.5),
        ("four terms", &[0.5, -0.25, 0.75, 1.0], 0.5, 1.0),
        ("review cancellation", &[1e300, 1.0, -1e300, 1.0], 0.5, 1.0),
        (
            "max cancellation",
            &[f64::MAX, f64::MAX, f64::MAX, -f64::MAX],
            0.5,
            f64::MAX,
        ),
        ("overflow", &[f64::MAX; 4], 0.5, f64::INFINITY),
        (
            "scaling underflow",
            &[1e300, 1e-100, -1e300, 1e-100],
            0.5,
            1e-100,
        ),
    ];
    println!("normalized ordinary vs Neumaier (up to four terms):");
    for &(name, values, weight, expected) in cases {
        let ordinary = normalized_sum(values, weight, false);
        let compensated = normalized_sum(values, weight, true);
        println!(
            "{name}: ordinary={ordinary:e}, compensated={compensated:e}, analytic={expected:e}"
        );
        if name == "scaling underflow" {
            assert_eq!((ordinary, compensated), (0.0, 0.0));
        } else if expected.is_infinite() {
            assert_eq!(ordinary, expected);
            assert_eq!(compensated, expected);
        } else {
            assert!((compensated - expected).abs() <= expected.abs() * 1e-14);
            if name == "review cancellation" {
                assert_eq!(ordinary, 0.5);
                assert_eq!(compensated, 1.0);
            } else {
                assert_eq!(ordinary, compensated);
            }
        }
    }
    let source = NetworkBuilder::new("cancellation.s2p", 2)
        .point(
            0.0,
            SMatrix::try_new(vec![
                vec![Complex { re: 1e300, im: 0.0 }, Complex { re: 1.0, im: 0.0 }],
                vec![
                    Complex {
                        re: -1e300,
                        im: 0.0,
                    },
                    Complex { re: 1.0, im: 0.0 },
                ],
            ])?,
        )
        .build()?;
    let pair = PortPair {
        positive: 1,
        negative: 2,
    };
    let view = MixedModeView::new(
        &source,
        MixedModeLayout::new(
            2,
            vec![ModePort::Differential(pair), ModePort::Common(pair)],
        )?,
    )?;
    assert_eq!(view.s_matrix_at(0)?.get(2, 2)?.re, 1.0);
    println!("public Scc regression: 1; scaling-underflow case remains unresolved");
    Ok(())
}

fn checksum(matrix: &SMatrix) -> f64 {
    matrix.data.iter().flatten().map(|z| z.re + z.im).sum()
}
