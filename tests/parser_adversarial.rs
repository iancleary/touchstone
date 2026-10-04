use touchstone::{Network, TouchstoneError};

#[path = "support/parser_fuzz.rs"]
mod parser_harness;

#[test]
fn malformed_and_unicode_source_names_return_errors() {
    for source in [
        "",
        "no_extension",
        "bad.",
        "bad.é",
        "bad.💣",
        "bad.sé",
        "bad.s2💣",
        "bad.s",
        "bad.sp",
        "bad.s0p",
        "bad.s01p",
        "bad.s-1p",
        "bad.s+1p",
        "bad.s٢p",
        "bad.s2147483648p",
        "bad.s18446744073709551616p",
    ] {
        assert!(
            Network::from_str(source, "# Hz S RI R 50\n1 0 0\n").is_err(),
            "unexpected success for {source:?}"
        );
    }
}

#[test]
fn huge_declared_matrices_do_not_overflow_or_allocate_from_the_rank() {
    for ports in [32767, 32768, 46341, 65536, i32::MAX] {
        let source = format!("huge.s{ports}p");
        let error = Network::from_str(&source, "# Hz S RI R 50\n1 0 0\n").unwrap_err();
        assert!(
            matches!(
                error.root_cause(),
                TouchstoneError::InvalidDataLineParts { actual: 3, .. }
                    | TouchstoneError::InvalidNumberOfPorts { .. }
            ),
            "unexpected error for {source}: {error}"
        );
    }
}

#[test]
fn arbitrary_bytes_and_unicode_source_names_do_not_panic() {
    // A fixed seed makes failures repeatable in the ordinary test suite.
    let mut state = 0x539d_96bc_72e1_a845_u64;
    for case in 0..4096 {
        let mut bytes = Vec::with_capacity(case % 257);
        for _ in 0..case % 257 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            bytes.push(state as u8);
        }
        parser_harness::exercise(&bytes);

        let source = format!("arbitrary.{}", String::from_utf8_lossy(&bytes));
        let _ = Network::from_bytes(&source, b"# Hz S RI R 50\n1 0 0\n");
    }
}

#[test]
fn seed_truncations_and_every_single_byte_mutation_do_not_panic() {
    for seed in [
        "legacy.s2p\n# GHz S RI R 50\n1 0.1 0.2 0.9 -0.1 0.01 0.02 -0.2 0.3\n",
        "reference.s2p\n[Version] 2.1\n# MHz S MA R 50\n[Number of Ports] 2\n\
         [Two-Port Data Order] 12_21\n[Number of Frequencies] 1\n[Reference]\n\
         50 75\n[Matrix Format] Full\n[Network Data]\n1 0.1 45 0.2 30 0.3 15 0.4 -15\n[End]\n",
        "multi.s3p\n# Hz S DB R 50\n1 -10 0 -20 0 -30 0\n\
         -40 0 -50 0 -60 0\n-70 0 -80 0 -90 0\n",
        "huge.s2147483647p\n# Hz S RI R 50\n1 0 0\n",
        "bad.é\n# Hz S RI R 50\n1 0 0\n",
        "nonfinite.s1p\n# GHz S DB R 50\nNaN inf -inf\n1e309 -1e309 1e309\n",
    ] {
        let seed = seed.as_bytes();
        for end in 0..=seed.len() {
            parser_harness::exercise(&seed[..end]);
        }
        let mut mutated = seed.to_vec();
        for position in 0..seed.len() {
            for replacement in 0..=u8::MAX {
                mutated[position] = replacement;
                parser_harness::exercise(&mutated);
            }
            mutated[position] = seed[position];
        }
    }
}
