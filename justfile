# list recipes
help:
    just --list

# format the code
fmt:
    cargo fmt --all

# alias for fmt
format: fmt

# check formatting without writing changes
fmt-check:
    cargo fmt --all -- --check

# lint the code without writing changes
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# apply automatic clippy fixes
lint-fix:
    cargo clippy --all-targets --all-features --fix -- -D warnings

# run tests
test:
    cargo test --all-features

# run the parser mutation regressions included in ordinary CI
test-adversarial:
    cargo test --test parser_adversarial

# bounded coverage-guided parser fuzzing (requires cargo-fuzz and nightly)
[positional-arguments]
fuzz seconds="60":
    #!/bin/sh
    set -eu
    case "$1" in ""|*[!0-9]*) echo "seconds must be an integer from 1 to 3600" >&2; exit 2 ;; esac
    if [ "${#1}" -gt 4 ] || [ "$1" -lt 1 ] || [ "$1" -gt 3600 ]; then
        echo "seconds must be an integer from 1 to 3600" >&2
        exit 2
    fi
    cargo +nightly fuzz run parse -- -max_len=4096 "-max_total_time=$1" -rss_limit_mb=2048

# check documentation with rustdoc warnings denied
doc-check:
    RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps

# verify package contents without publishing
package:
    cargo package

# build the crate
build:
    cargo build --release

# format, lint, test, document, and package like CI
check: fmt-check lint test doc-check package

# run the same checks and build mirrored by CI
ci: check build

# Cut a GitHub release for an explicit SemVer version.
[positional-arguments]
cut-release *args:
    ./scripts/cut-release.sh "$@"

# run the CLI against a Touchstone file or directory
[positional-arguments]
dev target="files/ntwk3.s2p":
    cargo run -- "$1"

# build the crate for release
release: build

# plot a single Touchstone file
[positional-arguments]
plot file="files/ntwk3.s2p":
    cargo run -- "$1"

# plot all Touchstone files in a directory
[positional-arguments]
plot-dir dir="files/":
    cargo run -- "$1"

# cascade two 2-port networks
[positional-arguments]
cascade first="files/ntwk1.s2p" second="files/ntwk2.s2p":
    cargo run -- cascade "$1" "$2"

# generate and open API docs
doc:
    cargo doc --open
