//! Edge case and error handling integration tests for the touchstone crate.

use touchstone::Network;

// ============================================================
// 1-port file parsing
// ============================================================

#[test]
fn parse_s1p_hfss_oneport() {
    let ntwk = Network::new("files/hfss_oneport.s1p".to_string());
    assert_eq!(ntwk.rank, 1);
    assert!(!ntwk.f.is_empty());
    assert_eq!(ntwk.frequency_unit, "GHz");
    assert_eq!(ntwk.format, "MA");
}

#[test]
fn parse_s1p_powerwave() {
    let ntwk = Network::new("files/hfss_oneport_powerwave.s1p".to_string());
    assert_eq!(ntwk.rank, 1);
    assert!(!ntwk.f.is_empty());
}

#[test]
fn s1p_s_parameters_all_formats() {
    let ntwk = Network::new("files/hfss_oneport.s1p".to_string());

    // Only S11 exists for 1-port
    let s11_db = ntwk.s_db(1, 1);
    let s11_ri = ntwk.s_ri(1, 1);
    let s11_ma = ntwk.s_ma(1, 1);

    assert_eq!(s11_db.len(), ntwk.f.len());
    assert_eq!(s11_ri.len(), ntwk.f.len());
    assert_eq!(s11_ma.len(), ntwk.f.len());

    for i in 0..s11_db.len() {
        assert!(s11_db[i].s_db.decibel().is_finite());
        assert!(s11_ri[i].s_ri.real().is_finite());
        assert!(s11_ma[i].s_ma.magnitude().is_finite());
    }
}

// ============================================================
// 4-port file parsing
// ============================================================

#[test]
fn parse_s4p_agilent() {
    let ntwk = Network::new("files/Agilent_E5071B.s4p".to_string());
    assert_eq!(ntwk.rank, 4);
    assert!(!ntwk.f.is_empty());
    // This file uses Hz and dB format with 75 ohm impedance
    assert_eq!(ntwk.frequency_unit, "Hz");
    assert_eq!(ntwk.format, "DB");
    assert!((ntwk.z0 - 75.0).abs() < 0.01);
}

#[test]
fn parse_s4p_rs_znb8() {
    let ntwk = Network::new("files/RS_ZNB8.s4p".to_string());
    assert_eq!(ntwk.rank, 4);
    assert!(!ntwk.f.is_empty());
}

#[test]
fn s4p_all_port_combinations() {
    let ntwk = Network::new("files/Agilent_E5071B.s4p".to_string());

    // Access all 16 S-parameter combinations for a 4-port
    for j in 1..=4 {
        for k in 1..=4 {
            let s = ntwk.s_db(j, k);
            assert_eq!(s.len(), ntwk.f.len(), "S{}{} length mismatch", j, k);
            for point in &s {
                assert!(point.s_db.decibel().is_finite(), "S{}{} has non-finite dB", j, k);
                assert!(point.s_db.angle().is_finite(), "S{}{} has non-finite angle", j, k);
            }
        }
    }
}

// ============================================================
// Option line variations (different freq units, formats, impedances)
// ============================================================

#[test]
fn option_line_ghz_ma() {
    // hfss_oneport.s1p uses: # GHZ S MA
    let ntwk = Network::new("files/hfss_oneport.s1p".to_string());
    assert_eq!(ntwk.frequency_unit, "GHz");
    assert_eq!(ntwk.format, "MA");
    assert!((ntwk.z0 - 50.0).abs() < 0.01);
}

#[test]
fn option_line_ghz_db() {
    // hfss_threeport_DB.s3p uses: # GHZ S DB
    let ntwk = Network::new("files/hfss_threeport_DB.s3p".to_string());
    assert_eq!(ntwk.frequency_unit, "GHz");
    assert_eq!(ntwk.format, "DB");
}

#[test]
fn option_line_hz_db_75ohm() {
    // Agilent_E5071B.s4p uses: # Hz S dB R 75
    let ntwk = Network::new("files/Agilent_E5071B.s4p".to_string());
    assert_eq!(ntwk.frequency_unit, "Hz");
    assert_eq!(ntwk.format, "DB");
    assert!((ntwk.z0 - 75.0).abs() < 0.01);
}

#[test]
fn option_line_50ohm_threeport_variants() {
    // Files with explicit 50 ohm
    let db50 = Network::new("files/hfss_threeport_DB_50Ohm.s3p".to_string());
    let ma50 = Network::new("files/hfss_threeport_MA_50Ohm.s3p".to_string());
    assert!((db50.z0 - 50.0).abs() < 0.01);
    assert!((ma50.z0 - 50.0).abs() < 0.01);
}

// ============================================================
// Network operations on real data
// ============================================================

#[test]
fn cascade_produces_valid_data() {
    let net1 = Network::new("files/ntwk1.s2p".to_string());
    let net2 = Network::new("files/ntwk2.s2p".to_string());
    let cascaded = net1.cascade(&net2);

    assert_eq!(cascaded.rank, 2);
    assert_eq!(cascaded.f.len(), net1.f.len());

    // All S-parameters should be finite
    for (j, k) in [(1, 1), (1, 2), (2, 1), (2, 2)] {
        for point in cascaded.s_db(j, k) {
            assert!(point.s_db.decibel().is_finite());
        }
    }
}

#[test]
fn cascade_matches_reference_file() {
    let net1 = Network::new("files/ntwk1.s2p".to_string());
    let net2 = Network::new("files/ntwk2.s2p".to_string());
    let cascaded = net1.cascade(&net2);
    let reference = Network::new("files/cascade_ntwk1_ntwk2.s2p".to_string());

    assert_eq!(cascaded.f.len(), reference.f.len());

    // Compare S21 dB values within tolerance
    let cas_s21 = cascaded.s_db(2, 1);
    let ref_s21 = reference.s_db(2, 1);
    for i in 0..cas_s21.len() {
        let diff = (cas_s21[i].s_db.decibel() - ref_s21[i].s_db.decibel()).abs();
        assert!(diff < 0.5, "S21 dB mismatch at index {}: {} vs {} (diff {})",
            i, cas_s21[i].s_db.decibel(), ref_s21[i].s_db.decibel(), diff);
    }
}

#[test]
fn mul_trait_cascade() {
    let net1 = Network::new("files/ntwk1.s2p".to_string());
    let expected_len = net1.f.len();
    let net2 = Network::new("files/ntwk2.s2p".to_string());

    // Mul trait should also work
    let cascaded = net1 * net2;
    assert_eq!(cascaded.rank, 2);
    assert_eq!(cascaded.f.len(), expected_len);
}

#[test]
fn s_db_ri_ma_consistency() {
    // Verify that DB, RI, and MA representations are consistent for the same data
    let ntwk = Network::new("files/ntwk1.s2p".to_string());

    let s11_db = ntwk.s_db(1, 1);
    let s11_ri = ntwk.s_ri(1, 1);
    let s11_ma = ntwk.s_ma(1, 1);

    for i in 0..s11_db.len() {
        // MA magnitude and DB magnitude should match
        let mag_from_ma = s11_ma[i].s_ma.magnitude();
        let mag_from_ri = s11_ri[i].s_ri.magnitude();
        let mag_from_db = s11_db[i].s_db.magnitude();

        assert!((mag_from_ma - mag_from_ri).abs() < 1e-6,
            "MA vs RI magnitude mismatch at {}: {} vs {}", i, mag_from_ma, mag_from_ri);
        assert!((mag_from_ma - mag_from_db).abs() < 1e-6,
            "MA vs DB magnitude mismatch at {}: {} vs {}", i, mag_from_ma, mag_from_db);

        // Angles should match
        let angle_from_ma = s11_ma[i].s_ma.angle();
        let angle_from_ri = s11_ri[i].s_ri.angle();
        let angle_from_db = s11_db[i].s_db.angle();

        assert!((angle_from_ma - angle_from_ri).abs() < 1e-4,
            "MA vs RI angle mismatch at {}: {} vs {}", i, angle_from_ma, angle_from_ri);
        assert!((angle_from_ma - angle_from_db).abs() < 1e-4,
            "MA vs DB angle mismatch at {}: {} vs {}", i, angle_from_ma, angle_from_db);
    }
}

// ============================================================
// Error cases
// ============================================================

#[test]
#[should_panic]
fn nonexistent_file_panics() {
    Network::new("files/does_not_exist.s2p".to_string());
}

#[test]
#[should_panic]
fn invalid_extension_panics() {
    Network::new("files/ntwk1.txt".to_string());
}

// ============================================================
// Round-trip: parse → save → re-parse
// ============================================================

#[test]
fn round_trip_s2p() {
    let original = Network::new("files/ntwk1.s2p".to_string());
    let tmp = "files/test_round_trip_edge.s2p";

    original.save(tmp).unwrap();
    let reloaded = Network::new(tmp.to_string());

    assert_eq!(original.rank, reloaded.rank);
    assert_eq!(original.f.len(), reloaded.f.len());
    assert!((original.z0 - reloaded.z0).abs() < 0.01);

    // Compare frequencies
    for i in 0..original.f.len() {
        let freq_diff = (original.f[i] - reloaded.f[i]).abs();
        assert!(freq_diff < 1e-3, "Frequency mismatch at {}: {} vs {}", i, original.f[i], reloaded.f[i]);
    }

    // Compare all S-parameters
    for (j, k) in [(1, 1), (1, 2), (2, 1), (2, 2)] {
        let orig_s = original.s_ri(j, k);
        let reload_s = reloaded.s_ri(j, k);
        for i in 0..orig_s.len() {
            let real_diff = (orig_s[i].s_ri.real() - reload_s[i].s_ri.real()).abs();
            let imag_diff = (orig_s[i].s_ri.imaginary() - reload_s[i].s_ri.imaginary()).abs();
            assert!(real_diff < 1e-4, "S{}{} real mismatch at {}", j, k, i);
            assert!(imag_diff < 1e-4, "S{}{} imag mismatch at {}", j, k, i);
        }
    }

    std::fs::remove_file(tmp).unwrap();
}

#[test]
fn round_trip_s3p() {
    let original = Network::new("files/hfss_threeport_DB.s3p".to_string());
    let tmp = "files/test_round_trip_s3p.s3p";

    original.save(tmp).unwrap();
    let reloaded = Network::new(tmp.to_string());

    assert_eq!(original.rank, reloaded.rank);
    assert_eq!(original.f.len(), reloaded.f.len());

    // Spot check S11
    let orig_s11 = original.s_db(1, 1);
    let reload_s11 = reloaded.s_db(1, 1);
    for i in 0..orig_s11.len() {
        let db_diff = (orig_s11[i].s_db.decibel() - reload_s11[i].s_db.decibel()).abs();
        assert!(db_diff < 0.1, "S11 dB round-trip mismatch at {}: {} vs {}",
            i, orig_s11[i].s_db.decibel(), reload_s11[i].s_db.decibel());
    }

    std::fs::remove_file(tmp).unwrap();
}

#[test]
fn round_trip_preserves_cascade_result() {
    let net1 = Network::new("files/ntwk1.s2p".to_string());
    let net2 = Network::new("files/ntwk2.s2p".to_string());
    let cascaded = net1.cascade(&net2);

    let tmp = "files/test_round_trip_cascade.s2p";
    cascaded.save(tmp).unwrap();
    let reloaded = Network::new(tmp.to_string());

    assert_eq!(cascaded.f.len(), reloaded.f.len());

    let cas_s21 = cascaded.s_ri(2, 1);
    let rel_s21 = reloaded.s_ri(2, 1);
    for i in 0..cas_s21.len() {
        let real_diff = (cas_s21[i].s_ri.real() - rel_s21[i].s_ri.real()).abs();
        assert!(real_diff < 1e-4, "Cascade round-trip real mismatch at {}", i);
    }

    std::fs::remove_file(tmp).unwrap();
}

// ============================================================
// Large port count
// ============================================================

#[test]
fn parse_s32p() {
    let ntwk = Network::new("files/ntwk.s32p".to_string());
    assert_eq!(ntwk.rank, 32);
    assert!(!ntwk.f.is_empty());
}

#[test]
fn parse_s8p() {
    let ntwk = Network::new("files/hfss_19.2.s8p".to_string());
    assert_eq!(ntwk.rank, 8);
    assert!(!ntwk.f.is_empty());
}

#[test]
fn parse_s10p() {
    let ntwk = Network::new("files/hfss_19.2.s10p".to_string());
    assert_eq!(ntwk.rank, 10);
    assert!(!ntwk.f.is_empty());
}

// ============================================================
// Various vendor file formats
// ============================================================

#[test]
fn parse_all_threeport_variants() {
    let files = [
        "files/hfss_threeport_DB.s3p",
        "files/hfss_threeport_DB_50Ohm.s3p",
        "files/hfss_threeport_MA.s3p",
        "files/hfss_threeport_MA_50Ohm.s3p",
        "files/hfss_threeport_MA_without_gamma_z0_50Ohm.s3p",
    ];

    for file in files {
        let ntwk = Network::new(file.to_string());
        assert_eq!(ntwk.rank, 3, "Wrong rank for {}", file);
        assert!(!ntwk.f.is_empty(), "No frequencies for {}", file);
    }
}
