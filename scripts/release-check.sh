#!/usr/bin/env bash
# Bounded local pre-release readiness check.
# Runs fast checks and package dry-runs. Never publishes, creates tags, or pushes.
set -euo pipefail

ALLOW_DIRTY=false
SKIP_CHECK=false
STAGE=pre

for arg in "$@"; do
    case "$arg" in
        --allow-dirty) ALLOW_DIRTY=true ;;
        --skip-check) SKIP_CHECK=true ;;
        --stage)
            echo "ERROR: --stage requires pre, root, or cli" >&2
            exit 1
            ;;
        --stage=pre) STAGE=pre ;;
        --stage=root) STAGE=root ;;
        --stage=cli) STAGE=cli ;;
        *)
            echo "Unknown argument: $arg" >&2
            echo "Usage: $0 [--allow-dirty] [--skip-check] [--stage=pre|root|cli]" >&2
            exit 1
            ;;
    esac
done

run() {
    printf '\n==> '
    printf '%q ' "$@"
    printf '\n'
    "$@"
}

if [ "$ALLOW_DIRTY" = false ]; then
    if [ -n "$(git status --porcelain)" ]; then
        echo "ERROR: Working tree is dirty. Use --allow-dirty to override." >&2
        git status --porcelain
        exit 1
    fi
fi

if [ "$SKIP_CHECK" = false ]; then
    run ./scripts/check.sh
fi

echo ""
echo "=== Version lockstep check ==="

python3 - <<'PY'
import json
import subprocess
import sys

metadata = json.loads(subprocess.check_output([
    "cargo", "metadata", "--no-deps", "--format-version", "1"
]))
packages = {pkg["name"]: pkg for pkg in metadata["packages"]}
carrier = packages["stegoeggo-stego"]
lib = packages["stegoeggo"]
cli = packages["stegoeggo-cli"]

if lib["version"] != cli["version"]:
    print(f"ERROR: version mismatch: stegoeggo={lib['version']} stegoeggo-cli={cli['version']}", file=sys.stderr)
    sys.exit(1)

lib_carrier_dep = None
for dep in lib["dependencies"]:
    if dep["name"] == "stegoeggo-stego":
        lib_carrier_dep = dep["req"]
        break

if lib_carrier_dep is None:
    print("ERROR: stegoeggo does not depend on stegoeggo-stego", file=sys.stderr)
    sys.exit(1)

expected_carrier_req = f"={carrier['version']}"
if lib_carrier_dep != expected_carrier_req:
    print(f"ERROR: library carrier dependency requirement is '{lib_carrier_dep}', expected '{expected_carrier_req}'", file=sys.stderr)
    sys.exit(1)

cli_stegoeggo_dep = None
for dep in cli["dependencies"]:
    if dep["name"] == "stegoeggo":
        cli_stegoeggo_dep = dep["req"]
        break

if cli_stegoeggo_dep is None:
    print("ERROR: stegoeggo-cli does not depend on stegoeggo", file=sys.stderr)
    sys.exit(1)

expected_req = f"={lib['version']}"
if cli_stegoeggo_dep != expected_req:
    print(f"ERROR: CLI dependency requirement is '{cli_stegoeggo_dep}', expected '{expected_req}'", file=sys.stderr)
    sys.exit(1)

print(f"Carrier version: {carrier['version']}")
print(f"Library version: {lib['version']}")
print(f"CLI version: {cli['version']}")
print(f"Library carrier dependency: {lib_carrier_dep}")
print(f"CLI library dependency: {cli_stegoeggo_dep}")
PY

ALLOW_DIRTY_FLAG=""
if [ "$ALLOW_DIRTY" = true ]; then
    ALLOW_DIRTY_FLAG="--allow-dirty"
fi
if [ "$STAGE" = "pre" ]; then
    echo ""
    echo "=== Full package verification: stegoeggo-stego ==="
    cargo package -p stegoeggo-stego $ALLOW_DIRTY_FLAG

    echo ""
    echo "=== Structural package validation: stegoeggo ==="
    cargo package -p stegoeggo --list $ALLOW_DIRTY_FLAG >/dev/null

    echo ""
    echo "=== Structural package validation: stegoeggo-cli ==="
    cargo package -p stegoeggo-cli --list $ALLOW_DIRTY_FLAG >/dev/null
elif [ "$STAGE" = "root" ]; then
    echo ""
    echo "=== Full package verification: stegoeggo ==="
    cargo package -p stegoeggo $ALLOW_DIRTY_FLAG
else
    echo ""
    echo "=== Full package verification: stegoeggo-cli ==="
    cargo package -p stegoeggo-cli $ALLOW_DIRTY_FLAG
fi

echo ""
echo "=== Package contents: stegoeggo-stego ==="
cargo package -p stegoeggo-stego --list $ALLOW_DIRTY_FLAG 2>/dev/null

echo ""
echo "=== Package contents: stegoeggo ==="
cargo package -p stegoeggo --list $ALLOW_DIRTY_FLAG 2>/dev/null

echo ""
echo "=== Package contents: stegoeggo-cli ==="
cargo package -p stegoeggo-cli --list $ALLOW_DIRTY_FLAG 2>/dev/null

echo ""
echo "=== Release check passed ==="
