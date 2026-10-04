//! Integration tests matching every code example in README.md

use touchstone::{Complex, Network, NetworkBuilder, SMatrix, TouchstoneWarning};

// --- Section 2: Loading a Network ---

#[test]
fn loading_a_network() {
    let ntwk = Network::new("files/ntwk1.s2p").unwrap();

    assert_eq!(ntwk.rank, 2);
    assert!(!ntwk.frequency_unit.is_empty());
    assert!(!ntwk.format.is_empty());
    assert!(ntwk.z0 > 0.0);
    assert!(!ntwk.frequencies().is_empty());
}

#[test]
fn parser_warnings() {
    let ntwk = Network::from_str("uploaded.s1p", "1.0 0.5 0.0\n").unwrap();

    assert!(matches!(
        ntwk.warnings.as_slice(),
        [TouchstoneWarning::MissingOptionLine { .. }]
    ));
}

// --- Section 3: Accessing S-Parameters ---

#[test]
fn checked_construction_and_borrowed_points() -> Result<(), touchstone::TouchstoneError> {
    let matrix = SMatrix::try_new(vec![vec![Complex { re: 0.5, im: 0.0 }]])?;
    let network = NetworkBuilder::new("generated.s1p", 1)
        .point(0.0, matrix.clone())
        .point(1.0e9, matrix)
        .build()?;
    assert_eq!(network.frequencies(), &[0.0, 1.0e9]);
    for point in network.iter_points() {
        println!("{} Hz: {:?}", point.frequency(), point.s().get(1, 1)?);
        assert_eq!(point.s().get(1, 1)?, Complex { re: 0.5, im: 0.0 });
    }
    Ok(())
}

#[test]
fn s_parameter_accessors_return_independent_values() {
    for format in ["RI", "MA", "DB"] {
        let network = Network::from_str(
            "example.s1p",
            format!("# Hz S {format} R 50\n1 0.5 0\n").as_str(),
        )
        .unwrap();
        let serialized = network.to_touchstone_string().unwrap();
        let expected_points = network.points().unwrap();
        let expected_ri = network.s_ri(1, 1)[0].s_ri;
        let expected_ma = network.s_ma(1, 1)[0].s_ma;
        let expected_db = network.s_db(1, 1)[0].s_db;

        let mut ri = network.s_ri(1, 1);
        let mut ma = network.s_ma(1, 1);
        let mut db = network.s_db(1, 1);
        ri[0].s_ri.0 = 9.0;
        ma[0].s_ma.0 = 9.0;
        db[0].s_db.0 = 9.0;
        assert_eq!(ri[0].s_ri.0, 9.0);
        assert_eq!(ma[0].s_ma.0, 9.0);
        assert_eq!(db[0].s_db.0, 9.0);

        let mut matrix = network.s_matrix_at(0).unwrap();
        matrix.data[0][0].re = 9.0;
        let mut point = network.point_at(0).unwrap();
        point.s.data[0][0].re = 9.0;
        let mut points = network.points().unwrap();
        points[0].s.data[0][0].re = 9.0;
        assert_eq!(matrix.data[0][0].re, 9.0);
        assert_eq!(point.s.data[0][0].re, 9.0);
        assert_eq!(points[0].s.data[0][0].re, 9.0);

        assert_eq!(network.s_ri(1, 1)[0].s_ri, expected_ri, "{format}");
        assert_eq!(network.s_ma(1, 1)[0].s_ma, expected_ma, "{format}");
        assert_eq!(network.s_db(1, 1)[0].s_db, expected_db, "{format}");
        assert_eq!(network.points().unwrap(), expected_points, "{format}");
        assert_eq!(
            network.to_touchstone_string().unwrap(),
            serialized,
            "{format}"
        );
    }
}

#[test]
fn s_parameters_db() {
    let ntwk = Network::new("files/ntwk1.s2p").unwrap();

    let s11_db = ntwk.s_db(1, 1);
    assert_eq!(s11_db.len(), ntwk.frequencies().len());
    for point in &s11_db {
        assert!(point.frequency > 0.0);
        // decibel can be negative, angle can be any value — just check they're finite
        assert!(point.s_db.decibel().is_finite());
        assert!(point.s_db.angle().is_finite());
    }
}

#[test]
fn s_parameters_ri() {
    let ntwk = Network::new("files/ntwk1.s2p").unwrap();

    let s21_ri = ntwk.s_ri(2, 1);
    assert_eq!(s21_ri.len(), ntwk.frequencies().len());
    for point in &s21_ri {
        assert!(point.frequency > 0.0);
        assert!(point.s_ri.real().is_finite());
        assert!(point.s_ri.imaginary().is_finite());
    }
}

#[test]
fn s_parameters_ma() {
    let ntwk = Network::new("files/ntwk1.s2p").unwrap();

    let s21_ma = ntwk.s_ma(2, 1);
    assert_eq!(s21_ma.len(), ntwk.frequencies().len());
    for point in &s21_ma {
        assert!(point.frequency > 0.0);
        assert!(point.s_ma.magnitude().is_finite());
        assert!(point.s_ma.angle().is_finite());
    }
}

#[test]
fn field_aliases_ri() {
    let ntwk = Network::new("files/ntwk1.s2p").unwrap();
    let s11_ri = ntwk.s_ri(1, 1);
    let point = &s11_ri[0];

    // RealImaginary has all these accessors
    assert!(point.s_ri.real().is_finite());
    assert!(point.s_ri.imaginary().is_finite());
    assert!(point.s_ri.magnitude().is_finite());
    assert!(point.s_ri.decibel().is_finite());
    assert!(point.s_ri.angle().is_finite());

    // Conversion methods
    let _ma = point.s_ri.magnitude_angle();
    let _da = point.s_ri.decibel_angle();
}

#[test]
fn field_aliases_db() {
    let ntwk = Network::new("files/ntwk1.s2p").unwrap();
    let s11_db = ntwk.s_db(1, 1);
    let point = &s11_db[0];

    assert!(point.s_db.decibel().is_finite());
    assert!(point.s_db.angle().is_finite());
    assert!(point.s_db.magnitude().is_finite());
    assert!(point.s_db.real().is_finite());
    assert!(point.s_db.imaginary().is_finite());
}

#[test]
fn field_aliases_ma() {
    let ntwk = Network::new("files/ntwk1.s2p").unwrap();
    let s21_ma = ntwk.s_ma(2, 1);
    let point = &s21_ma[0];

    assert!(point.s_ma.magnitude().is_finite());
    assert!(point.s_ma.angle().is_finite());
    assert!(point.s_ma.decibel().is_finite());
    assert!(point.s_ma.real().is_finite());
    assert!(point.s_ma.imaginary().is_finite());

    // Conversion
    let _ri = point.s_ma.real_imaginary();
    let _da = point.s_ma.decible_angle(); // note: method name has typo in crate
}

// --- Section 4: Saving Networks ---

#[test]
fn save_network() {
    let ntwk = Network::new("files/ntwk1.s2p").unwrap();
    let tmp_path = "files/test_save_readme.s2p";

    ntwk.save(tmp_path).unwrap();

    // Verify round-trip
    let reloaded = Network::new(tmp_path).unwrap();
    assert_eq!(reloaded.rank, ntwk.rank);
    assert_eq!(reloaded.frequencies().len(), ntwk.frequencies().len());

    // Clean up
    std::fs::remove_file(tmp_path).unwrap();
}

// --- Section 5: Cascading 2-Port Networks ---

#[test]
fn cascade_networks() {
    let net1 = Network::new("files/ntwk1.s2p").unwrap();
    let net2 = Network::new("files/ntwk2.s2p").unwrap();

    let cascaded = net1.cascade(&net2);
    let checked = net1.try_cascade(&net2).unwrap();
    assert_eq!(checked.points().unwrap(), cascaded.points().unwrap());
    assert_eq!(cascaded.rank, 2);
    assert!(!cascaded.frequencies().is_empty());
    println!(
        "Cascaded network has {} data points",
        cascaded.frequencies().len()
    );
}

#[test]
fn cascade_ports() {
    let net1 = Network::new("files/ntwk1.s2p").unwrap();
    let net2 = Network::new("files/ntwk2.s2p").unwrap();

    let cascaded = net1.cascade_ports(&net2, 2, 1);
    assert_eq!(cascaded.rank, 2);
    assert!(!cascaded.frequencies().is_empty());
}

// --- Multi-port loading (verifies N-port support mentioned in docs) ---

#[test]
fn load_1port() {
    let ntwk = Network::new("files/hfss_oneport.s1p").unwrap();
    assert_eq!(ntwk.rank, 1);
}

#[test]
fn load_3port() {
    let ntwk = Network::new("files/hfss_18.2.s3p").unwrap();
    assert_eq!(ntwk.rank, 3);
}

#[test]
fn load_4port() {
    let ntwk = Network::new("files/Agilent_E5071B.s4p").unwrap();
    assert_eq!(ntwk.rank, 4);
}
