use touchstone::{
    ABCDMatrix, Complex, Extrapolation, Interpolation, Network, NetworkBuilder, NetworkPoint,
    ParameterMatrix, SMatrix, TouchstoneError,
};

fn c(re: f64, im: f64) -> Complex {
    Complex { re, im }
}

#[test]
fn checked_matrix_constructors_infer_rank_and_reject_invalid_values() {
    let data = vec![
        vec![c(1.0, 0.0), c(2.0, 0.0)],
        vec![c(3.0, 0.0), c(4.0, 0.0)],
    ];
    let s = SMatrix::try_new(data.clone()).unwrap();
    let z = ParameterMatrix::try_new(data).unwrap();
    assert_eq!(s.rank, 2);
    assert_eq!(z.rank, 2);
    assert_eq!(s.get(2, 1).unwrap(), c(3.0, 0.0));
    assert_eq!(z.get(1, 2).unwrap(), c(2.0, 0.0));

    for data in [vec![], vec![vec![]], vec![vec![c(0.0, 0.0)], vec![]]] {
        assert!(matches!(
            SMatrix::try_new(data.clone()),
            Err(TouchstoneError::InvalidParameterMatrixShape { .. })
        ));
        assert!(matches!(
            ParameterMatrix::try_new(data),
            Err(TouchstoneError::InvalidParameterMatrixShape { .. })
        ));
    }
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for value in [c(invalid, 0.0), c(0.0, invalid)] {
            let data = vec![vec![value]];
            assert!(matches!(
                SMatrix::try_new(data.clone()),
                Err(TouchstoneError::InvalidParameterMatrixValue { .. })
            ));
            assert!(matches!(
                ParameterMatrix::try_new(data),
                Err(TouchstoneError::InvalidParameterMatrixValue { .. })
            ));
            for index in 0..4 {
                let mut entries = [c(0.0, 0.0); 4];
                entries[index] = value;
                assert!(
                    ABCDMatrix::try_new(entries[0], entries[1], entries[2], entries[3]).is_err()
                );
            }
        }
    }
    let abcd = ABCDMatrix::try_new(c(1.0, 0.0), c(0.0, 0.0), c(0.0, 0.0), c(1.0, 0.0)).unwrap();
    assert_eq!(
        abcd.to_s_matrix(50.0).unwrap().get(2, 1).unwrap(),
        c(1.0, 0.0)
    );
}

#[test]
fn checked_point_constructor_validates_frequency_and_matrix() {
    let s = SMatrix::try_new(vec![vec![c(0.5, 0.0)]]).unwrap();
    assert_eq!(
        NetworkPoint::try_new(0.0, s.clone()).unwrap().frequency,
        0.0
    );
    for frequency in [-1.0, f64::NAN, f64::INFINITY] {
        assert!(matches!(
            NetworkPoint::try_new(frequency, s.clone()),
            Err(TouchstoneError::InvalidFrequency { .. })
        ));
    }
    let mut malformed = s.clone();
    malformed.rank = 2;
    assert!(NetworkPoint::try_new(1.0, malformed).is_err());
    let mut nonfinite = s;
    nonfinite.data[0][0].im = f64::NAN;
    assert!(NetworkPoint::try_new(1.0, nonfinite).is_err());
}

#[test]
fn borrowed_views_preserve_frequency_and_asymmetric_port_order() {
    let network = Network::from_str(
        "views.s2p",
        "# Hz S RI R 50\n0 0.1 0.2 0.3 0.4 0.5 0.6 0.7 0.8\n2 0.2 0.3 0.4 0.5 0.6 0.7 0.8 0.9\n",
    )
    .unwrap();
    assert_eq!(network.frequencies(), &[0.0, 2.0]);
    let mut copied_frequencies = network.f();
    copied_frequencies[0] = 99.0;
    assert_eq!(network.frequencies()[0], 0.0);

    let first = network.point_ref(0).unwrap();
    assert_eq!(first.frequency(), 0.0);
    assert_eq!(first.s().rank(), 2);
    assert_eq!(first.s().get(2, 1).unwrap(), c(0.3, 0.4));
    assert_eq!(first.s().get(1, 2).unwrap(), c(0.5, 0.6));
    assert!(matches!(
        first.s().get(0, 1),
        Err(TouchstoneError::InvalidPortIndex { .. })
    ));
    assert!(matches!(
        first.s().get(1, 3),
        Err(TouchstoneError::InvalidPortIndex { .. })
    ));
    assert!(matches!(
        network.point_ref(2),
        Err(TouchstoneError::InvalidPointIndex { .. })
    ));

    let mut iter = network.iter_points();
    assert_eq!(iter.len(), 2);
    assert_eq!(iter.next_back().unwrap().frequency(), 2.0);
    assert_eq!(iter.next().unwrap().frequency(), 0.0);
    assert_eq!(iter.len(), 0);
    assert!(iter.next().is_none());
    for (borrowed, owned) in network.iter_points().zip(network.points().unwrap()) {
        assert_eq!(borrowed.frequency(), owned.frequency);
        for to in 1..=2 {
            for from in 1..=2 {
                assert_eq!(
                    borrowed.s().get(to, from).unwrap(),
                    owned.s.get(to, from).unwrap()
                );
            }
        }
    }
}

#[test]
fn builder_and_sampling_enforce_the_same_frequency_grid_rules() {
    let matrix = SMatrix::try_new(vec![vec![c(0.5, 0.0)]]).unwrap();
    for grid in [
        vec![1.0, 1.0],
        vec![2.0, 1.0],
        vec![-1.0, 0.0],
        vec![0.0, f64::NAN],
        vec![0.0, f64::INFINITY],
    ] {
        let mut builder = NetworkBuilder::new("grid.s1p", 1);
        let mut contents = String::from("# Hz S RI R 50\n");
        for &frequency in &grid {
            builder.push_point(frequency, matrix.clone());
            contents.push_str(&format!("{frequency} 0.5 0\n"));
        }
        let build_error = builder.build().unwrap_err();
        let parsed = Network::from_str("grid.s1p", &contents).unwrap();
        let sample_error = parsed
            .sample_at(0.0, Interpolation::Linear, Extrapolation::Clamp)
            .unwrap_err();
        assert_eq!(
            std::mem::discriminant(&build_error),
            std::mem::discriminant(&sample_error),
            "grid: {grid:?}"
        );
        assert!(matches!(
            build_error,
            TouchstoneError::DuplicateFrequency { .. }
                | TouchstoneError::UnsortedFrequencies { .. }
                | TouchstoneError::InvalidFrequency { .. }
        ));
    }
    let network = NetworkBuilder::new("dc.s1p", 1)
        .point(0.0, matrix.clone())
        .point(2.0, matrix)
        .build()
        .unwrap();
    assert_eq!(
        network
            .sample_at(1.0, Interpolation::Linear, Extrapolation::Error)
            .unwrap()
            .s
            .get(1, 1)
            .unwrap(),
        c(0.5, 0.0)
    );
    assert!(matches!(
        network.sample_at(-1.0, Interpolation::Linear, Extrapolation::Clamp),
        Err(TouchstoneError::InvalidFrequency { .. })
    ));
}
