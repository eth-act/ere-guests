#!/usr/bin/env bash
#
# Prints the SDK version of <zkvm> resolved by the ere-catalog build script.
#
# Usage: zkvm-version.sh <zkvm>

set -euo pipefail

ZKVM="${1:?usage: zkvm-version.sh <zkvm>}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

# Rebuild from scratch so a stale build directory of an earlier Cargo.lock cannot win below.
cargo clean --quiet --package ere-catalog
cargo build --quiet --package ere-catalog

IMPL=$(find target/debug/build -path '*/out/zkvm_sdk_version_impl.rs' -printf '%T@ %p\n' |
    sort -rn | head -1 | cut -d' ' -f2-)
if [[ -z $IMPL ]]; then
    echo "zkvm_sdk_version_impl.rs not found after building ere-catalog" >&2
    exit 1
fi

# The generated impl holds one `Self::<Kind> => "<version>",` arm per zkVM. A zkVM without an arm
# leaves VERSION empty rather than aborting the pipeline, so the check below reports it.
VERSION=$(grep -i "Self::$ZKVM =>" "$IMPL" | cut -d'"' -f2 || true)
if [[ -z $VERSION ]]; then
    echo "No SDK version of $ZKVM in $IMPL" >&2
    exit 1
fi

echo "$VERSION"
