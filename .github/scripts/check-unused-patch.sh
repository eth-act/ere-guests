#!/bin/bash

set -uo pipefail

CARGO_LOCK=""

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --cargo-lock)
            CARGO_LOCK="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--cargo-lock <path>]"
            exit 1
            ;;
    esac
done

check_unused_patch() {
    local CARGO_LOCK="$1"
    local HAS_ERRORS=0

    echo "Checking unused patches in $CARGO_LOCK..."
    echo

    # Check for [[patch.unused]] section in Cargo.lock
    if grep -q "^\[\[patch\.unused\]\]$" "$CARGO_LOCK"; then
        # Extract package names from [[patch.unused]] section
        local UNUSED_PATCHES=$(awk '
            /^\[\[patch\.unused\]\]$/ { in_unused=1; next }
            /^\[\[/ && !/^\[\[patch\.unused\]\]$/ { in_unused=0 }
            in_unused && /^name = / {
                gsub(/^name = "|"$/, "")
                print
            }
        ' "$CARGO_LOCK")

        if [ -n "$UNUSED_PATCHES" ]; then
            echo "The following patches are unused:"
            echo "  $UNUSED_PATCHES" | tr '\n' ' '
            echo
            echo
            HAS_ERRORS=1
        fi
    fi

    return $HAS_ERRORS
}

HAS_ERRORS=0

if [ -n "$CARGO_LOCK" ]; then
    # Check only the specified Cargo.lock file
    if [ ! -f "$CARGO_LOCK" ]; then
        echo "Error: Cargo.lock not found at $CARGO_LOCK"
        exit 1
    fi
    check_unused_patch "$CARGO_LOCK" || HAS_ERRORS=1
else
    # Iterate through all Cargo.lock files in bin directory
    while read -r CARGO_LOCK; do
        check_unused_patch "$CARGO_LOCK" || HAS_ERRORS=1
    done < <(find bin -name "Cargo.lock" -type f)
fi

exit $HAS_ERRORS
