#!/bin/bash

# Exit on error
set -e

echo "Updating plots for Touchstone..."

# Update plots for 1-port example
echo "Plotting 1-port file..."
cargo run -- files/hfss_oneport.s1p

# Update plots for 2-port example files
echo "Plotting 2-port files..."
cargo run -- files/ntwk1.s2p
cargo run -- files/ntwk2.s2p
cargo run -- files/ntwk3.s2p

## Plotting currently doesn't support >2 ports
# # Update plots for 3-port example
# echo "Plotting 3-port file..."
# cargo run -- files/hfss_18.2.s3p

# # Update plots for 4-port example
# echo "Plotting 4-port file..."
# cargo run -- files/Agilent_E5071B.s4p

# # Update plots for 8-port example
# echo "Plotting 8-port file..."
# cargo run -- files/hfss_19.2.s8p

# Update cascade example (2-port only)
echo "Generating cascade example..."
# This generates files/cascade_ntwk1_ntwk2.s2p and files/cascade_ntwk1_ntwk2.s2p.html
cargo run -- cascade files/ntwk1.s2p files/ntwk2.s2p --name files/cascade_ntwk1_ntwk2.s2p

# Update directory combined plot
echo "Generating combined directory plot..."
cargo run -- files/test_plot_dir/

echo "All plots updated successfully!"
echo "Generated plots for: 1-port, 2-port, 3-port, 4-port, 8-port, cascade, and combined directory."
