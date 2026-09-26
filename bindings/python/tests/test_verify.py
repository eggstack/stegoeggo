"""Tests for the verify operation."""

from pathlib import Path

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
    mac_key: bytes = b"shared-verification-key",
):
    notice = (
        stegoeggo.RightsNotice()
        .with_copyright_holder("Acme Corp")
        .with_license_url("https://example.com/license")
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


def test_verify_unprotected_image_has_no_rights():
    data = _fixture("canonical_independent.png")
    report = stegoeggo.verify(data)
    assert isinstance(report, stegoeggo.VerificationReport)
    assert report.rights_found is False
    assert report.status == stegoeggo.VerificationStatus.NotFound
    assert report.evidence_strength == stegoeggo.EvidenceStrength.NoNoticeFound


def test_verify_metadata_only_protected_image():
    data = _fixture("canonical_independent.png")
    protected = stegoeggo.protect(data, _metadata_request())
    report = stegoeggo.verify(protected)
    assert report.rights_found is True
    assert report.copyright_holder == "Acme Corp"
    assert report.license_url == "https://example.com/license"
    assert report.evidence_strength == stegoeggo.EvidenceStrength.MetadataNoticeOnly


def test_verify_stego_with_correct_mac_key():
    data = _fixture("canonical_complete.png")
    request = _stego_request()
    protected = stegoeggo.protect(data, request)
    report = stegoeggo.verify(protected, mac_key=b"shared-verification-key")
    assert report.rights_found is True
    assert report.authentication_attempted is True
    assert report.authentication_key_matched is True
    assert report.hidden_marker_status == stegoeggo.VerificationStatus.Verified
    assert report.hidden_marker_seed == 42
    assert report.hidden_marker_payload_version == 3
    assert report.hidden_marker_tiled is False
    assert (
        report.evidence_strength
        == stegoeggo.EvidenceStrength.MetadataNoticeAndAuthenticatedProvenance
    )


def test_verify_stego_with_wrong_mac_key():
    data = _fixture("canonical_complete.png")
    protected = stegoeggo.protect(data, _stego_request())
    report = stegoeggo.verify(protected, mac_key=b"wrong-key")
    assert report.authentication_key_matched is False
    assert report.hidden_marker_status == stegoeggo.VerificationStatus.Invalid


def test_verify_stego_with_missing_mac_key():
    data = _fixture("canonical_complete.png")
    protected = stegoeggo.protect(data, _stego_request())
    report = stegoeggo.verify(protected, mac_key=None)
    assert report.hidden_marker_status == stegoeggo.VerificationStatus.Invalid


def test_verify_does_not_leak_mac_key():
    data = _fixture("canonical_complete.png")
    request = _stego_request(mac_key=b"top-secret-key")
    protected = stegoeggo.protect(data, request)
    report = stegoeggo.verify(protected)
    assert "top-secret-key" not in repr(report)
    assert "top-secret-key" not in str(report.to_json())


def test_verify_malformed_input():
    report = stegoeggo.verify(b"not a real image")
    assert report.rights_found is False
    assert report.hidden_marker_status == stegoeggo.VerificationStatus.NotFound


def test_verify_truncated_input():
    data = _fixture("canonical_complete.png")
    truncated = data[:128]
    report = stegoeggo.verify(truncated)
    assert isinstance(report, stegoeggo.VerificationReport)


def test_verify_empty_input():
    report = stegoeggo.verify(b"")
    assert report.rights_found is False


def test_verify_report_to_dict_to_json():
    data = _fixture("canonical_complete.png")
    request = _stego_request()
    protected = stegoeggo.protect(data, request)
    report = stegoeggo.verify(protected, mac_key=b"shared-verification-key")
    d = report.to_dict()
    assert isinstance(d, dict)
    assert d.get("rights", {}).get("found") is True
    raw = report.to_json()
    assert "rights" in raw
    assert "hidden_marker" in raw


def test_verify_with_resource_limits():
    data = _fixture("canonical_complete.png")
    request = _stego_request()
    protected = stegoeggo.protect(data, request)
    limits = stegoeggo.ResourceLimits.defaults()
    report = stegoeggo.verify(protected, mac_key=b"shared-verification-key", resource_limits=limits)
    assert isinstance(report, stegoeggo.VerificationReport)
