#!/bin/bash

set -uo pipefail

HAS_ERRORS=0

echo "Running \`cargo +nightly fmt --all${@:+ $@}\` in workspace..."
echo
cargo +nightly fmt --all "$@" || HAS_ERRORS=1

exit $HAS_ERRORS
