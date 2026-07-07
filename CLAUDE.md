# CLAUDE.md — touchstone

## Overview

Rust crate for parsing, analyzing, and writing Touchstone (SNP) files — the industry-standard format for S-parameter data. Supports 1-port through N-port (tested to 32-port), all data formats (RI/MA/DB), generated networks, interpolation/resampling, reference impedance metadata, network parameter conversions, and 2-port network cascading via ABCD parameters. Published on crates.io; current release target is v0.14.1.

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
- Tests are in `src/lib.rs` and individual module files
