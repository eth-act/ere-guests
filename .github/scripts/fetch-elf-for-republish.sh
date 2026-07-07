#!/usr/bin/env bash
#
# Fetches the prebuilt guest ELF for <guest> listed in artifact-registry.json,
# verifies its sha256, and writes OUTPUT_DIR/stateless-validator-<guest>.elf.
#
# Usage: fetch-elf-for-republish.sh <el>-<zkvm>
#   REGISTRY    artifact-registry.json (default: artifact-registry.json)
#   OUTPUT_DIR  output directory (default: output)

set -euo pipefail

GUEST="${1:?usage: fetch-elf-for-republish.sh <el>-<zkvm>}"
REGISTRY="${REGISTRY:-artifact-registry.json}"
OUTPUT_DIR="${OUTPUT_DIR:-output}"

entry() { jq -r --arg g "$GUEST" ".stateless_validator_elf[\$g].$1 // empty" "$REGISTRY"; }

WORKSPACE="$(mktemp -d)"
trap 'rm -rf "$WORKSPACE"' EXIT
mkdir -p "$OUTPUT_DIR"

curl -fsSL --proto '=https' --tlsv1.2 --retry 3 "$(entry url)" -o "$WORKSPACE/download"
echo "$(entry sha256) $WORKSPACE/download" | sha256sum -c -

OUT="$OUTPUT_DIR/stateless-validator-$GUEST.elf"
mv "$WORKSPACE/download" "$OUT"

echo "Prepared $OUT"
