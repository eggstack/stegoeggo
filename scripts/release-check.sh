#!/usr/bin/env bash
# Bounded local pre-release readiness check.
# Runs fast checks and package dry-runs. Never publishes, creates tags, or pushes.
set -euo pipefail

ALLOW_DIRTY=false
SKIP_CHECK=false

for arg in "$@"; do
    case "$arg" in
        --allow-dirty) ALLOW_DIRTY=true ;;
        --skip-check) SKIP_CHECK=true ;;
        *)
            echo "Unknown argument: $arg" >&2
            echo "Usage: $0 [--allow-dirty] [--skip-check]" >&2
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

echo ""
echo "=== Package dry-run: stegoeggo-stego ==="
ALLOW_DIRTY_FLAG=""
if [ "$ALLOW_DIRTY" = true ]; then
    ALLOW_DIRTY_FLAG="--allow-dirty"
fi
cargo package -p stegoeggo-stego $ALLOW_DIRTY_FLAG 2>&1 | tail -3

echo ""
echo "=== Package dry-run: stegoeggo ==="
cargo package -p stegoeggo $ALLOW_DIRTY_FLAG 2>&1 | tail -3

echo ""
echo "=== Package dry-run: stegoeggo-cli ==="
if cargo package -p stegoeggo-cli $ALLOW_DIRTY_FLAG 2>&1 | tail -3; then
    echo "CLI package dry-run: OK"
else
    echo ""
    echo "WARNING: CLI dry-run failed. This is expected before library publication."
    echo "The CLI depends on stegoeggo via exact version (=), which must exist on crates.io first."
    echo "Library dry-run succeeded above. CLI dry-run will succeed after library publication."
fi

echo ""
echo "=== Package contents: stegoeggo-stego ==="
cargo package -p stegoeggo-stego --list 2>/dev/null

echo ""
echo "=== Package contents: stegoeggo ==="
cargo package -p stegoeggo --list 2>/dev/null

echo ""
echo "=== Release check passed ==="
