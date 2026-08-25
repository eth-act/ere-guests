#!/bin/bash

set -uo pipefail

HAS_ERRORS=0

echo "Running \`cargo fetch${@:+ $@}\` in workspace..."
echo
cargo fetch "$@" || HAS_ERRORS=1

exit $HAS_ERRORS
