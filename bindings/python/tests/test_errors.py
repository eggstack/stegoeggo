"""Tests for the structured exception hierarchy."""

from pathlib import Path

import pytest

import stegoeggo

FIXTURE_DIR = Path(__file__).parent / "fixtures" / "conformance" / "canonical"


def _fixture(name: str) -> bytes:
    return (FIXTURE_DIR / name).read_bytes()


def test_invalid_format_raises_invalid_format_error():
    with pytest.raises(stegoeggo.InvalidFormatError) as excinfo:
        stegoeggo.protect(b"not a real image", stegoeggo.ProtectionRequest.metadata_only(
            stegoeggo.RightsNotice(),
            stegoeggo.RightsPolicy.ProhibitedAiMlTraining,
        ).with_output_format(stegoeggo.ImageOutputFormat.Png))
    assert excinfo.value.__class__.__mro__[1] is stegoeggo.StegoEggoError


def test_truncated_input_raises_some_stegoeggo_error():
    with pytest.raises(stegoeggo.StegoEggoError):
        stegoeggo.protect(b"\x89PNG\r\n\x1a\n", stegoeggo.ProtectionRequest.metadata_only(
            stegoeggo.RightsNotice(),
            stegoeggo.RightsPolicy.ProhibitedAiMlTraining,
        ))


def test_invalid_format_bytes_raises():
    with pytest.raises(stegoeggo.InvalidFormatError):
        stegoeggo.protect(
            b"not a real image",
            stegoeggo.ProtectionRequest.metadata_only(
                stegoeggo.RightsNotice(),
                stegoeggo.RightsPolicy.ProhibitedAiMlTraining,
            ),
        )


def test_contradictory_legal_claims_warns():
    """A request with legal metadata but legal claims disabled should produce a warning."""

    notice = stegoeggo.RightsNotice().with_copyright_holder("Acme")
    request = (
        stegoeggo.ProtectionRequest.metadata_only(
            notice, stegoeggo.RightsPolicy.ProhibitedAiMlTraining,
        )
        .with_seed(42)
        .with_timestamp_override("2026-01-01T00:00:00Z")
    )
    out, warnings = stegoeggo.protect_with_warnings(
        _fixture("canonical_complete.png"), request,
    )
    assert isinstance(warnings, list)


def test_missing_rights_constraints_emits_warning():
    notice = stegoeggo.RightsNotice()
    request = (
        stegoeggo.ProtectionRequest.metadata_only(
            notice, stegoeggo.RightsPolicy.ProhibitedSeeConstraints,
        )
        .with_seed(42)
        .with_timestamp_override("2026-01-01T00:00:00Z")
    )
    out, warnings = stegoeggo.protect_with_warnings(
        _fixture("canonical_complete.png"), request,
    )
    warning_names = [w.name for w in warnings]
    assert "MISSING_RIGHTS_CONSTRAINTS" in warning_names


def test_insufficient_capacity_structured_attribute():
    """Force insufficient capacity by raising max_input_bytes below the test fixture size."""

    tiny = (
        stegoeggo.ResourceLimits.builder()
        .with_max_input_bytes(8)
        .build()
    )
    notice = stegoeggo.RightsNotice().with_copyright_holder("Acme")
    request = (
        stegoeggo.ProtectionRequest.metadata_only(
            notice, stegoeggo.RightsPolicy.ProhibitedAiMlTraining,
        )
        .with_resource_limits(tiny)
        .with_seed(42)
        .with_timestamp_override("2026-01-01T00:00:00Z")
    )
    with pytest.raises(stegoeggo.ResourceLimitError):
        stegoeggo.protect(_fixture("canonical_complete.png"), request)
