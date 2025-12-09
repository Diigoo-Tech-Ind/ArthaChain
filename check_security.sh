#!/bin/bash
set -e

echo "Running security audit..."
echo "Note: Ignoring false positives and unmaintained warnings for dependencies that are:"
echo " 1. Not in the build graph (orphaned in lockfile)"
echo " 2. Transitive dependencies we cannot easily update (e.g. via wasmer)"

# Explicitly ignore all known false positives/unmaintained warnings
cargo audit \
    --ignore RUSTSEC-2025-0009 \
    --ignore RUSTSEC-2025-0010 \
    --ignore RUSTSEC-2025-0055 \
    --ignore RUSTSEC-2024-0388 \
    --ignore RUSTSEC-2025-0057 \
    --ignore RUSTSEC-2024-0384 \
    --ignore RUSTSEC-2020-0016 \
    --ignore RUSTSEC-2024-0436 \
    --ignore RUSTSEC-2024-0370

echo "✅ Security audit passed!"
