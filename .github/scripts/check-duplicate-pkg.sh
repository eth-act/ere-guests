#!/bin/bash

set -uo pipefail

INCLUDE=""
EXCLUDE=""
CARGO_LOCK=""

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --include)
            INCLUDE="$2"
            shift 2
            ;;
        --exclude)
            EXCLUDE="$2"
            shift 2
            ;;
        --cargo-lock)
            CARGO_LOCK="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 --include <regex-pattern> [--exclude <regex-pattern>] [--cargo-lock <path>]"
            exit 1
            ;;
    esac
done

if [ -z "$INCLUDE" ]; then
    echo "Error: --include <regex-pattern> is required"
    echo "Usage: $0 --include <regex-pattern> [--exclude <regex-pattern>] [--cargo-lock <path>]"
    exit 1
fi

check_duplicate_pkg() {
    local CARGO_LOCK="$1"
    local HAS_DUPLICATES=0

    echo "Checking duplicate packages in $CARGO_LOCK..."
    echo

    # Get all package names matching the regex pattern
    local MATCHING_PACKAGES
    if [ -n "$EXCLUDE" ]; then
        MATCHING_PACKAGES=$(grep '^name = "' "$CARGO_LOCK" | sed 's/^name = "//; s/"$//' | grep -E "$INCLUDE" | grep -v -E "$EXCLUDE" | sort -u || true)
    else
        MATCHING_PACKAGES=$(grep '^name = "' "$CARGO_LOCK" | sed 's/^name = "//; s/"$//' | grep -E "$INCLUDE" | sort -u || true)
    fi

    if [ -z "$MATCHING_PACKAGES" ]; then
        echo "No packages matching pattern found"
        echo
        return 0
    fi

    echo "Checking packages matching pattern:"
    echo "  $MATCHING_PACKAGES" | tr '\n' ' '
    echo
    echo

    # Single pass through Cargo.lock to collect all package info
    local RESULTS
    RESULTS=$(awk -v pkgs="$MATCHING_PACKAGES" '
        BEGIN {
            split(pkgs, pkg_array, "\n")
            for (i in pkg_array) {
                if (pkg_array[i] != "") {
                    pkg_map[pkg_array[i]] = 1
                }
            }
        }
        /^\[\[package\]\]/ { in_pkg=1; name=""; version=""; source="" }
        in_pkg && /^name = / { gsub(/^name = "|"$/, ""); name=$0 }
        in_pkg && /^version = / { gsub(/^version = "|"$/, ""); version=$0 }
        in_pkg && /^source = / { gsub(/^source = "|"$/, ""); source=$0 }
        in_pkg && /^$/ {
            if (name in pkg_map) {
                pkg_versions[name]++
                pkg_info[name, pkg_versions[name]] = sprintf("version: %s\nsource: %s", version, (source ? source : "local"))
            }
            in_pkg=0
        }
        END {
            for (pkg in pkg_versions) {
                if (pkg_versions[pkg] > 1) {
                    printf "Package '\''%s'\'' has more than 1 versions\n\n", pkg
                    for (i = 1; i <= pkg_versions[pkg]; i++) {
                        print pkg_info[pkg, i]
                        print ""
                    }
                }
            }
        }
    ' "$CARGO_LOCK")

    if [ -n "$RESULTS" ]; then
        echo "$RESULTS"
        echo
        HAS_DUPLICATES=1
    fi

    return $HAS_DUPLICATES
}

HAS_DUPLICATES=0

if [ -n "$CARGO_LOCK" ]; then
    # Check only the specified Cargo.lock file
    if [ ! -f "$CARGO_LOCK" ]; then
        echo "Error: Cargo.lock not found at $CARGO_LOCK"
        exit 1
    fi
    check_duplicate_pkg "$CARGO_LOCK" || HAS_DUPLICATES=1
else
    # Iterate through all Cargo.lock files in bin directory
    while read -r CARGO_LOCK; do
        check_duplicate_pkg "$CARGO_LOCK" || HAS_DUPLICATES=1
    done < <(find bin -name "Cargo.lock" -type f)
fi

exit $HAS_DUPLICATES
