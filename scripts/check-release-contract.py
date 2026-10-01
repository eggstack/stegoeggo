#!/usr/bin/env python3
"""Keep the published target/asset matrix aligned across release surfaces.

Eggpack is producer authority since Release-Distribution M002: the canonical
target set, asset names, and checksum sidecars come from
`release/eggpack/distribution.toml`, and the checked-in release workflow is
generated from that configuration (drift-guarded by CI running
`eggpack ci check` at the pinned tool revision). This script therefore
compares product mappings against the Eggpack configuration instead of
duplicating producer facts, and retains only stegoeggo-owned invariants.
"""
from pathlib import Path
import json
import re
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
EGG = ROOT / "release/eggpack"

PUBLIC_ASSETS = {
    "x86_64-unknown-linux-gnu": "stegoeggo-x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu": "stegoeggo-aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin": "stegoeggo-x86_64-apple-darwin",
    "aarch64-apple-darwin": "stegoeggo-aarch64-apple-darwin",
    "x86_64-pc-windows-msvc": "stegoeggo-x86_64-pc-windows-msvc.exe",
}

with open(EGG / "distribution.toml", "rb") as handle:
    contract = tomllib.load(handle)
product = contract["product"]["id"]
if product != "stegoeggo":
    print(f"release contract errors:\n- contract product id is {product!r}, want 'stegoeggo'", file=sys.stderr)
    sys.exit(1)
contracted = {}
for target in contract["targets"]:
    triple = target["triple"]
    asset = target["asset"]["asset"].replace("{product}", product).replace("{target}", triple)
    contracted[triple] = asset

workflow = (ROOT / ".github/workflows/release-binaries.yml").read_text()
installer = (ROOT / "packaging/install.sh").read_text()
powershell = (ROOT / "packaging/install.ps1").read_text()
readme = (ROOT / "README.md").read_text()
installation = (ROOT / "docs/installation.md").read_text()
update_rs = (ROOT / "stegoeggo-cli/src/update.rs").read_text()
cli_cargo = (ROOT / "stegoeggo-cli/Cargo.toml").read_text()
policy = json.loads((EGG / "github-policy.json").read_text())
pack = (EGG / "pack.toml").read_text()
qual_bindings = (EGG / "qualification-bindings.toml").read_text()
validators = json.loads((EGG / "consumer-validators.json").read_text())
errors = []

if contracted != PUBLIC_ASSETS:
    errors.append(f"Eggpack contract asset expansion differs from public names: {contracted}")

for target in PUBLIC_ASSETS:
    if target not in workflow:
        errors.append(f"generated workflow does not mention target {target}")

for workflow_file in sorted((ROOT / ".github/workflows").glob("*.yml")):
    text = workflow_file.read_text()
    if re.search(r"cargo install\b.*(?:\s-p\b|\s--package\b)", text):
        errors.append(f"{workflow_file.name} passes -p/--package to cargo install")

if workflow.count("contents: write") != 1:
    errors.append("generated workflow must grant contents: write to exactly one job")

if "release_tag" not in workflow:
    errors.append("generated workflow must accept the exact release_tag dispatch input")
if re.search(r"(?m)^\s*push\s*:", workflow):
    errors.append("generated workflow must not trigger on push during initial adoption")

for forbidden in ["--clobber", "gh release publish", "gh release create --latest",
                  "git tag ", "git push origin --tags", "release publish"]:
    if forbidden in workflow:
        errors.append(f"generated workflow must not contain {forbidden!r}")
if "apt-get install" in workflow or "apt install zig" in workflow:
    errors.append("generated workflow must not install Zig through apt")

revision = policy.get("eggpack_tool", {}).get("revision", "")
if not re.fullmatch(r"[0-9a-f]{40}", revision):
    errors.append("eggpack_tool.revision must be an exact 40-hex commit revision")

for pinned in ['zig = "0.14.1"', 'cargo_zigbuild = "0.23.3"']:
    if pinned not in pack:
        errors.append(f"pack.toml must pin the legacy cross toolchain: {pinned}")

for target in PUBLIC_ASSETS:
    if f'[targets."{target}".smoke]' not in qual_bindings:
        errors.append(f"qualification bindings lack a smoke for {target}")
    if target not in validators:
        errors.append(f"consumer validators lack an entry for {target}")
    validator = validators.get(target, {})
    if validator.get("script") != "scripts/smoke-release-binary.py":
        errors.append(f"consumer validator for {target} must use scripts/smoke-release-binary.py")

for fragment in [
    "Linux:x86_64", "Linux:aarch64", "Darwin:x86_64", "Darwin:arm64",
    "x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin", "aarch64-apple-darwin",
]:
    if fragment not in installer:
        errors.append(f"Unix installer missing mapping fragment {fragment}")
if "x86_64-pc-windows-msvc" not in powershell:
    errors.append("PowerShell installer missing Windows target")
if '"$binaryPath" version' not in powershell and "& $binaryPath version" not in powershell:
    errors.append("PowerShell installer must validate the candidate version identity")
for fragment in ['"$binary_path" version', "stegoeggo-cli --locked"]:
    if fragment not in installer:
        errors.append(f"Unix installer missing contract fragment {fragment}")

for document, name in [(readme, "README"), (installation, "installation docs")]:
    for installer_name in ["install.sh", "install.ps1"]:
        if f"releases/latest/download/{installer_name}" not in document:
            errors.append(f"{name} must advertise the published latest {installer_name} URL")

if 'Command::new("curl")' in update_rs:
    errors.append("stegoeggo-cli/src/update.rs must not spawn curl for self-update networking")
if "eggfetch-core" not in cli_cargo and "eggfetch-core" not in (ROOT / "Cargo.toml").read_text():
    errors.append("workspace must declare the qualified eggfetch-core updater transport")
for target in PUBLIC_ASSETS:
    if target not in update_rs:
        errors.append(f"updater target mapping is missing {target}")
if 'format!("stegoeggo-{target}.exe")' not in update_rs:
    errors.append("updater Windows asset naming drifted")
if 'format!("stegoeggo-{target}")' not in update_rs:
    errors.append("updater Unix asset naming drifted")
if '.args(["version"])' not in update_rs:
    errors.append("updater must validate candidates with the bare version argv")

if errors:
    print("release contract errors:", file=sys.stderr)
    print("\n".join(f"- {error}" for error in errors), file=sys.stderr)
    sys.exit(1)
print("release target/asset contract passed")
