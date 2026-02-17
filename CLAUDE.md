# CLAUDE.md — touchstone

## Overview

Rust crate for parsing, analyzing, and writing Touchstone (SNP) files — the industry-standard format for S-parameter data. Supports 1-port through N-port (tested to 32-port), all data formats (RI/MA/DB), and 2-port network cascading via ABCD parameters. Published on crates.io (v0.11.1).

## Commands

```bash
cargo test                        # Run all 108 tests
cargo clippy -- -D warnings       # Lint
cargo fmt -- --check              # Format check
cargo run -- files/ntwk3.s2p      # CLI: plot a single file (opens HTML)
cargo run -- files/               # CLI: plot all files in directory
cargo run -- cascade f1.s2p f2.s2p  # CLI: cascade two 2-port networks
cargo doc --open                  # Generate and view API docs
```

## Module Map

| Module | File | Description |
|--------|------|-------------|
| `lib` | `src/lib.rs` | `Network` struct — parse, access S-params, cascade, save |
| `parser` | `src/parser.rs` | Touchstone file parser (auto-detects format/ports) |
| `option_line` | `src/option_line.rs` | `#` option line parsing (freq unit, format, Z0) |
| `data_line` | `src/data_line.rs` | `ParsedDataLine` — per-frequency S-parameter data |
| `data_pairs` | `src/data_pairs.rs` | `RealImaginary`, `MagnitudeAngle`, `DecibelAngle` + matrix types |
| `file_extension` | `src/file_extension.rs` | `.sNp` extension detection and port count extraction |
| `utils` | `src/utils.rs` | Math utilities (complex conversions, ABCD ↔ S) |
| `cli` | `src/cli.rs` | CLI entry point (plot, cascade commands) |
| `plot` | `src/plot.rs` | HTML plot generation |

## Key Types

- `Network` — main struct; created via `Network::new(path)`, has `s_db()`, `s_ri()`, `s_ma()`, `cascade()`, `save()`
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
