#!/bin/bash

# Exit on error
set -e

echo "Updating plots for Touchstone..."

# Update plots for individual example files
cargo run -- files/ntwk1.s2p
cargo run -- files/ntwk2.s2p
cargo run -- files/ntwk3.s2p

# Update cascade example
# This generates files/cascade_ntwk1_ntwk2.s2p and files/cascade_ntwk1_ntwk2.s2p.html
cargo run -- cascade files/ntwk1.s2p files/ntwk2.s2p --name files/cascade_ntwk1_ntwk2.s2p

# Update directory combined plot
cargo run -- files/

echo "All plots updated successfully."
