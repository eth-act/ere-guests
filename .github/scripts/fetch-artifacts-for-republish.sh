#!/usr/bin/env bash
#
# Fetches the prebuilt guest artifacts listed in artifact-registry.json, verifies
# their sha256, and writes them as
# OUTPUT_DIR/stateless-validator-<stateless-validator>-<zkvm>-<zkvm-version>.{elf,vk}.
#
# The zkVM version is the zkvm_version of the registry entry.
#
# Usage: fetch-artifacts-for-republish.sh <stateless-validator> <zkvm>
#   REGISTRY    artifact-registry.json (default: artifact-registry.json)
#   OUTPUT_DIR  output directory (default: output)

set -euo pipefail

USAGE="usage: fetch-artifacts-for-republish.sh <stateless-validator> <zkvm>"
NAME="${1:?$USAGE}"
ZKVM="${2:?$USAGE}"
REGISTRY="${REGISTRY:-artifact-registry.json}"
OUTPUT_DIR="${OUTPUT_DIR:-output}"

entry() {
    jq -r --arg name "$NAME" --arg zkvm "$ZKVM" --arg field "$1" \
        '.stateless_validators[] | select(.name == $name)
         | .artifacts[] | select(.zkvm == $zkvm) | .[$field] // empty' "$REGISTRY"
}

WORKSPACE="$(mktemp -d)"
trap 'rm -rf "$WORKSPACE"' EXIT
mkdir -p "$OUTPUT_DIR"

# Downloads the artifact at $1, verifies it against sha256 $2, and moves it to $3.
fetch() {
    if [[ -z $1 || -z $2 ]]; then
        echo "Missing registry URL or checksum for $NAME-$ZKVM" >&2
        exit 1
    fi
    curl -fsSL --proto '=https' --tlsv1.2 --retry 3 "$1" -o "$WORKSPACE/download"
    echo "$2 $WORKSPACE/download" | sha256sum -c --strict -
    mv "$WORKSPACE/download" "$3"
    echo "Prepared $3"
}

ZKVM_VERSION="$(entry zkvm_version)"
if [[ -z $ZKVM_VERSION ]]; then
    echo "$NAME-$ZKVM lists no zkvm_version" >&2
    exit 1
fi

OUT="$OUTPUT_DIR/stateless-validator-$NAME-$ZKVM-$ZKVM_VERSION"
fetch "$(entry elf_url)" "$(entry elf_sha256)" "$OUT.elf"
fetch "$(entry vk_url)" "$(entry vk_sha256)" "$OUT.vk"
