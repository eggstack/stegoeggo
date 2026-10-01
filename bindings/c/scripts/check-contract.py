#!/usr/bin/env python3
"""Check the C ABI v1 document against the sorted symbol manifest."""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[3]
ABI_DOC = ROOT / "bindings" / "c" / "ABI-V1.md"
MANIFEST = ROOT / "bindings" / "c" / "abi-v1-symbols.txt"

EXPECTED_TOTAL = 90
EXPECTED_BOOTSTRAP = 3
EXPECTED_V1 = 87


def extract_declared_symbols(text):
    blocks = re.findall(r"```c(.*?)```", text, re.S)
    if not blocks:
        print("error: no fenced C blocks found in ABI-V1.md", file=sys.stderr)
        return None, []
    names = []
    for block in blocks:
        for match in re.finditer(r"\bstegoeggo[_a-z0-9]*\s*\(", block):
            names.append(match.group(0)[:-1].strip())
    return blocks, names


def main():
    doc_text = ABI_DOC.read_text(encoding="utf-8")
    _, names = extract_declared_symbols(doc_text)
    manifest_names = [
        line.strip()
        for line in MANIFEST.read_text(encoding="utf-8").splitlines()
        if line.strip() != ""
    ]

    failures = []

    if len(names) != len(set(names)):
        seen = set()
        duplicates = sorted({n for n in names if n in seen or seen.add(n)})
        failures.append(f"duplicate declarations in ABI-V1.md: {duplicates}")

    declared = sorted(set(names))
    if declared != sorted(manifest_names):
        only_doc = [n for n in declared if n not in set(manifest_names)]
        only_manifest = [n for n in manifest_names if n not in set(declared)]
        if only_doc:
            failures.append(f"in document but not manifest: {only_doc}")
        if only_manifest:
            failures.append(f"in manifest but not document: {only_manifest}")

    if manifest_names != sorted(manifest_names):
        failures.append("manifest is not bytewise sorted")
    if len(set(manifest_names)) != len(manifest_names):
        failures.append("manifest contains duplicate lines")

    bootstrap = [n for n in manifest_names if not n.startswith("stegoeggo_v1_")]
    versioned = [n for n in manifest_names if n.startswith("stegoeggo_v1_")]
    if len(manifest_names) != EXPECTED_TOTAL:
        failures.append(
            f"manifest total {len(manifest_names)} != {EXPECTED_TOTAL}"
        )
    if len(bootstrap) != EXPECTED_BOOTSTRAP:
        failures.append(
            f"bootstrap count {len(bootstrap)} != {EXPECTED_BOOTSTRAP}: {bootstrap}"
        )
    if len(versioned) != EXPECTED_V1:
        failures.append(f"v1 count {len(versioned)} != {EXPECTED_V1}")
    unexpected = [
        n for n in manifest_names if not n.startswith("stegoeggo_")
    ]
    if unexpected:
        failures.append(f"non-stegoeggo names in manifest: {unexpected}")

    if failures:
        for failure in failures:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(
        f"ABI v1 contract OK: {len(manifest_names)} total "
        f"({len(bootstrap)} bootstrap + {len(versioned)} v1)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
