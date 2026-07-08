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
cut-release *args:
    ./scripts/cut-release.sh {{args}}

# run the CLI against a Touchstone file or directory
dev target="files/ntwk3.s2p":
    cargo run -- "{{target}}"

# build the crate for release
release: build

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
