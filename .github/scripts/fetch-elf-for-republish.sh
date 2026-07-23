#!/usr/bin/env bash
#
# Fetches the prebuilt guest ELF listed in artifact-registry.json, verifies its
# sha256, and writes OUTPUT_DIR/stateless-validator-<stateless-validator>-<zkvm>.elf.
#
# Usage: fetch-elf-for-republish.sh <stateless-validator> <zkvm>
#   REGISTRY    artifact-registry.json (default: artifact-registry.json)
#   OUTPUT_DIR  output directory (default: output)

set -euo pipefail

NAME="${1:?usage: fetch-elf-for-republish.sh <stateless-validator> <zkvm>}"
ZKVM="${2:?usage: fetch-elf-for-republish.sh <stateless-validator> <zkvm>}"
REGISTRY="${REGISTRY:-artifact-registry.json}"
OUTPUT_DIR="${OUTPUT_DIR:-output}"

entry() {
    jq -r --arg name "$NAME" --arg zkvm "$ZKVM" --arg field "$1" \
        '.stateless_validators[] | select(.name == $name)
         | .elfs[] | select(.zkvm == $zkvm) | .[$field] // empty' "$REGISTRY"
}

WORKSPACE="$(mktemp -d)"
trap 'rm -rf "$WORKSPACE"' EXIT
mkdir -p "$OUTPUT_DIR"

curl -fsSL --proto '=https' --tlsv1.2 --retry 3 "$(entry url)" -o "$WORKSPACE/download"
echo "$(entry sha256) $WORKSPACE/download" | sha256sum -c -

OUT="$OUTPUT_DIR/stateless-validator-$NAME-$ZKVM.elf"
mv "$WORKSPACE/download" "$OUT"

echo "Prepared $OUT"
