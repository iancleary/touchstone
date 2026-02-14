//! Integration tests matching every code example in README.md

use touchstone::Network;

// --- Section 2: Loading a Network ---

#[test]
fn loading_a_network() {
    let ntwk = Network::new("files/ntwk1.s2p".to_string());

    assert_eq!(ntwk.rank, 2);
    assert!(!ntwk.frequency_unit.is_empty());
    assert!(!ntwk.format.is_empty());
    assert!(ntwk.z0 > 0.0);
    assert!(!ntwk.f.is_empty());
}

// --- Section 3: Accessing S-Parameters ---

#[test]
fn s_parameters_db() {
    let ntwk = Network::new("files/ntwk1.s2p".to_string());

    let s11_db = ntwk.s_db(1, 1);
    assert_eq!(s11_db.len(), ntwk.f.len());
    for point in &s11_db {
        assert!(point.frequency > 0.0);
        // decibel can be negative, angle can be any value — just check they're finite
        assert!(point.s_db.decibel().is_finite());
        assert!(point.s_db.angle().is_finite());
    }
}

#[test]
fn s_parameters_ri() {
    let ntwk = Network::new("files/ntwk1.s2p".to_string());

    let s21_ri = ntwk.s_ri(2, 1);
    assert_eq!(s21_ri.len(), ntwk.f.len());
    for point in &s21_ri {
        assert!(point.frequency > 0.0);
        assert!(point.s_ri.real().is_finite());
        assert!(point.s_ri.imaginary().is_finite());
    }
}

#[test]
fn s_parameters_ma() {
    let ntwk = Network::new("files/ntwk1.s2p".to_string());

    let s21_ma = ntwk.s_ma(2, 1);
    assert_eq!(s21_ma.len(), ntwk.f.len());
    for point in &s21_ma {
        assert!(point.frequency > 0.0);
        assert!(point.s_ma.magnitude().is_finite());
        assert!(point.s_ma.angle().is_finite());
    }
}

#[test]
fn field_aliases_ri() {
    let ntwk = Network::new("files/ntwk1.s2p".to_string());
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
    let ntwk = Network::new("files/ntwk1.s2p".to_string());
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
    let ntwk = Network::new("files/ntwk1.s2p".to_string());
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
    let ntwk = Network::new("files/ntwk1.s2p".to_string());
    let tmp_path = "files/test_save_readme.s2p";

    ntwk.save(tmp_path).unwrap();

    // Verify round-trip
    let reloaded = Network::new(tmp_path.to_string());
    assert_eq!(reloaded.rank, ntwk.rank);
    assert_eq!(reloaded.f.len(), ntwk.f.len());

    // Clean up
    std::fs::remove_file(tmp_path).unwrap();
}

// --- Section 5: Cascading 2-Port Networks ---

#[test]
fn cascade_networks() {
    let net1 = Network::new("files/ntwk1.s2p".to_string());
    let net2 = Network::new("files/ntwk2.s2p".to_string());

    let cascaded = net1.cascade(&net2);
    assert_eq!(cascaded.rank, 2);
    assert!(!cascaded.f.is_empty());
    println!("Cascaded network has {} data points", cascaded.f.len());
}

#[test]
fn cascade_ports() {
    let net1 = Network::new("files/ntwk1.s2p".to_string());
    let net2 = Network::new("files/ntwk2.s2p".to_string());

    let cascaded = net1.cascade_ports(&net2, 2, 1);
    assert_eq!(cascaded.rank, 2);
    assert!(!cascaded.f.is_empty());
}

// --- Multi-port loading (verifies N-port support mentioned in docs) ---

#[test]
fn load_1port() {
    let ntwk = Network::new("files/hfss_oneport.s1p".to_string());
    assert_eq!(ntwk.rank, 1);
}

#[test]
fn load_3port() {
    let ntwk = Network::new("files/hfss_18.2.s3p".to_string());
    assert_eq!(ntwk.rank, 3);
}

#[test]
fn load_4port() {
    let ntwk = Network::new("files/Agilent_E5071B.s4p".to_string());
    assert_eq!(ntwk.rank, 4);
}
