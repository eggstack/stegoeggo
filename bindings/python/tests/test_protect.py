"""Protection-operation tests using the canonical conformance fixtures."""

from pathlib import Path

import pytest

import stegoeggo

FIXTURE_DIR = Path(__file__).parent / "fixtures" / "conformance" / "canonical"


def _fixture(name: str) -> bytes:
    path = FIXTURE_DIR / name
    return path.read_bytes()


def _metadata_request(seed: int = 42, ts: str = "2026-01-01T00:00:00Z"):
    notice = (
        stegoeggo.RightsNotice()
        .with_copyright_holder("Acme Corp")
        .with_license_url("https://example.com/license")
        .with_ai_constraints("No AI training permitted")
    )
    return (
        stegoeggo.ProtectionRequest.metadata_only(
            notice, stegoeggo.RightsPolicy.ProhibitedAiMlTraining,
        )
        .with_output_format(stegoeggo.ImageOutputFormat.Png)
        .with_seed(seed)
        .with_timestamp_override(ts)
    )


def _stego_request(
    seed: int = 42,
    ts: str = "2026-01-01T00:00:00Z",
    mac_key: bytes = b"deterministic-key-for-parity",
):
    notice = (
        stegoeggo.RightsNotice()
        .with_copyright_holder("Acme Corp")
        .with_license_url("https://example.com/license")
        .with_ai_constraints("No AI training permitted")
    )
    return (
        stegoeggo.ProtectionRequest.with_hidden_marker(
            notice, stegoeggo.RightsPolicy.ProhibitedAiMlTraining,
        )
        .with_output_format(stegoeggo.ImageOutputFormat.Png)
        .with_seed(seed)
        .with_timestamp_override(ts)
        .with_mac_key(mac_key)
    )


@pytest.mark.parametrize("name", [
    "canonical_complete.png",
    "canonical_copyright_only.png",
    "canonical_independent.png",
    "canonical_policy_only.png",
    "canonical_unicode.png",
])
def test_metadata_only_protect_png(name):
    data = _fixture(name)
    out = stegoeggo.protect(data, _metadata_request())
    assert out[:8] == b"\x89PNG\r\n\x1a\n"


@pytest.mark.parametrize("name", [
    "canonical_complete.jpg",
    "canonical_independent.jpg",
])
def test_metadata_only_protect_jpeg(name):
    data = _fixture(name)
    out = stegoeggo.protect(data, _metadata_request())
    assert out[:8] == b"\x89PNG\r\n\x1a\n"


def test_metadata_only_protect_webp():
    data = _fixture("canonical_complete.webp")
    out = stegoeggo.protect(data, _metadata_request())
    assert out[:8] == b"\x89PNG\r\n\x1a\n"


@pytest.mark.parametrize("name", [
    "canonical_complete.png",
    "canonical_copyright_only.png",
])
def test_hidden_marker_protect_png(name):
    data = _fixture(name)
    out = stegoeggo.protect(data, _stego_request())
    assert out[:8] == b"\x89PNG\r\n\x1a\n"
    assert out != data


def test_protect_returns_bytes():
    data = _fixture("canonical_independent.png")
    out = stegoeggo.protect(data, _metadata_request())
    assert isinstance(out, bytes)


def test_deterministic_seed_and_timestamp():
    data = _fixture("canonical_independent.png")
    r1 = _metadata_request(seed=123, ts="2026-02-02T02:02:02Z")
    r2 = _metadata_request(seed=123, ts="2026-02-02T02:02:02Z")
    out1 = stegoeggo.protect(data, r1)
    out2 = stegoeggo.protect(data, r2)
    assert out1 == out2


def test_different_seeds_yield_different_output_for_stego():
    data = _fixture("canonical_complete.png")
    r1 = _stego_request(seed=1)
    r2 = _stego_request(seed=2)
    out1 = stegoeggo.protect(data, r1)
    out2 = stegoeggo.protect(data, r2)
    assert out1 != out2


def test_protect_with_warnings():
    data = _fixture("canonical_complete.png")
    out, warnings = stegoeggo.protect_with_warnings(data, _metadata_request())
    assert out[:8] == b"\x89PNG\r\n\x1a\n"
    assert isinstance(warnings, list)


def test_protect_with_report_metadata_injected():
    data = _fixture("canonical_independent.png")
    out, report = stegoeggo.protect_with_report(data, _metadata_request())
    assert out[:8] == b"\x89PNG\r\n\x1a\n"
    assert report.metadata_injected is True
    assert report.stego_attempted is False
    assert report.stego_succeeded is False
    assert report.effective_policy == stegoeggo.RightsPolicy.ProhibitedAiMlTraining
    assert report.effective_dmi == stegoeggo.DmiValue.ProhibitedAiMlTraining
    assert report.format_transcoded is False
    assert isinstance(report.warnings, list)
    assert report.resource_usage is not None
    assert report.resource_usage.input_bytes == len(data)


def test_protect_with_report_stego_succeeded():
    data = _fixture("canonical_complete.png")
    out, report = stegoeggo.protect_with_report(data, _stego_request())
    assert report.stego_attempted is True
    assert report.stego_succeeded is True


def test_protect_with_report_does_not_leak_mac_key():
    data = _fixture("canonical_complete.png")
    out, report = stegoeggo.protect_with_report(data, _stego_request(mac_key=b"top-secret-key"))
    assert "top-secret-key" not in repr(report)


def test_protect_file_and_verify_file(tmp_path):
    src = FIXTURE_DIR / "canonical_independent.png"
    dst = tmp_path / "out.png"
    dst.write_bytes(src.read_bytes())
    request = _metadata_request()
    stegoeggo.protect_file(str(dst), request)
    protected = dst.read_bytes()
    assert protected != src.read_bytes()
    report = stegoeggo.verify_file(str(dst))
    assert report.rights_found is True
