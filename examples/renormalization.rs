//! Analytic checks and a bounded direct-S versus S->Z->S experiment.
//! Run: cargo run --offline --release --example renormalization
use std::time::Instant;
use touchstone::{Complex, Network, NetworkBuilder, SMatrix, TouchstoneError};

fn one(s: f64, z0: f64) -> Result<Network, TouchstoneError> {
    Network::from_str("load.s1p", &format!("# Hz S RI R {z0}\n1 {s} 0\n"))
}

fn compare(source: &Network, new_z0: f64, expected: f64) -> Result<(), TouchstoneError> {
    let direct = source.renormalize(new_z0)?.try_s_ri_at(0, 1, 1)?;
    assert!((direct.re - expected).abs() < 2e-14);
    let detour = source
        .z_matrix_at(0)
        .and_then(|z| z.to_s_matrix_from_z(new_z0));
    match detour {
        Ok(s) => {
            let value = s.get(1, 1)?;
            assert!((value.re - expected).abs() < 2e-14);
            println!(
                "S={} -> direct {}, Z detour {}",
                source.try_s_ri_at(0, 1, 1)?.re,
                direct.re,
                value.re
            );
        }
        Err(e) => println!(
            "S={} -> direct {}, Z detour rejected: {e}",
            source.try_s_ri_at(0, 1, 1)?.re,
            direct.re
        ),
    }
    Ok(())
}

// Experimental scalar wave scaling, NOT an alternative production solver.
// Normalize a=old/max(old,new), b=new/max(old,new) to avoid sum overflow.
fn scaled_waves(s: f64, old: f64, new: f64) -> f64 {
    let scale = old.max(new);
    let a = (old / scale) * (1.0 + s);
    let b = (new / scale) * (1.0 - s);
    (a - b) / (a + b)
}

fn main() -> Result<(), TouchstoneError> {
    for (s, expected) in [(0.2, 0.0), (0.0, -0.2), (1.0, 1.0), (-1.0, -1.0)] {
        compare(&one(s, 50.0)?, 75.0, expected)?;
    }
    let identity = one(1.0, 50.0)?;
    assert_eq!(
        identity.to_touchstone_string()?,
        identity.renormalize(50.0)?.to_touchstone_string()?
    );

    // Independent T-resistor circuit: series 75, shunt 25, series 25 ohms.
    // Z=[[100,25],[25,50]] from Kirchhoff's laws. Closed-form port-wave
    // equations at r=75 give S=[[2,3],[3,-4]]/17, not a round-trip oracle.
    let source = Network::from_str(
        "resistors.s2p",
        &format!(
        "! T resistor circuit\n# GHz S RI R 50\n[Vendor Note] unsupported\n1 {} 0 {} 0 {} 0 {} 0\n",
        7.0/23.0, 4.0/23.0, 4.0/23.0, -1.0/23.0,
    ),
    )?;
    let output = source.renormalize(75.0)?;
    let via_z = source.z_matrix_at(0)?.to_s_matrix_from_z(75.0)?;
    let reparsed = Network::from_str("saved.s2p", &output.to_touchstone_string()?)?;
    assert_eq!(output.warnings, source.warnings);
    assert_eq!(reparsed.comments, source.comments);
    assert_eq!(reparsed.frequency_unit, source.frequency_unit);
    assert_eq!(reparsed.frequencies(), source.frequencies());
    assert_eq!(reparsed.z0, 75.0);
    assert!(reparsed.warnings.is_empty());
    for (i, row) in [[2.0 / 17.0, 3.0 / 17.0], [3.0 / 17.0, -4.0 / 17.0]]
        .iter()
        .enumerate()
    {
        for (j, expected) in row.iter().enumerate() {
            for actual in [
                output.try_s_ri_at(0, i + 1, j + 1)?,
                via_z.get(i + 1, j + 1)?,
                reparsed.try_s_ri_at(0, i + 1, j + 1)?,
            ] {
                assert!((actual.re - expected).abs() < 2e-14);
            }
        }
    }
    println!(
        "T circuit and transform -> serialize -> parse passed; warnings retained in memory only"
    );

    for (old, new) in [(1.0, 1e17), (1e17, 1.0), (1e-300, 1e300), (1e300, 1e-300)] {
        for s in [1.0, -1.0] {
            let error = one(s, old)?.renormalize(new).unwrap_err();
            assert!(matches!(
                error,
                TouchstoneError::UnrepresentableReferenceRatio { .. }
            ));
            println!(
                "{old:e} -> {new:e}, S={s}: direct rejected; scaled-wave probe={}",
                scaled_waves(s, old, new)
            );
        }
    }
    assert!(scaled_waves(1.0, 1e-300, 1e300).is_nan());
    assert!(scaled_waves(-1.0, 1e300, 1e-300).is_nan());
    for (s, old, new) in [(5.0, 50.0, 75.0), (1.0, 1.0, 1e13)] {
        let error = one(s, old)?.renormalize(new).unwrap_err();
        assert!(matches!(error, TouchstoneError::SingularMatrix { .. }));
        println!("expected singular/cutoff rejection: {error}");
    }
    compare(&one(0.0, 1e308)?, 1.5e308, -0.2)?;

    // Synthetic dense complex matrices, 32 points, repeated 10 times. Compare
    // actual public workflows (direct rebuilds all caches; detour returns matrices).
    // Single elapsed totals, not a controlled benchmark or an equal-work speedup.
    for rank in [2, 8, 32] {
        let data = (0..rank)
            .map(|i| {
                (0..rank)
                    .map(|j| Complex {
                        re: if i == j { 0.1 } else { 0.1 / rank as f64 },
                        im: (i as f64 - j as f64) * 0.01 / rank as f64,
                    })
                    .collect()
            })
            .collect();
        let matrix = SMatrix::try_new(data)?;
        let mut builder = NetworkBuilder::new(format!("throughput.s{rank}p"), rank);
        for point in 0..32 {
            builder.push_point(1e9 + point as f64 * 1e6, matrix.clone());
        }
        let network = builder.build()?;
        let direct = network.renormalize(75.0)?;
        for point in 0..32 {
            let detour = network.z_matrix_at(point)?.to_s_matrix_from_z(75.0)?;
            for i in 1..=rank {
                for j in 1..=rank {
                    let delta = direct.try_s_ri_at(point, i, j)? - detour.get(i, j)?;
                    assert!(delta.re.hypot(delta.im) < 1e-13);
                }
            }
        }
        let start = Instant::now();
        for _ in 0..10 {
            std::hint::black_box(network.renormalize(75.0)?);
        }
        let direct_time = start.elapsed();
        let start = Instant::now();
        for _ in 0..10 {
            for point in 0..32 {
                std::hint::black_box(network.z_matrix_at(point)?.to_s_matrix_from_z(75.0)?);
            }
        }
        println!(
            "rank {rank}, 320 points: direct incl caches {direct_time:?}, S->Z->S matrices {:?}",
            start.elapsed()
        );
    }
    Ok(())
}
