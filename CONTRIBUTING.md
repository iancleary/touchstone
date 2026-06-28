# Contributing

Thanks for helping improve touchstone. This crate is a Rust project for parsing,
analyzing, writing, and plotting Touchstone SNP files.

## Toolchain

Use the stable Rust toolchain with `rustfmt` and `clippy` installed:

```bash
rustup update stable
rustup default stable
rustup component add rustfmt clippy
```

This repository uses `just` as the task runner. Install it with Cargo if it is
not already available:

```bash
cargo install just --locked
```

List available project commands with:

```bash
just --list
```

## Local checks

Before opening a pull request, run the same checks that CI runs:

```bash
just ci
```

`just ci` runs formatting verification, clippy with warnings denied, tests, and
a debug build. The individual commands are also available:

```bash
just fmt-check
just lint
just test
just build
```

Use `just fmt` to apply Rust formatting before committing.

## Pull requests

- Keep changes focused and include tests when behavior changes.
- Update documentation or examples when public APIs or CLI behavior changes.
- Do not commit generated plot HTML or local build artifacts unless they are
  intentionally part of the change.
- Include enough context in the pull request description for reviewers to
  understand the motivation and validation performed.
