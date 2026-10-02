#!/usr/bin/env python3
"""Manual release/binding workflow contract checker (maintenance-quality M001).

Compares expected target IDs, artifact names, and Rust/tool versions across
the four release/binding workflows and asserts every third-party `uses:`
reference is pinned to a full commit SHA. Manual only: never wire into
required CI without an explicit maintainer decision.

Usage: python3 scripts/check-release-workflow-contract.py
"""

import pathlib
import re
import sys

try:
    import yaml
except ImportError:
    sys.exit("error: pyyaml is required (python3 -m pip install pyyaml)")

ROOT = pathlib.Path(__file__).resolve().parent.parent
WORKFLOWS = ROOT / ".github" / "workflows"

FULL_SHA = re.compile(r"^[^@\s]+@[0-9a-f]{40}$")

EXPECTED_PINS = {
    "actions/checkout": "11d5960a326750d5838078e36cf38b85af677262",
    "actions/upload-artifact": "ea165f8d65b6e75b540449e92b4886f43607fa02",
    "actions/download-artifact": "d3f86a106a0bac45b974a628896c90dbdf5c8093",
    "actions/setup-python": "a26af69be951a213d495a4c3e4e4022e16d87065",
    "actions/setup-node": "49933ea5288caeca8642d1e84afbd3f7d6820020",
    "pnpm/action-setup": "f40ffcd9367d9f12939873eb1018b921a783ffaa",
    "ilammy/msvc-dev-cmd": "0b201ec74fa43914dc39ae48a89fd1d8cb592756",
    "dtolnay/rust-toolchain": "2c7215f132e9ebf062739d9130488b56d53c060c",
}

ZIG_VERSION = "0.14.1"
ZIGBUILD_VERSION = "0.23.3"
ZIG_SHAS = {
    "x86_64-linux": "24aeeec8af16c381934a6cd7d95c807a8cb2cf7df9fa40d359aa884195c4716c",
    "aarch64-linux": "f7a654acc967864f7a050ddacfaa778c7504a0eca8d2b678839c21eea47c992b",
}

failures = []


def fail(message):
    failures.append(message)
    print(f"FAIL: {message}")


def load(name):
    with open(WORKFLOWS / name, encoding="utf-8") as handle:
        return yaml.safe_load(handle)


def uses_refs(document):
    refs = []

    def walk(node):
        if isinstance(node, dict):
            uses = node.get("uses")
            if isinstance(uses, str):
                refs.append(uses)
            for value in node.values():
                walk(value)
        elif isinstance(node, list):
            for value in node:
                walk(value)

    walk(document)
    return refs


def check_pins(name, document):
    for ref in uses_refs(document):
        if ref.startswith("./"):
            continue
        if "@" not in ref:
            fail(f"{name}: unpinned action reference {ref!r}")
            continue
        action, sha = ref.split("@", 1)
        if not FULL_SHA.match(ref):
            fail(f"{name}: non-SHA action reference {ref!r}")
            continue
        expected = EXPECTED_PINS.get(action)
        if expected is None:
            fail(f"{name}: unknown third-party action {action!r} (add to EXPECTED_PINS)")
        elif sha != expected:
            fail(f"{name}: {action} pin {sha} != expected {expected}")


def matrix_ids(document, job):
    matrix = document["jobs"][job]["strategy"]["matrix"]["include"]
    return [row["id"] for row in matrix]


FIVE_WAY = ["linux-x86_64", "linux-aarch64", "macos-x86_64", "macos-arm64", "windows-x86_64"]


def check_text(name, needle):
    text = (WORKFLOWS / name).read_text(encoding="utf-8")
    if needle not in text:
        fail(f"{name}: expected text not found: {needle!r}")
    return text


def main():
    binaries = load("release-binaries.yml")
    python = load("release-python.yml")
    node = load("release-node.yml")
    c_abi = load("release-c.yml")

    for name, document in (
        ("release-binaries.yml", binaries),
        ("release-python.yml", python),
        ("release-node.yml", node),
        ("release-c.yml", c_abi),
    ):
        check_pins(name, document)

    if matrix_ids(python, "build_wheels") != FIVE_WAY:
        fail("release-python.yml build_wheels matrix changed")
    if matrix_ids(python, "verify_smoke") != FIVE_WAY:
        fail("release-python.yml verify_smoke matrix changed")
    if matrix_ids(node, "build_addon") != FIVE_WAY:
        fail("release-node.yml build_addon matrix changed")
    if matrix_ids(c_abi, "build_c") != FIVE_WAY:
        fail("release-c.yml build_c matrix changed")
    if matrix_ids(c_abi, "smoke_artifact") != FIVE_WAY:
        fail("release-c.yml smoke_artifact matrix changed")

    for row in c_abi["jobs"]["build_c"]["strategy"]["matrix"]["include"]:
        if row.get("zigbuild"):
            if row.get("zig_sha") != ZIG_SHAS.get(row.get("zig_arch")):
                fail(f"release-c.yml {row['id']}: zig_sha mismatch for {row.get('zig_arch')}")

    node_text = check_text("release-node.yml", "stegoeggo.linux-x64-gnu.node")
    for artifact in (
        "stegoeggo.linux-arm64-gnu.node",
        "stegoeggo.darwin-x64.node",
        "stegoeggo.darwin-arm64.node",
        "stegoeggo.win32-x64-msvc.node",
    ):
        if artifact not in node_text:
            fail(f"release-node.yml: expected artifact name not found: {artifact!r}")

    binaries_text = check_text("release-binaries.yml", "cargo-zigbuild --version '0.23.3' --locked")
    for arch, sha in (("aarch64-linux", ZIG_SHAS["aarch64-linux"]), ("x86_64-linux", ZIG_SHAS["x86_64-linux"])):
        if f"zig-{arch}-0.14.1.tar.xz" not in binaries_text or sha not in binaries_text:
            fail(f"release-binaries.yml: verified Zig {arch} archive/SHA changed")

    c_text = check_text("release-c.yml", "cargo-zigbuild")
    if "cargo-zigbuild --version '0.23.3' --locked" in c_text:
        fail("release-c.yml: inline cargo-zigbuild install should use the shared action")
    if "ziglang.org/download" in c_text:
        fail("release-c.yml: inline Zig download should use the shared action")
    if "cbindgen --version 0.29.4 --locked" not in c_text:
        fail("release-c.yml: cbindgen 0.29.4 authoritative gate changed")
    if "toolchain: 1.89" not in c_text:
        fail("release-c.yml: Rust 1.89 toolchain selection changed")

    python_text = check_text("release-python.yml", 'python-version: "3.11"')
    if "cibuildwheel==2.22.0" not in python_text:
        fail("release-python.yml: cibuildwheel 2.22.0 pin changed")
    if "maturin==1.5.0" not in python_text:
        fail("release-python.yml: maturin 1.5.0 pin changed")

    if 'version: 12.6.0' not in node_text:
        fail("release-node.yml: pnpm 12.6.0 pin changed")
    for node_line in ('node-version: "22"', 'node-version: "24"', 'node-version: "26"'):
        if node_line not in node_text:
            fail(f"release-node.yml: expected {node_line} smoke coverage changed")

    actions = ROOT / ".github" / "actions"
    zigbuild_action = (actions / "install-cargo-zigbuild" / "action.yml").read_text(encoding="utf-8")
    if f"default: '{ZIGBUILD_VERSION}'" not in zigbuild_action:
        fail("install-cargo-zigbuild default version drifted from 0.23.3")
    zig_action = (actions / "provision-verified-zig" / "action.yml").read_text(encoding="utf-8")
    if f"default: '{ZIG_VERSION}'" not in zig_action:
        fail("provision-verified-zig default version drifted from 0.14.1")

    if failures:
        print(f"\n{len(failures)} contract violation(s)")
        return 1
    print("release workflow contract ok (4 workflows, pins, matrices, toolchains)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
