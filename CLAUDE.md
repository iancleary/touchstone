# CLAUDE.md — touchstone

## Overview

Rust crate for parsing, analyzing, and writing Touchstone (SNP) files — the industry-standard format for S-parameter data. Supports 1-port through N-port (tested to 32-port), all data formats (RI/MA/DB), generated networks, interpolation/resampling, reference impedance metadata, network parameter conversions, and 2-port network cascading via ABCD parameters. Published on crates.io; current release target is v0.14.3.

## Agent Usage

Use `touchstone` whenever the task starts from measured or simulated
S-parameter data: `.s1p`, `.s2p`, `.s3p`, `.sNp` parsing, plotting, resampling,
reference impedance metadata, S/Y/Z/ABCD conversion, or cascading two-port
networks. Create a `Network` with `Network::new`, `Network::from_str`, or
`Network::from_bytes`; use `NetworkBuilder` only when generating synthetic
network data in memory.

Do not use this crate for scalar dBm/noise/wavelength conversion; use
`rfconversions`. Do not use it for block-level gain/NF/P1dB/IP3 lineup
modeling unless S-parameter files are the source of the stages; use
`gainlineup` for block cascades. Use `linkbudget` when the question is about
end-to-end link margin, BER, modulation, orbit, Doppler, or regulatory PFD.

Important conventions for agents: S-parameter port indices are 1-indexed
(`s_db(2, 1)` is S21), parsed frequencies are stored in Hz, and interpolation is
performed on real/imaginary components before rebuilding magnitude/angle or
dB/angle views. Preserve parser warnings for non-fatal file issues instead of
discarding them.

## Agent Operating Loop

Start with the file format contract before changing code. Touchstone parsing is
mostly about preserving RF file semantics across many dialects, so read the
smallest relevant slice of `docs/touchstone_ver2_1.pdf`, then inspect the
parser, data-pair conversions, and tests that already cover that behavior.

Make changes accretive:

- When adding parser support, add or update a fixture in `files/` and a focused
  integration test that names the Touchstone construct being protected.
- When fixing conversion, interpolation, cascade, or matrix behavior, assert on
  stable public APIs such as `Network::from_str`, `Network::from_bytes`,
  `s_matrix_at`, `points`, `sample_at`, or `to_touchstone_string` instead of
  internal parser storage.
- When documenting public behavior, keep README examples and
  `tests/readme_examples.rs` aligned so examples stay executable.
- When changing CLI behavior, update CLI help expectations and avoid checking in
  generated plot HTML unless the example artifact is intentionally part of the
  change.
- When a malformed file should be tolerated, preserve the issue as a
  `TouchstoneWarning`; when it would make numeric data ambiguous, return a
  contextual `TouchstoneError`.

Keep the repo agent-intuitive by leaving the next person a named path through
the code: document durable workflow changes here, user-facing API behavior in
README, release mechanics in `docs/release.md`, and executable expectations in
tests.

## Commands

```bash
cargo test                        # Run all tests
cargo clippy -- -D warnings       # Lint
cargo fmt -- --check              # Format check
cargo run -- files/ntwk3.s2p      # CLI: plot a single file (opens HTML)
cargo run -- files/               # CLI: plot all files in directory
cargo run -- cascade f1.s2p f2.s2p  # CLI: cascade two 2-port networks
cargo doc --open                  # Generate and view API docs
just cut-release --dry-run --notes-file /tmp/touchstone-release.md  # Preview release
just cut-release --notes-file /tmp/touchstone-release.md            # Cut release
```

## Release Workflow

- Use the `cut-release` skill for ordinary release execution.
- Use the `create-release-process` skill only when changing the release workflow itself.
- Release docs live in `docs/release.md`.
- The checked-in entrypoint is `just cut-release`; it delegates to `scripts/cut-release.sh`.
- Release notes must be written to a local markdown file and passed with `--notes-file`.
- Omitted `--version` infers the next patch version from `Cargo.toml`; pass `--version X.Y.Z` for minor releases.
- The runner updates `Cargo.toml`, `Cargo.lock`, `AGENTS.md`, and `CLAUDE.md`, validates with `just check` and `cargo package`, commits, pushes `main`, and creates the GitHub release. The release event publishes to crates.io.

## Module Map

| Module | File | Description |
|--------|------|-------------|
| `lib` | `src/lib.rs` | `Network` struct — parse, access S-params, cascade, save |
| `parser` | `src/parser.rs` | Touchstone file parser (auto-detects format/ports) |
| `option_line` | `src/option_line.rs` | `#` option line parsing (freq unit, format, Z0) |
| `data_line` | `src/data_line.rs` | `ParsedDataLine` — per-frequency S-parameter data |
| `data_pairs` | `src/data_pairs.rs` | `RealImaginary`, `MagnitudeAngle`, `DecibelAngle` + matrix types |
| `network_builder` | `src/network_builder.rs` | `NetworkBuilder` for generated S-parameter networks |
| `file_extension` | `src/file_extension.rs` | `.sNp` extension detection and port count extraction |
| `utils` | `src/utils.rs` | Math utilities (complex conversions, ABCD ↔ S) |
| `cli` | `src/cli.rs` | CLI entry point (plot, cascade commands) |
| `file_operations` | `src/file_operations.rs` | File I/O utilities |
| `open` | `src/open.rs` | Cross-platform file/URL opening |
| `plot` | `src/plot.rs` | HTML plot generation |

## Change And Test Map

- Parser grammar, option lines, v2 keywords, warnings, or error context:
  `src/parser.rs`, `src/option_line.rs`, `src/data_line.rs`,
  `tests/edge_cases.rs`, `tests/s2p_coverage.rs`, and any new fixture under
  `files/`.
- Data representation, magnitude/angle/dB/RI conversion, matrices, or aliases:
  `src/data_pairs.rs`, `src/utils.rs`, `tests/matrix_api.rs`, and
  `tests/generated_network_contracts.rs`.
- Network construction, serialization, or generated fixtures:
  `src/network_builder.rs`, `src/lib.rs`, and
  `tests/generated_network_contracts.rs`.
- Sampling, resampling, extrapolation, parameter conversion, or cascade:
  `src/lib.rs`, `src/utils.rs`, `tests/network_builder.rs`,
  `tests/in_memory_network.rs`, and `tests/matrix_api.rs`.
- CLI plotting, path handling, cascade command, or diagnostics:
  `src/cli.rs`, `src/file_operations.rs`, `src/plot.rs`, `src/open.rs`, and
  `tests/cli_integration.rs`.
- README examples or public quick-start changes: `README.md` and
  `tests/readme_examples.rs`.

## Key Types

- `Network` — main struct; created via `Network::new(path)`, `Network::from_str(name, contents)`, or `Network::from_bytes(name, bytes)`; has `s_db()`, `s_ri()`, `s_ma()`, `sample_at()`, `resample()`, `cascade()`, and `save()`
- `NetworkBuilder` — generated S-parameter network construction from in-memory matrices
- `ReferenceImpedance` — common or per-port Touchstone v2 reference impedance metadata
- `SMatrix`, `ParameterMatrix`, `ABCDMatrix`, `Complex` — stable matrix and complex value APIs for simulation-oriented workflows
- `Interpolation`, `Extrapolation` — sampling and resampling policy enums
- `FrequencyRI`, `FrequencyDB`, `FrequencyMA` — per-point S-parameter accessors
- S-parameter port indices are **1-indexed** (S₁₁, S₂₁, etc.)
- `Network * Network` — `Mul` trait implements cascade via ABCD

## Where to Look

- **README.md** — Full API examples, file format reference, CLI usage
- **src/lib.rs** — `Network` struct, all public methods, cascade logic, save/load
- **src/data_pairs.rs** — Complex number representations and conversions
- **src/parser.rs** — File parsing logic
- **files/** — Example .s2p/.s3p/.s4p test files
- **tests/** — Integration coverage for fixtures, README examples, CLI behavior,
  generated networks, in-memory parsing, matrices, and builder contracts
