#!/bin/bash

set -euo pipefail

if [ $# -lt 1 ]; then
    echo "Usage: $0 <path-to-cargo-toml>"
    exit 1
fi

CARGO_TOML="$1"

WARNING=$(cargo tree --manifest-path "$CARGO_TOML" 2>&1 | grep -F "was not used in the crate graph" || true)

if [ -n "$WARNING" ]; then
    echo "$WARNING"
    exit 1
fi
