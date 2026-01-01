#!/bin/bash

set -euo pipefail

if [ $# -lt 2 ]; then
    echo "Usage: $0 <cargo-lock-path> <regex-pattern> [exclude-regex-pattern]"
    exit 1
fi

CARGO_LOCK="$1"
REGEX_PATTERN="$2"
EXCLUDE_REGEX_PATTERN="${3:-}"

[ ! -f "$CARGO_LOCK" ] && echo "$CARGO_LOCK not found" && exit 1

# Get all package names matching the regex pattern
if [ -n "$EXCLUDE_REGEX_PATTERN" ]; then
    MATCHING_PACKAGES=$(grep '^name = "' "$CARGO_LOCK" | sed 's/^name = "//; s/"$//' | grep -E "$REGEX_PATTERN" | grep -v -E "$EXCLUDE_REGEX_PATTERN" | sort -u || true)
else
    MATCHING_PACKAGES=$(grep '^name = "' "$CARGO_LOCK" | sed 's/^name = "//; s/"$//' | grep -E "$REGEX_PATTERN" | sort -u || true)
fi

if [ -z "$MATCHING_PACKAGES" ]; then
    echo "No packages matching pattern '$REGEX_PATTERN' found"
    exit 0
fi

echo "Checking packages matching pattern '$REGEX_PATTERN':"
echo "$MATCHING_PACKAGES" | tr '\n' ' '
echo
echo

HAS_DUPLICATES=0

while IFS= read -r pkg; do
    RESULT=$(awk -v pkg="$pkg" '
        /^\[\[package\]\]/ { in_pkg=1; name=""; version=""; source="" }
        in_pkg && /^name = / { gsub(/^name = "|"$/, ""); name=$0 }
        in_pkg && /^version = / { gsub(/^version = "|"$/, ""); version=$0 }
        in_pkg && /^source = / { gsub(/^source = "|"$/, ""); source=$0 }
        in_pkg && /^$/ {
            if (name == pkg) {
                printf "version: %s\nsource: %s\n\n", version, (source ? source : "local")
            }
            in_pkg=0
        }
    ' "$CARGO_LOCK")

    COUNT=$(echo "$RESULT" | grep -c "^version:" || true)

    if [ "$COUNT" -gt 1 ]; then
        echo "Package '$pkg' has more than 1 versions"
        echo ""
        echo "$RESULT"
        HAS_DUPLICATES=1
    fi
done <<< "$MATCHING_PACKAGES"

exit $HAS_DUPLICATES
