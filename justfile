# list recipes
help:
    just --list

# format the code
fmt:
    cargo fmt

# alias for fmt
format: fmt

# check formatting without writing changes
fmt-check:
    cargo fmt -- --check

# lint the code
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# run tests
test:
    cargo test

# check documentation with rustdoc warnings denied
doc-check:
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

# verify package contents without publishing
package:
    cargo package

# build the crate
build:
    cargo build

# format, lint, test, document, and package like CI
check: fmt-check lint test doc-check package

# run the same checks and build mirrored by CI
ci: check build

# build the crate for release
release:
    cargo build --release

# run the CLI against a Touchstone file or directory
dev target="files/ntwk3.s2p":
    cargo run -- "{{target}}"

# plot a single Touchstone file
plot file="files/ntwk3.s2p":
    cargo run -- "{{file}}"

# plot all Touchstone files in a directory
plot-dir dir="files/":
    cargo run -- "{{dir}}"

# cascade two 2-port networks
cascade first="files/ntwk1.s2p" second="files/ntwk2.s2p":
    cargo run -- cascade "{{first}}" "{{second}}"

# generate and open API docs
doc:
    cargo doc --open

# cut a GitHub/crates.io release; pass args such as --dry-run or --notes-file
cut-release *args:
    scripts/cut-release.sh {{args}}
