"""Tests for the canonical ProtectionRequest and RightsNotice DTOs."""

import pytest

import stegoeggo


def test_rights_notice_builder_chain():
    notice = (
        stegoeggo.RightsNotice()
        .with_copyright_holder("Example Corp")
        .with_contact_email("legal@example.com")
        .with_license_url("https://example.com/license")
        .with_usage_terms("All rights reserved")
        .with_creation_date("2026-01-01")
        .with_ai_constraints("No AI training permitted")
        .with_web_statement_of_rights("https://example.com/rights")
        .with_creator("Alice")
        .with_credit_line("Photo by Alice")
        .with_copyright_owner("Example Corp")
        .with_licensor_name("Bob")
        .with_licensor_email("bob@example.com")
        .with_licensor_url("https://bob.example.com")
        .with_metadata_date("2026-01-01T00:00:00Z")
        .with_notice_applied_at("2026-01-01T00:00:00Z")
        .with_dmi(stegoeggo.DmiValue.ProhibitedAiMlTraining)
    )
    assert notice.copyright_holder == "Example Corp"
    assert notice.contact_email == "legal@example.com"
    assert notice.license_url == "https://example.com/license"
    assert notice.usage_terms == "All rights reserved"
    assert notice.creation_date == "2026-01-01"
    assert notice.ai_constraints == "No AI training permitted"
    assert notice.web_statement_of_rights == "https://example.com/rights"
    assert notice.creator == "Alice"
    assert notice.credit_line == "Photo by Alice"
    assert notice.copyright_owner == "Example Corp"
    assert notice.licensor_name == "Bob"
    assert notice.licensor_email == "bob@example.com"
    assert notice.licensor_url == "https://bob.example.com"
    assert notice.metadata_date == "2026-01-01T00:00:00Z"
    assert notice.notice_applied_at == "2026-01-01T00:00:00Z"
    assert notice.dmi == stegoeggo.DmiValue.ProhibitedAiMlTraining
    assert notice.has_legal_content is True
    assert "copyright_holder" in repr(notice)


def test_empty_rights_notice():
    notice = stegoeggo.RightsNotice()
    assert notice.has_legal_content is False
    assert notice.copyright_holder is None
    assert notice.dmi is None


def test_protection_request_metadata_only():
    notice = stegoeggo.RightsNotice().with_copyright_holder("Acme")
    request = stegoeggo.ProtectionRequest.metadata_only(
        notice, stegoeggo.RightsPolicy.ProhibitedAiMlTraining,
    )
    assert request.policy == stegoeggo.RightsPolicy.ProhibitedAiMlTraining
    assert request.has_mac_key is False
    assert request.seed is None


def test_protection_request_with_hidden_marker():
    notice = stegoeggo.RightsNotice().with_copyright_holder("Acme")
    request = stegoeggo.ProtectionRequest.with_hidden_marker(
        notice, stegoeggo.RightsPolicy.Allowed,
    )
    assert request.policy == stegoeggo.RightsPolicy.Allowed


def test_protection_request_from_preset():
    notice = stegoeggo.RightsNotice().with_copyright_holder("Acme")
    for preset in (
        stegoeggo.ProtectionPreset.LegalNotice,
        stegoeggo.ProtectionPreset.LegalNoticeWithStego,
        stegoeggo.ProtectionPreset.AuthenticatedProvenance,
        stegoeggo.ProtectionPreset.Maximal,
    ):
        request = stegoeggo.ProtectionRequest.from_preset(
            preset, notice, stegoeggo.RightsPolicy.ProhibitedAiMlTraining,
        )
        assert request.policy == stegoeggo.RightsPolicy.ProhibitedAiMlTraining


def test_protection_request_builder_chain():
    notice = stegoeggo.RightsNotice().with_copyright_holder("Acme")
    request = (
        stegoeggo.ProtectionRequest.metadata_only(
            notice, stegoeggo.RightsPolicy.ProhibitedAiMlTraining,
        )
        .with_seed(42)
        .with_intensity(0.8)
        .with_output_format(stegoeggo.ImageOutputFormat.Png)
        .with_jpeg_quality(85)
        .with_progressive_jpeg()
        .with_max_dimension(4096)
        .with_metadata_update_policy(stegoeggo.MetadataUpdatePolicy.ReplaceStegoOwned)
        .with_stego_redundancy(2)
        .with_content_hash(b"\x01\x02\x03\x04")
        .with_timestamp_override("2026-01-01T00:00:00Z")
        .with_mac_key(b"secret-key")
    )
    assert request.seed == 42
    assert request.intensity == pytest.approx(0.8)
    assert request.has_mac_key is True


def test_protection_request_repr_does_not_leak_mac_key():
    notice = stegoeggo.RightsNotice().with_copyright_holder("Acme")
    request = stegoeggo.ProtectionRequest.metadata_only(
        notice, stegoeggo.RightsPolicy.ProhibitedAiMlTraining,
    ).with_mac_key(b"super-secret-key")
    assert "super-secret-key" not in repr(request)


def test_resource_limits_builder():
    limits = (
        stegoeggo.ResourceLimits.builder()
        .with_max_input_bytes(50 * 1024 * 1024)
        .with_max_width(8192)
        .with_max_height(8192)
        .with_max_png_chunks(256)
        .build()
    )
    assert "8192x8192" in repr(limits)


def test_hidden_marker_modes():
    assert stegoeggo.HiddenMarkerMode.disabled() == stegoeggo.HiddenMarkerMode.disabled()
    assert stegoeggo.HiddenMarkerMode.best_effort().is_tiled is False
    assert stegoeggo.HiddenMarkerMode.tiled(64).is_tiled is True
    assert stegoeggo.HiddenMarkerMode.tiled(64).tile_size == 64
    assert stegoeggo.HiddenMarkerMode.tiled(64) == stegoeggo.HiddenMarkerMode.tiled(64)
    assert stegoeggo.HiddenMarkerMode.tiled(64) != stegoeggo.HiddenMarkerMode.tiled(128)


def test_hidden_marker_tiled_out_of_range():
    with pytest.raises(ValueError):
        stegoeggo.HiddenMarkerMode.tiled(8)
    with pytest.raises(ValueError):
        stegoeggo.HiddenMarkerMode.tiled(2048)


def test_processing_options_builder():
    opts = (
        stegoeggo.ProcessingOptions()
        .with_output_format(stegoeggo.ImageOutputFormat.WebP)
        .with_jpeg_quality(80)
        .with_progressive_jpeg()
        .with_max_dimension(2048)
    )
    text = repr(opts)
    assert "webp" in text or "WebP" in text or "WEBP" in text
