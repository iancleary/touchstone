//! Run with --release --no-default-features --features experimental-vector-fitting.
//! No files are modified. All fit/validation data are explicitly disjoint.
use std::error::Error;
use touchstone::vector_fitting::{FitOptions, FitStatus, InitialPoles, RationalNetworkModel};
use touchstone::{Complex, Network, NetworkBuilder, SMatrix};

fn c(re: f64, im: f64) -> Complex {
    Complex { re, im }
}
fn synthetic(xs: &[f64], delay: bool) -> Result<Network, Box<dyn Error>> {
    let mut builder = NetworkBuilder::new("synthetic.s2p", 2);
    for &x in xs {
        let values = if delay {
            let h = c(
                (24.0 * std::f64::consts::PI * x).cos(),
                -(24.0 * std::f64::consts::PI * x).sin(),
            ) * 0.9;
            vec![vec![c(0.0, 0.0), h], vec![h, c(0.0, 0.0)]]
        } else {
            let z = c(0.0, x);
            let h = z * 0.032 / (z * z + z * 0.032 + c(0.61 * 0.61, 0.0));
            vec![
                vec![h * 0.8, (c(1.0, 0.0) - h) * 0.8],
                vec![(c(1.0, 0.0) - h) * 0.8, h * 0.8],
            ]
        };
        builder = builder.point(x * 1e9, SMatrix::try_new(values)?);
    }
    Ok(builder.build()?)
}
fn compare(name: &str, training: &Network, withheld: &Network) -> Result<(), Box<dyn Error>> {
    let options = FitOptions {
        initial_poles: InitialPoles::ConjugatePair {
            real: -0.15,
            imaginary: 0.2,
        },
        max_iterations: 12,
    };
    println!(
        "\n{name}: {} training, {} withheld, band {:?} Hz",
        training.frequencies().len(),
        withheld.frequencies().len(),
        [
            training.frequencies()[0],
            training.frequencies().last().copied().unwrap()
        ]
    );
    let fixed = RationalNetworkModel::fixed_pole_baseline(training, options)?;
    let fixed_assessment = fixed.assess(withheld)?;
    println!("EXPERIMENTAL fixed-pole baseline (NOT VF): train RMS/max {:.6e}/{:.6e}; withheld {:.6e}/{:.6e}",fixed.report().training_error.aggregate.rms,fixed.report().training_error.aggregate.max,fixed_assessment.model.aggregate.rms,fixed_assessment.model.aggregate.max);
    match RationalNetworkModel::fit(training, options) {
        Ok(model) => {
            let assessment = model.assess(withheld)?;
            println!("EXPERIMENTAL relocated VF: {:?}, {} iterations; train RMS/max {:.6e}/{:.6e}; withheld {:.6e}/{:.6e}",model.report().status,model.report().iterations.len(),model.report().training_error.aggregate.rms,model.report().training_error.aggregate.max,assessment.model.aggregate.rms,assessment.model.aggregate.max);
            println!(
                "actual Network::sample_at RI-linear: withheld RMS/max {:.6e}/{:.6e}",
                assessment.ri_linear.aggregate.rms, assessment.ri_linear.aggregate.max
            );
            // Explicit example-only suitability budget, not a library acceptance certificate.
            let suitable = model.report().status == FitStatus::Converged
                && assessment.model.aggregate.rms <= 0.01
                && assessment.model.aggregate.max <= 0.05;
            println!(
                "example suitability (converged AND withheld RMS<=0.01 AND max<=0.05): {suitable}"
            );
            println!("normalized poles: {:?}", model.normalized_poles());
        }
        Err(error) => {
            println!(
                "VF FAILED: {error}; not suitable. RI-linear withheld RMS/max {:.6e}/{:.6e}",
                fixed_assessment.ri_linear.aggregate.rms, fixed_assessment.ri_linear.aggregate.max
            );
        }
    }
    Ok(())
}

fn measured_split() -> Result<(Network, Network), Box<dyn Error>> {
    // Preserve the vendor's untouched DB/angle rows and all provenance/header text.
    let text = include_str!("../files/LFCN-2352+_Plus25degC.s2p");
    let rows: Vec<_> = text
        .lines()
        .filter(|line| {
            let line = line.trim();
            !line.is_empty() && !line.starts_with('!') && !line.starts_with('#')
        })
        .collect();
    let header = text
        .lines()
        .filter(|line| line.trim().starts_with('!') || line.trim().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    let indices: Vec<_> = (0..257).map(|i| i * (rows.len() - 1) / 256).collect();
    let held: Vec<_> = indices.windows(2).map(|w| (w[0] + w[1]) / 2).collect();
    let subset = |indices: &[usize]| -> Result<Network, Box<dyn Error>> {
        let mut data = header.clone();
        data.push('\n');
        for &i in indices {
            data.push_str(rows[i]);
            data.push('\n');
        }
        Ok(Network::from_str("LFCN-2352+_Plus25degC.s2p", &data)?)
    };
    println!("\nMeasured fixture: Mini-Circuits LOW PASS FILTER LFCN-2352+, +25 deg C, input 0 dBm; BSD-3-Clause scikit-rf source URL recorded in fixture header.");
    println!("257 deterministic index-spaced original rows; 256 disjoint original midpoint-index rows. No interpolation, repair, or holdout-driven seed selection.");
    Ok((subset(&indices)?, subset(&held)?))
}
fn main() -> Result<(), Box<dyn Error>> {
    println!("EXPERIMENTAL order<=2 shared-pole fitting. Stability != passivity; holdouts != unmeasured-frequency certificate.");
    for (name, delay, count) in [
        ("analytic passive resonator", false, 41),
        ("12-cycle delay failure", true, 81),
    ] {
        let xs: Vec<_> = (0..count)
            .map(|i| 0.05 + 0.95 * i as f64 / (count - 1) as f64)
            .collect();
        let hs: Vec<_> = xs
            .windows(2)
            .flat_map(|w| [0.125, 0.375, 0.625, 0.875].map(|t| w[0] + t * (w[1] - w[0])))
            .collect();
        compare(name, &synthetic(&xs, delay)?, &synthetic(&hs, delay)?)?;
    }
    let (training, withheld) = measured_split()?;
    compare(
        "vendor passive-component measurement (not a passivity certificate)",
        &training,
        &withheld,
    )?;
    Ok(())
}
