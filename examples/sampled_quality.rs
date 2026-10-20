//! Run the real public API and compare two bounded analytic singular-value formulas.
//! `cargo run --offline --example sampled_quality [optional.s2p]`

use touchstone::{Complex, Network, NetworkBuilder, SMatrix, SampledQualityOptions};

fn alternate_sigma(matrix: &SMatrix) -> f64 {
    let scale = matrix
        .data
        .iter()
        .flatten()
        .fold(0.0_f64, |s, v| s.max(v.re.abs()).max(v.im.abs()));
    if scale == 0.0 {
        return 0.0;
    }
    let v: Vec<_> = matrix
        .data
        .iter()
        .flatten()
        .map(|v| Complex {
            re: v.re / scale,
            im: v.im / scale,
        })
        .collect();
    let trace: f64 = v.iter().map(|v| v.re * v.re + v.im * v.im).sum();
    let det = v[0] * v[3] - v[1] * v[2];
    let det_abs = det.re.hypot(det.im);
    0.5 * ((trace + 2.0 * det_abs).sqrt() + (trace - 2.0 * det_abs).max(0.0).sqrt()) * scale
}

fn compare(name: &str, network: &Network) -> Result<f64, touchstone::TouchstoneError> {
    for warning in &network.warnings {
        eprintln!("{name}: warning: {warning}");
    }
    let report = network.sampled_quality(SampledQualityOptions { tolerance: 1e-12 })?;
    let mut max_relative_difference = 0.0_f64;
    for (index, point) in report.points.iter().enumerate() {
        let alternate = alternate_sigma(&network.s_matrix_at(index)?);
        let relative = (point.sigma_max - alternate).abs() / point.sigma_max.max(f64::MIN_POSITIVE);
        max_relative_difference = max_relative_difference.max(relative);
    }
    println!("{name}: samples={}, worst_sigma={:.16e} at {} Hz, worst_symmetry={} Hz, max relative formula difference={:.3e}", report.points.len(), report.points[report.worst_passivity.point_index].sigma_max, report.worst_passivity.frequency_hz, report.worst_symmetry.frequency_hz, max_relative_difference);
    Ok(max_relative_difference)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let c = |re, im| Complex { re, im };
    let cases = [
        ("zero", [[c(0.0, 0.0); 2]; 2], 0.0),
        (
            "ideal thru",
            [[c(0.0, 0.0), c(1.0, 0.0)], [c(1.0, 0.0), c(0.0, 0.0)]],
            1.0,
        ),
        (
            "attenuator",
            [[c(0.0, 0.0), c(0.5, 0.0)], [c(0.5, 0.0), c(0.0, 0.0)]],
            0.5,
        ),
        ("coherent gain", [[c(0.6, 0.0); 2]; 2], 1.2),
        ("tiny rank one", [[c(0.3e-300, 0.4e-300); 2]; 2], 1e-300),
        ("large rank one", [[c(0.3e300, 0.4e300); 2]; 2], 1e300),
        (
            "near degeneracy",
            [
                [c(1.0 + 1e-10, 0.0), c(0.0, 0.0)],
                [c(0.0, 0.0), c(1.0, 0.0)],
            ],
            1.0 + 1e-10,
        ),
        (
            "complex unitary",
            [
                [c(0.3, 0.4), c(0.0, 0.75_f64.sqrt())],
                [c(0.0, 0.75_f64.sqrt()), c(0.3, -0.4)],
            ],
            1.0,
        ),
    ];
    for (name, data, expected) in cases {
        let matrix = SMatrix::try_new(data.into_iter().map(Vec::from).collect())?;
        let network = NetworkBuilder::new("experiment.s2p", 2)
            .point(0.0, matrix)
            .build()?;
        let sigma = network
            .sampled_quality(SampledQualityOptions { tolerance: 1e-12 })?
            .points[0]
            .sigma_max;
        assert!((sigma - expected).abs() <= 2e-15 * expected.max(f64::MIN_POSITIVE));
        compare(name, &network)?;
    }
    let fixture = Network::new("files/ntwk1.s2p")?;
    assert!(fixture
        .sampled_quality(SampledQualityOptions { tolerance: 1e-12 })?
        .points
        .iter()
        .all(|p| p.sigma_max <= 1.0 + 1e-9));
    compare("normal passive fixture", &fixture)?;
    if let Some(path) = std::env::args().nth(1) {
        let network = Network::new(path)?;
        if network.rank != 2 {
            return Err("the alternate-formula experiment requires a two-port input".into());
        }
        compare("user input", &network)?;
    }
    Ok(())
}
