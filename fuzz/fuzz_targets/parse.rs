#![no_main]

use libfuzzer_sys::fuzz_target;

#[path = "../../tests/support/parser_fuzz.rs"]
mod parser_harness;

fuzz_target!(|input: &[u8]| parser_harness::exercise(input));
