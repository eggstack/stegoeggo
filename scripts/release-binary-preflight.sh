#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGETS_FILE="$ROOT_DIR/scripts/release-targets.txt"
TAG=""
ASSET_DIR=""
ALLOW_DIRTY=false
SKIP_CHECK=false

usage() {
    cat <<'EOF'
Usage: release-binary-preflight.sh --tag=vX.Y.Z [options]

Validate a manually dispatched GitHub binary release without publishing or
uploading anything.

Options:
  --tag=vX.Y.Z       Release tag; when omitted, use an exact tag at HEAD
  --asset-dir=DIR    Also validate built executable assets and sidecars
  --allow-dirty      Allow local changes (useful for pre-tag local checks)
  --skip-check       Skip ./scripts/check.sh
  --help             Show this help
EOF
}

fail() {
    echo "ERROR: $*" >&2
    exit 1
}

while (($# > 0)); do
    case "$1" in
        --help|-h) usage; exit 0 ;;
        --tag=*) TAG="${1#--tag=}"; shift ;;
        --tag) (($# >= 2)) || fail "--tag requires vX.Y.Z"; TAG="$2"; shift 2 ;;
        --asset-dir=*) ASSET_DIR="${1#--asset-dir=}"; shift ;;
        --asset-dir) (($# >= 2)) || fail "--asset-dir requires a directory"; ASSET_DIR="$2"; shift 2 ;;
        --allow-dirty) ALLOW_DIRTY=true; shift ;;
        --skip-check) SKIP_CHECK=true; shift ;;
        *) fail "unknown argument '$1'" ;;
    esac
done

if [[ -z "$TAG" ]]; then
    TAG="$(git -C "$ROOT_DIR" describe --exact-match --tags HEAD 2>/dev/null || true)"
    [[ -n "$TAG" ]] || fail "HEAD is not at an exact release tag; pass --tag=vX.Y.Z"
fi
[[ "$TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "invalid release tag '$TAG'"

if [[ "$ALLOW_DIRTY" == false && -n "$(git -C "$ROOT_DIR" status --porcelain)" ]]; then
    fail "working tree is dirty; use --allow-dirty to override"
fi
head_commit="$(git -C "$ROOT_DIR" rev-parse HEAD)"
tag_commit="$(git -C "$ROOT_DIR" rev-parse --verify "$TAG^{commit}" 2>/dev/null || true)"
[[ -n "$tag_commit" ]] || fail "tag does not exist locally: $TAG"
[[ "$head_commit" == "$tag_commit" ]] || fail "$TAG does not point to HEAD"

if [[ "$SKIP_CHECK" == false ]]; then
    "$ROOT_DIR/scripts/check.sh"
fi

python3 - "$ROOT_DIR" "$TARGETS_FILE" "${TAG#v}" <<'PY'
import json
import pathlib
import subprocess
import sys

root = pathlib.Path(sys.argv[1])
targets_file = pathlib.Path(sys.argv[2])
tag_version = sys.argv[3]
workflow_text = (root / ".github/workflows/release-binaries.yml").read_text()
update_text = (root / "stegoeggo-cli/src/update.rs").read_text()
metadata = json.loads(subprocess.check_output([
    "cargo", "metadata", "--no-deps", "--format-version", "1",
], cwd=root))
packages = {package["name"]: package for package in metadata["packages"]}
carrier = packages["stegoeggo-stego"]
library = packages["stegoeggo"]
cli = packages["stegoeggo-cli"]

if not (carrier["version"] == library["version"] == cli["version"]):
    raise SystemExit("ERROR: carrier, library, and CLI versions are not in lockstep")
if carrier["version"] != tag_version:
    raise SystemExit(f"ERROR: tag version {tag_version} does not match workspace version {carrier['version']}")

def dependency_req(package, name):
    return next((dependency["req"] for dependency in package["dependencies"] if dependency["name"] == name), None)

if dependency_req(library, "stegoeggo-stego") != f'={carrier["version"]}':
    raise SystemExit("ERROR: library carrier dependency is not exact and lockstep")
if dependency_req(cli, "stegoeggo") != f'={library["version"]}':
    raise SystemExit("ERROR: CLI library dependency is not exact and lockstep")
if cli["features"].get("default") != ["signatures"]:
    raise SystemExit("ERROR: CLI default feature set must be exactly signatures")
if cli["features"].get("signatures") != ["stegoeggo/signatures", "stegoeggo/detached-manifest"]:
    raise SystemExit("ERROR: CLI signatures feature set changed unexpectedly")

rows = []
for raw in targets_file.read_text().splitlines():
    if not raw.strip() or raw.lstrip().startswith("#"):
        continue
    fields = raw.split("|")
    if len(fields) != 4:
        raise SystemExit(f"ERROR: malformed target row: {raw}")
    target, asset, runner, build = fields
    expected_asset = f"stegoeggo-{target}" + (".exe" if target.endswith("-pc-windows-msvc") else "")
    if asset != expected_asset:
        raise SystemExit(f"ERROR: target {target} must use asset {expected_asset}")
    if workflow_text.count(f"target: {target}") != 1:
        raise SystemExit(f"ERROR: workflow target matrix drifted for {target}")
    if workflow_text.count(f"asset: {asset}") != 1:
        raise SystemExit(f"ERROR: workflow asset matrix drifted for {asset}")
    if target not in update_text:
        raise SystemExit(f"ERROR: updater target mapping is missing {target}")
    rows.append((target, asset))
if len(rows) != 5 or len({target for target, _ in rows}) != 5 or len({asset for _, asset in rows}) != 5:
    raise SystemExit("ERROR: target manifest must contain five unique targets and assets")
if 'format!("stegoeggo-{target}.exe")' not in update_text:
    raise SystemExit("ERROR: updater Windows asset naming drifted")
if 'format!("stegoeggo-{target}")' not in update_text:
    raise SystemExit("ERROR: updater Unix asset naming drifted")

print(f"Version lockstep: {carrier['version']}")
print("CLI distributed features: signatures")
print(f"Binary targets: {len(rows)}")
PY

[[ -f "$ROOT_DIR/packaging/install.sh" ]] || fail "missing packaging/install.sh"
[[ -f "$ROOT_DIR/packaging/install.ps1" ]] || fail "missing packaging/install.ps1"
if [[ -n "$ASSET_DIR" ]]; then
    "$ROOT_DIR/scripts/release-check-assets.sh" --dir="$ASSET_DIR" --version="${TAG#v}"
fi
echo "Binary release preflight passed for $TAG"
