#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

python3 - "$ROOT_DIR" <<'PY'
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
files = {
    name: (root / name).read_text()
    for name in (
        "README.md",
        "docs/installation.md",
        "docs/cli-usage.md",
        "RELEASING.md",
        "SECURITY.md",
        "STABILITY.md",
        "AGENTS.md",
        "architecture/cli.md",
    )
}

installer_url = "https://github.com/eggstack/stegoeggo/releases/latest/download/install.sh"
for name in ("README.md", "docs/installation.md", "docs/cli-usage.md", "AGENTS.md"):
    if installer_url not in files[name]:
        raise SystemExit(f"missing preferred installer URL in {name}")

for command in ("stegoeggo protect", "stegoeggo inspect", "stegoeggo verify"):
    if command not in files["README.md"] or command not in files["docs/cli-usage.md"]:
        raise SystemExit(f"missing canonical command documentation: {command}")

if 'stegoeggo = "0.4"' not in files["README.md"]:
    raise SystemExit("README library dependency is not on the current 0.4 release line")
if "GitHub binary releases are manual but are a supported CLI distribution" not in files["RELEASING.md"]:
    raise SystemExit("RELEASING.md does not describe the binary distribution contract")
if "0.4.x" not in files["SECURITY.md"]:
    raise SystemExit("SECURITY.md supported-version table is stale")

target_rows = []
for raw in (root / "scripts/release-targets.txt").read_text().splitlines():
    if not raw.strip() or raw.lstrip().startswith("#"):
        continue
    fields = raw.split("|")
    if len(fields) != 4:
        raise SystemExit(f"malformed release target row: {raw}")
    target, asset, _, _ = fields
    target_rows.append((target, asset))
    if asset not in files["docs/installation.md"] or asset not in files["architecture/cli.md"]:
        raise SystemExit(f"release asset {asset} is missing from architecture/user docs")

if len(target_rows) != 5:
    raise SystemExit(f"expected five release targets, found {len(target_rows)}")

for name in ("docs/installation.md", "STABILITY.md", "SECURITY.md", "architecture/cli.md"):
    if "sha256" not in files[name].lower().replace("sha-256", "sha256"):
        raise SystemExit(f"checksum contract is missing from {name}")

print(f"Documentation contracts valid: {len(target_rows)} release targets")
PY
