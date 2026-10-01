#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

python3 - "$ROOT_DIR" <<'PY'
import json
import pathlib
import re
import sys
import tomllib

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
        ".github/workflows/release-binaries.yml",
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

with open(root / "release/eggpack/distribution.toml", "rb") as handle:
    contract = tomllib.load(handle)
if contract["product"]["id"] != "stegoeggo":
    raise SystemExit("Eggpack contract product id must be stegoeggo")
targets = []
for entry in contract["targets"]:
    triple = entry["triple"]
    asset = entry["asset"]["asset"].replace("{product}", "stegoeggo").replace("{target}", triple)
    targets.append((triple, asset))
if len(targets) != 5 or len({triple for triple, _ in targets}) != 5:
    raise SystemExit("Eggpack contract must define exactly five unique targets")

workflow = files[".github/workflows/release-binaries.yml"]
for triple, asset in targets:
    if asset not in files["docs/installation.md"] or asset not in files["architecture/cli.md"]:
        raise SystemExit(f"release asset {asset} is missing from architecture/user docs")
    if triple not in workflow:
        raise SystemExit(f"generated workflow does not mention target {triple}")

if "release_tag" not in workflow:
    raise SystemExit("generated workflow must accept the exact release_tag dispatch input")
if re.search(r"(?m)^\s*push\s*:", workflow):
    raise SystemExit("generated workflow must not trigger on push")
if workflow.count("contents: write") != 1:
    raise SystemExit("generated workflow must grant contents: write to exactly one job")
for forbidden in ("--clobber", "gh release publish", "gh release create --latest"):
    if forbidden in workflow:
        raise SystemExit(f"generated workflow must not contain {forbidden!r}")
if re.search(r"cargo install\b.*(?:\s-p\b|\s--package\b)", workflow):
    raise SystemExit("generated workflow passes -p/--package to cargo install")
if "apt-get install" in workflow or "apt install zig" in workflow:
    raise SystemExit("generated workflow must not install Zig through apt")
if "_stage-github-draft" not in workflow:
    raise SystemExit("generated workflow must stage through the Eggpack draft path")

policy = json.loads((root / "release/eggpack/github-policy.json").read_text())
revision = policy.get("eggpack_tool", {}).get("revision", "")
if not re.fullmatch(r"[0-9a-f]{40}", revision):
    raise SystemExit("eggpack_tool.revision must be an exact 40-hex commit revision")
if revision not in workflow:
    raise SystemExit("generated workflow does not install the pinned Eggpack revision")

for name in ("docs/installation.md", "STABILITY.md", "SECURITY.md", "architecture/cli.md"):
    if "sha256" not in files[name].lower().replace("sha-256", "sha256"):
        raise SystemExit(f"checksum contract is missing from {name}")

print(f"Documentation contracts valid: {len(targets)} release targets")
PY
