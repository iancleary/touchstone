#!/bin/bash
set -e

for i in {1..20}; do
    echo "Run #$i"
    cargo test > /dev/null 2>&1
done
echo "All runs passed!"
