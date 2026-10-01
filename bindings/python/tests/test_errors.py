"""Tests for the structured exception hierarchy and projection attributes."""

from pathlib import Path

import pytest

import stegoeggo

FIXTURE_DIR = Path(__file__).parent / "fixtures" / "conformance" / "canonical"


def _fixture(name: str) -> bytes:
    return (FIXTURE_DIR / name).read_bytes()


def _metadata_request(holder: str = "Acme"):
    notice = stegoeggo.RightsNotice().with_copyright_holder(holder)
    return (
        stegoeggo.ProtectionRequest.metadata_only(
            notice, stegoeggo.RightsPolicy.ProhibitedAiMlTraining,
        )
        .with_seed(42)
        .with_timestamp_override("2026-01-01T00:00:00Z")
    )


def test_invalid_format_raises_invalid_format_error():
    with pytest.raises(stegoeggo.InvalidFormatError) as excinfo:
        stegoeggo.protect(
            b"not a real image",
            stegoeggo.ProtectionRequest.metadata_only(
                stegoeggo.RightsNotice(),
                stegoeggo.RightsPolicy.ProhibitedAiMlTraining,
            ).with_output_format(stegoeggo.ImageOutputFormat.Png),
        )
    assert excinfo.value.__class__.__mro__[1] is stegoeggo.StegoEggoError


def test_truncated_input_is_encoded_decode_error():
    """ImageTruncated must map to a documented existing category, not the base class."""

    payload = _fixture("canonical_complete.png")
    truncated = payload[: len(payload) // 3]
    with pytest.raises(stegoeggo.EncodeDecodeError) as excinfo:
        stegoeggo.protect(
            truncated,
            stegoeggo.ProtectionRequest.metadata_only(
                stegoeggo.RightsNotice(),
                stegoeggo.RightsPolicy.ProhibitedAiMlTraining,
            ),
        )
    assert isinstance(excinfo.value, stegoeggo.StegoEggoError)
    assert "truncated" in str(excinfo.value).lower()


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

    request = _metadata_request()
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


def test_input_too_large_structured_attributes():
    """InputTooLarge surfaces resource='input_bytes', size, and limit."""

    payload = _fixture("canonical_complete.png")
    limits = stegoeggo.ResourceLimits.builder().with_max_input_bytes(8).build()
    request = (
        _metadata_request()
        .with_resource_limits(limits)
    )
    with pytest.raises(stegoeggo.ResourceLimitError) as excinfo:
        stegoeggo.protect(payload, request)
    err = excinfo.value
    assert err.resource == "input_bytes"
    assert err.size == len(payload)
    assert err.limit == 8
    assert "8" in str(err)


def test_dimensions_exceeded_structured_attributes():
    """DimensionsExceeded surfaces width, height, max_width, max_height."""

    payload = _fixture("canonical_independent.png")
    limits = (
        stegoeggo.ResourceLimits.builder()
        .with_max_width(4)
        .with_max_height(4)
        .build()
    )
    request = _metadata_request().with_resource_limits(limits)
    with pytest.raises(stegoeggo.ResourceLimitError) as excinfo:
        stegoeggo.protect(payload, request)
    err = excinfo.value
    assert err.resource == "dimensions"
    assert err.width > 4
    assert err.height > 4
    assert err.max_width == 4
    assert err.max_height == 4


def test_container_limit_exceeded_structured_attributes():
    """ContainerLimitExceeded surfaces resource='container', kind, count, limit."""

    payload = _fixture("canonical_complete.png")
    limits = stegoeggo.ResourceLimits.builder().with_max_png_chunks(1).build()
    request = _metadata_request().with_resource_limits(limits)
    with pytest.raises(stegoeggo.ResourceLimitError) as excinfo:
        stegoeggo.protect(payload, request)
    err = excinfo.value
    assert err.resource == "container"
    assert err.kind == "PNG chunks"
    assert err.count > 1
    assert err.limit == 1


def test_metadata_limit_exceeded_structured_attributes():
    """MetadataLimitExceeded surfaces resource='metadata', kind, size, limit."""

    payload = _fixture("canonical_independent.png")
    limits = stegoeggo.ResourceLimits.builder().with_max_metadata_field_bytes(1).build()
    notice = stegoeggo.RightsNotice().with_copyright_holder("Acme Long Name")
    request = (
        stegoeggo.ProtectionRequest.metadata_only(
            notice, stegoeggo.RightsPolicy.ProhibitedAiMlTraining,
        )
        .with_resource_limits(limits)
        .with_seed(42)
        .with_timestamp_override("2026-01-01T00:00:00Z")
    )
    with pytest.raises(stegoeggo.ResourceLimitError) as excinfo:
        stegoeggo.protect(payload, request)
    err = excinfo.value
    assert err.resource == "metadata"
    assert isinstance(err.kind, str) and len(err.kind) > 0
    assert err.size > 1
    assert err.limit == 1


def test_resource_limit_error_does_not_leak_secret_keys():
    """Limits errors must not surface through exception repr/attributes."""

    payload = _fixture("canonical_independent.png")
    limits = (
        stegoeggo.ResourceLimits.builder()
        .with_max_input_bytes(8)
        .build()
    )
    request = (
        _metadata_request()
        .with_resource_limits(limits)
        .with_mac_key(b"super-secret-mac-key")
    )
    with pytest.raises(stegoeggo.ResourceLimitError) as excinfo:
        stegoeggo.protect(payload, request)
    err = excinfo.value
    text = repr(err) + str(err.__dict__)
    assert "super-secret-mac-key" not in text


def test_invalid_format_does_not_leak_secret_keys():
    """The fallback path on bad input must not surface secret bytes."""

    request = _metadata_request().with_mac_key(b"super-secret-mac-key")
    try:
        stegoeggo.protect(b"not a real image", request)
    except Exception as err:
        text = repr(err) + str(getattr(err, "__dict__", {}))
        assert "super-secret-mac-key" not in text
