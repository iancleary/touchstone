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

# build the crate
build:
    cargo build

# build the crate for release
release:
    cargo build --release

# cut a GitHub release
cut-release *args:
    ./scripts/cut-release.sh {{args}}

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

# format, lint, and test
check: fmt-check lint test

# run checks and build
ci: check build
