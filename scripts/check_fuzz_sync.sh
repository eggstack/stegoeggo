#!/usr/bin/env bash
# check_fuzz_sync.sh — Verify fuzz targets in Cargo.toml match fuzz.yml workflow
#
# Exit codes:
#   0 — Targets synchronized
#   1 — Mismatch detected

set -euo pipefail

CARGO_TOML="fuzz/Cargo.toml"
FUZZ_YML=".github/workflows/fuzz.yml"

if [ ! -f "$CARGO_TOML" ]; then
    echo "Error: $CARGO_TOML not found" >&2
    exit 1
fi

if [ ! -f "$FUZZ_YML" ]; then
    echo "Error: $FUZZ_YML not found" >&2
    exit 1
fi

# Extract [[bin]] name values from fuzz/Cargo.toml
# Skip the first match (package name) and extract only bin targets
cargo_targets=$(awk -F'"' '/^\[\[bin\]\]/{found=1; next} found && /^name = /{print $2; found=0}' "$CARGO_TOML" | sort)

# Extract target names from the workflow_dispatch target options.
workflow_targets=$(awk '
    /^      target:$/ { in_target = 1; next }
    in_target && /^      seconds:/ { in_target = 0 }
    in_target && /^          - [a-z0-9_]+$/ { print $2 }
' "$FUZZ_YML" | sort -u)

if [ "$cargo_targets" = "$workflow_targets" ]; then
    echo "Fuzz targets synchronized: $(echo "$cargo_targets" | wc -l) targets"
    exit 0
else
    echo "MISMATCH DETECTED" >&2
    echo "" >&2
    echo "In Cargo.toml but not in workflow:" >&2
    comm -23 <(echo "$cargo_targets") <(echo "$workflow_targets") >&2
    echo "" >&2
    echo "In workflow but not in Cargo.toml:" >&2
    comm -13 <(echo "$cargo_targets") <(echo "$workflow_targets") >&2
    exit 1
fi
