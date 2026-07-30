#!/usr/bin/env bash
#
# Fetches the prebuilt guest artifacts listed in artifact-registry.json, verifies
# their sha256, and writes them as
# OUTPUT_DIR/stateless-validator-<stateless-validator>-<zkvm>-<zkvm-version>.{elf,vk}.
#
# The VK is optional and is written only when the registry lists one,
# leaving the caller to generate it otherwise.
#
# The zkVM version is the one this repository builds against. Once republished
# artifacts carry their own zkVM version, take it from artifact-registry.json instead.
#
# Usage: fetch-artifacts-for-republish.sh <stateless-validator> <zkvm> <zkvm-version>
#   REGISTRY    artifact-registry.json (default: artifact-registry.json)
#   OUTPUT_DIR  output directory (default: output)

set -euo pipefail

USAGE="usage: fetch-artifacts-for-republish.sh <stateless-validator> <zkvm> <zkvm-version>"
NAME="${1:?$USAGE}"
ZKVM="${2:?$USAGE}"
ZKVM_VERSION="${3:?$USAGE}"
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
    curl -fsSL --proto '=https' --tlsv1.2 --retry 3 "$1" -o "$WORKSPACE/download"
    echo "$2 $WORKSPACE/download" | sha256sum -c --strict -
    mv "$WORKSPACE/download" "$3"
    echo "Prepared $3"
}

OUT="$OUTPUT_DIR/stateless-validator-$NAME-$ZKVM-$ZKVM_VERSION"
fetch "$(entry elf_url)" "$(entry elf_sha256)" "$OUT.elf"

VK_URL="$(entry vk_url)"
if [[ -n $VK_URL ]]; then
    fetch "$VK_URL" "$(entry vk_sha256)" "$OUT.vk"
fi
