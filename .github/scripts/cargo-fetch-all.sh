#!/bin/bash

set -uo pipefail

HAS_ERRORS=0

echo "Running \`cargo fetch${@:+ $@}\` in workspace..."
echo
cargo fetch "$@" || HAS_ERRORS=1

while read -r CARGO_TOML; do
    DIR=$(dirname "$CARGO_TOML")
    echo "Running \`cargo fetch${@:+ $@}\` in $DIR..."
    echo
    (cd "$DIR" && cargo fetch "$@") || HAS_ERRORS=1
done < <(find bin -name "Cargo.toml" -type f)

exit $HAS_ERRORS
