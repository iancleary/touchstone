#!/bin/bash

# Exit on error
set -e

echo "Updating plots for Touchstone..."

# Update plots for 1-port example
echo "Plotting 1-port file..."
cargo run -- files/hfss_oneport.s1p

# Update plots for root-level 2-port example files.
# The cascade output is regenerated below, and noise-parameter fixtures are
# parser coverage inputs rather than currently plot-capable examples.
echo "Plotting 2-port files..."
while IFS= read -r s2p_file; do
    case "$(basename "$s2p_file")" in
        cascade_ntwk1_ntwk2.s2p)
            continue
            ;;
        ntwk4_n.s2p|ntwk_noise.s2p|ntwk_noise_interp.s2p|thru.s2p)
            echo "Skipping unsupported noise-parameter fixture: $s2p_file"
            continue
            ;;
    esac

    cargo run -- "$s2p_file"
done < <(find files -maxdepth 1 -type f -name "*.s2p" | sort)

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
echo "Generated plots for: 1-port, all root-level 2-port files, cascade, and combined directory."
