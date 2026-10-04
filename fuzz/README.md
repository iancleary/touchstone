# Parser fuzzing

Install the prerequisites once:

```bash
rustup toolchain install nightly --profile minimal
cargo install cargo-fuzz --locked
```

Run a bounded coverage-guided campaign from the repository root:

```bash
cargo +nightly fuzz run parse -- -max_len=4096 -max_total_time=60 -rss_limit_mb=2048
```

The first input line supplies an arbitrary UTF-8 source name. Remaining bytes
supply the file body. The harness also uses valid one-, two-, and three-port
source names so mutations reach the numeric and keyword parsers. It calls both
`Network::from_bytes` and `Network::from_str` for valid UTF-8, and requires their
success/failure results to agree. Invalid UTF-8 must return an error.

The campaign checks for panics and sanitizer failures. It does not certify file
conformance, numerical accuracy, or a fixed memory bound for arbitrarily large
inputs. The input length and process memory limits bound each local campaign.

Named corpus files cover RI, MA, DB, full N-port matrices, explicit data order,
reference impedances, nonfinite numbers, Unicode names, and oversized port
counts. Keep minimized failures as named corpus seeds and focused regression
tests. LibFuzzer's generated corpus files and crash artifacts are ignored.

The normal test suite needs no nightly toolchain or fuzz dependencies. Run its
deterministic arbitrary-byte, source-name, truncation, and single-byte mutation
coverage with:

```bash
cargo test --test parser_adversarial
```
