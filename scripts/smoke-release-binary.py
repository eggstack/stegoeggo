#!/usr/bin/env python3
"""Bounded StegoEggo release-candidate product validator.

Receives only the exact Eggpack candidate path and uses checked-in source
fixtures/config. Enforces exact version identity, help, protect/inspect/verify
semantics, and the Linux GLIBC 2.17 symbol ceiling.
"""
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "tests/fixtures/conformance/canonical/canonical_policy_only.png"
TIMEOUT_SECS = 120
MAX_OUTPUT = 256 * 1024


def fail(message):
    print(f"smoke-release-binary: {message}", file=sys.stderr)
    return 1


def run_bounded(argv, timeout=TIMEOUT_SECS):
    try:
        completed = subprocess.run(
            argv,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired:
        print(f"smoke-release-binary: timed out: {argv[0]}", file=sys.stderr)
        raise SystemExit(1)
    except OSError as exc:
        print(f"smoke-release-binary: failed to exec: {exc}", file=sys.stderr)
        raise SystemExit(1)
    if len(completed.stdout) > MAX_OUTPUT or len(completed.stderr) > MAX_OUTPUT:
        print("smoke-release-binary: subprocess output exceeded bound", file=sys.stderr)
        raise SystemExit(1)
    return completed


def expected_version():
    text = (ROOT / "Cargo.toml").read_text()
    match = re.search(r'(?m)^version\s*=\s*"(\d+\.\d+\.\d+)"', text)
    if not match:
        print("smoke-release-binary: cannot determine workspace version", file=sys.stderr)
        raise SystemExit(1)
    return match.group(1)


def glibc_ceiling(path):
    with open(path, "rb") as handle:
        magic = handle.read(4)
    if magic != b"\x7fELF":
        return None
    readelf = shutil.which("readelf")
    if readelf is None:
        print("smoke-release-binary: readelf is missing; cannot prove GLIBC floor", file=sys.stderr)
        raise SystemExit(1)
    completed = run_bounded([readelf, "-V", str(path)], timeout=60)
    if completed.returncode != 0:
        print("smoke-release-binary: readelf failed", file=sys.stderr)
        raise SystemExit(1)
    text = completed.stdout.decode("utf-8", errors="replace")
    versions = re.findall(r"GLIBC_(\d+)\.(\d+)", text)
    if not versions:
        print("smoke-release-binary: no GLIBC requirements found", file=sys.stderr)
        raise SystemExit(1)
    return max((int(major), int(minor)) for major, minor in versions)


def main():
    if len(sys.argv) != 2:
        print(f"usage: {sys.argv[0]} PATH", file=sys.stderr)
        return 2
    candidate = pathlib.Path(sys.argv[1])
    if not candidate.is_file():
        return fail(f"candidate does not exist: {candidate}")
    if not FIXTURE.is_file():
        return fail(f"fixture is missing: {FIXTURE}")

    version = expected_version()
    expected_identity = f"stegoeggo {version}"

    completed = run_bounded([str(candidate), "version"], timeout=30)
    if completed.returncode != 0:
        return fail("candidate version exited nonzero")
    first_line = completed.stdout.decode("utf-8", errors="replace").splitlines()
    first_line = first_line[0].strip() if first_line else ""
    if first_line != expected_identity:
        return fail(f"version identity mismatch: got {first_line!r}, want {expected_identity!r}")

    completed = run_bounded([str(candidate), "--help"], timeout=30)
    if completed.returncode != 0:
        return fail("candidate --help exited nonzero")

    ceiling = glibc_ceiling(candidate)
    if ceiling is not None and ceiling > (2, 17):
        return fail(f"GLIBC ceiling exceeded: GLIBC_{ceiling[0]}.{ceiling[1]} > GLIBC_2.17")

    tmpdir = tempfile.mkdtemp(prefix="stegoeggo-release-smoke-")
    try:
        protected = pathlib.Path(tmpdir) / "protected.png"
        completed = run_bounded(
            [
                str(candidate),
                "protect",
                str(FIXTURE),
                "--output",
                str(protected),
                "--rights-policy",
                "prohibited-ai-ml-training",
                "--preset",
                "legal-notice",
            ],
            timeout=90,
        )
        if completed.returncode != 0:
            return fail("candidate protect exited nonzero")
        if not protected.is_file():
            return fail("protect did not produce the expected output file")

        completed = run_bounded([str(candidate), "inspect", str(protected)], timeout=60)
        if completed.returncode != 0:
            return fail("candidate inspect exited nonzero")
        inspect_text = completed.stdout.decode("utf-8", errors="replace")
        if "ProhibitedAiMlTraining" not in inspect_text:
            return fail("inspect output is missing the expected rights policy")

        completed = run_bounded([str(candidate), "verify", str(protected)], timeout=60)
        if completed.returncode != 0:
            return fail("candidate verify exited nonzero")
    finally:
        shutil.rmtree(tmpdir, ignore_errors=True)

    print(f"release smoke passed: {expected_identity}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
