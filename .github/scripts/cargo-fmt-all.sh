#!/bin/bash

set -uo pipefail

HAS_ERRORS=0

echo "Running \`cargo +nightly fmt --all${@:+ $@}\` in workspace..."
echo
cargo +nightly fmt --all "$@" || HAS_ERRORS=1

while read -r CARGO_TOML; do
    DIR=$(dirname "$CARGO_TOML")
    echo "Running \`cargo +nightly fmt --all${@:+ $@}\` in $DIR..."
    echo
    (cd "$DIR" && cargo +nightly fmt --all "$@") || HAS_ERRORS=1
done < <(find bin -name "Cargo.toml" -type f)

exit $HAS_ERRORS
