#!/bin/bash

set -uo pipefail

HAS_ERRORS=0

echo "Checking format of workspace..."
echo ""
cargo +nightly fmt --check --all || HAS_ERRORS=1

CONFIG_PATH=$(realpath ./rustfmt.toml)

while read -r CARGO_TOML; do
    DIR=$(dirname "$CARGO_TOML")
    echo "Checking format of $DIR..."
    echo ""
    (cd "$DIR" && cargo +nightly fmt --check --all -- --config-path "$CONFIG_PATH") || HAS_ERRORS=1
done < <(find bin -name "Cargo.toml" -type f)

exit $HAS_ERRORS
