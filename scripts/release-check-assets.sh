#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ASSET_DIR=""
VERSION=""
NATIVE_SMOKE=false

usage() {
    echo "Usage: $0 --dir=DIRECTORY [--version=X.Y.Z] [--native-smoke]"
}

fail() {
    echo "ERROR: $*" >&2
    exit 1
}

sha256() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | awk '{print $1}'
    else
        fail "neither sha256sum nor shasum is available"
    fi
}

for arg in "$@"; do
    case "$arg" in
        --help|-h) usage; exit 0 ;;
        --dir=*) ASSET_DIR="${arg#--dir=}" ;;
        --version=*) VERSION="${arg#--version=}" ;;
        --native-smoke) NATIVE_SMOKE=true ;;
        *) fail "unknown argument '$arg'" ;;
    esac
done

[[ -n "$ASSET_DIR" ]] || fail "--dir is required"
[[ -d "$ASSET_DIR" ]] || fail "asset directory does not exist: $ASSET_DIR"
if [[ -n "$VERSION" && ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    fail "invalid version '$VERSION'; expected X.Y.Z"
fi

assets=()
while IFS= read -r line; do
    [[ -n "$line" ]] && assets+=("$line")
done < <(python3 - "$ROOT_DIR" <<'PY'
import pathlib
import sys
import tomllib

root = pathlib.Path(sys.argv[1])
with open(root / "release/eggpack/distribution.toml", "rb") as handle:
    contract = tomllib.load(handle)
product = contract["product"]["id"]
for target in contract["targets"]:
    triple = target["triple"]
    print(target["asset"]["asset"].replace("{product}", product).replace("{target}", triple))
PY
)
[[ "${#assets[@]}" -eq 5 ]] || fail "Eggpack contract must define five assets"

count=0
for asset in "${assets[@]}"; do
    binary="$ASSET_DIR/$asset"
    checksum="$binary.sha256"
    [[ -f "$binary" ]] || fail "missing binary asset: $asset"
    [[ -f "$checksum" ]] || fail "missing checksum sidecar: $asset.sha256"
    expected="$(awk 'NF {print tolower($1); exit}' "$checksum")"
    named_asset="$(awk 'NF {print $2; exit}' "$checksum")"
    [[ "$expected" =~ ^[0-9a-f]{64}$ ]] || fail "invalid checksum sidecar: $asset.sha256"
    [[ "${named_asset##*/}" == "$asset" ]] || fail "checksum sidecar names '${named_asset:-nothing}', expected '$asset'"
    actual="$(sha256 "$binary")"
    [[ "$actual" == "$expected" ]] || fail "checksum mismatch: $asset"
    if [[ "$NATIVE_SMOKE" == true ]]; then
        identity="$("$binary" version)" || fail "version command failed: $asset"
        [[ "$identity" =~ ^stegoeggo\ [0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "invalid version identity: $asset"
        "$binary" --help >/dev/null || fail "help command failed: $asset"
        if [[ -n "$VERSION" && "$identity" != "stegoeggo $VERSION" ]]; then
            fail "version identity mismatch for $asset: $identity"
        fi
    fi
    count=$((count + 1))
done

[[ "$count" -eq 5 ]] || fail "expected 5 target assets, found $count"
[[ -f "$ASSET_DIR/install.sh" ]] || fail "missing install.sh"
[[ -f "$ASSET_DIR/install.ps1" ]] || fail "missing install.ps1"
[[ -f "$ASSET_DIR/install-exact.sh" ]] || fail "missing install-exact.sh"
[[ -f "$ASSET_DIR/install-exact.ps1" ]] || fail "missing install-exact.ps1"
[[ -f "$ASSET_DIR/release-manifest.json" ]] || fail "missing release-manifest.json"
for path in "$ASSET_DIR"/*; do
    [[ -f "$path" ]] || fail "asset directory contains a non-file entry: ${path##*/}"
    name="${path##*/}"
    expected_file=false
    for asset in "${assets[@]}"; do
        if [[ "$name" == "$asset" || "$name" == "$asset.sha256" ]]; then
            expected_file=true
            break
        fi
    done
    case "$name" in
        install.sh|install.ps1|install-exact.sh|install-exact.ps1|release-manifest.json)
            expected_file=true
            ;;
    esac
    [[ "$expected_file" == true ]] || fail "unexpected release asset: $name"
done
echo "Release assets valid: $count binaries with SHA-256 sidecars plus Eggpack staging set"
