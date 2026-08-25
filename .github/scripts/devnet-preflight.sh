#!/usr/bin/env bash
#
# Probes the devnet batch catalog and emits an artifact-registry-driven Actions matrix.
# A missing or empty catalog is an expected successful no-op while devnet-8 is unpublished.
#
# Usage: devnet-preflight.sh <catalog-url>
#   REGISTRY             artifact registry (default: artifact-registry.json)
#   DEVNET_CATALOG_PATH  local batch index override for deterministic testing

set -euo pipefail

CATALOG_URL="${1:?usage: devnet-preflight.sh <catalog-url>}"
REGISTRY="${REGISTRY:-artifact-registry.json}"
WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT
CATALOG="$WORK_DIR/batches.jsonl"

MATRIX=$(jq -ce '{ include: [.stateless_validators[] as $validator | $validator.artifacts[] | { stateless_validator: $validator.name, zkvm: .zkvm, zkvm_version: .zkvm_version }] } | select(.include | length > 0)' "$REGISTRY")
echo "matrix=${MATRIX}" >> "${GITHUB_OUTPUT:-/dev/stdout}"

if [[ -n ${DEVNET_CATALOG_PATH:-} ]]; then
    if [[ -f $DEVNET_CATALOG_PATH ]]; then
        cp "$DEVNET_CATALOG_PATH" "$CATALOG"
        HTTP_STATUS=200
    else
        HTTP_STATUS=000
    fi
else
    HTTP_STATUS=$(curl -sS -L --retry 3 -o "$CATALOG" -w '%{http_code}' "$CATALOG_URL" || true)
fi

if [[ $HTTP_STATUS == 200 ]] && grep -q '[^[:space:]]' "$CATALOG"; then
    echo "available=true" >> "${GITHUB_OUTPUT:-/dev/stdout}"
    if [[ -n ${GITHUB_STEP_SUMMARY:-} ]]; then
        echo "Glamsterdam devnet-8 catalog is available; scheduled execution is enabled." >> "$GITHUB_STEP_SUMMARY"
    fi
else
    echo "available=false" >> "${GITHUB_OUTPUT:-/dev/stdout}"
    if [[ -n ${GITHUB_STEP_SUMMARY:-} ]]; then
        echo "Glamsterdam devnet-8 catalog is not available yet (HTTP ${HTTP_STATUS}); this run is an expected no-op." >> "$GITHUB_STEP_SUMMARY"
    fi
fi
