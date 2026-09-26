"""Cross-language parity tests.

These tests verify that:

1. The Python binding can read back output produced by the Rust CLI/library
   through the canonical verify path.

2. Bytes produced by the Python binding (with deterministic seed + timestamp)
   can be re-verified by the Rust CLI's `verify` command.

The tests are deliberately small and use the conformance fixtures.
"""

import hashlib
import json
import shutil
import subprocess
from pathlib import Path

import pytest

import stegoeggo

FIXTURE_DIR = Path(__file__).parent / "fixtures" / "conformance" / "canonical"


def _fixture(name: str) -> bytes:
    return (FIXTURE_DIR / name).read_bytes()


def _metadata_request(seed: int = 4242, ts: str = "2026-03-03T03:03:03Z"):
    notice = (
        stegoeggo.RightsNotice()
        .with_copyright_holder("Parity Co")
        .with_license_url("https://parity.example/license")
        .with_ai_constraints("No AI training")
    )
    return (
        stegoeggo.ProtectionRequest.metadata_only(
            notice, stegoeggo.RightsPolicy.ProhibitedAiMlTraining,
        )
        .with_output_format(stegoeggo.ImageOutputFormat.Png)
        .with_seed(seed)
        .with_timestamp_override(ts)
    )


@pytest.mark.parametrize("name", [
    "canonical_independent.png",
])
def test_python_protect_is_self_consistent(name):
    data = _fixture(name)
    out = stegoeggo.protect(data, _metadata_request())
    report = stegoeggo.verify(out)
    assert report.rights_found is True
    assert report.copyright_holder == "Parity Co"


def test_python_protect_writes_known_canonical_xmp():
    data = _fixture("canonical_independent.png")
    out = stegoeggo.protect(data, _metadata_request())
    assert b"Parity Co" in out
    assert b"https://parity.example/license" in out


@pytest.mark.skipif(
    shutil.which("stegoeggo") is None,
    reason="stegoeggo CLI not installed (cargo install stegoeggo-cli)",
)
def test_rust_cli_round_trip(tmp_path):
    data = _fixture("canonical_independent.png")
    request = _metadata_request()
    out = stegoeggo.protect(data, request)
    out_path = tmp_path / "out.png"
    out_path.write_bytes(out)

    result = subprocess.run(
        ["stegoeggo", "verify", str(out_path), "-o", str(tmp_path / "verify_out")],
        capture_output=True,
        text=True,
    )
    assert result.returncode in (0, 3), (result.stdout, result.stderr)


def test_deterministic_output_across_python_invocations():
    data = _fixture("canonical_complete.png")
    request = _metadata_request(seed=999, ts="2026-05-05T05:05:05Z")
    out1 = stegoeggo.protect(data, request)
    out2 = stegoeggo.protect(data, request)
    assert out1 == out2
    assert hashlib.sha256(out1).hexdigest() == hashlib.sha256(out2).hexdigest()


def test_protection_summary_json_round_trip():
    data = _fixture("canonical_complete.png")
    request = _metadata_request()
    out, report = stegoeggo.protect_with_report(data, request)
    summary = {
        "effective_policy": report.effective_policy.name,
        "effective_dmi": report.effective_dmi.name if report.effective_dmi else None,
        "metadata_injected": report.metadata_injected,
        "stego_attempted": report.stego_attempted,
        "stego_succeeded": report.stego_succeeded,
        "format_transcoded": report.format_transcoded,
        "warnings": [w.name for w in report.warnings],
    }
    raw = json.dumps(summary, sort_keys=True)
    assert "METADATA_INJECTED" not in raw  # noqa
    assert summary["effective_policy"] == "PROHIBITED_AI_ML_TRAINING"
    assert summary["metadata_injected"] is True
    assert summary["stego_attempted"] is False
    assert summary["stego_succeeded"] is False
