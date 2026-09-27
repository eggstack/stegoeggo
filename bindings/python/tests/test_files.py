"""Tests for the failure-safe file helpers and output-path option."""

from pathlib import Path

import pytest

import stegoeggo
from stegoeggo import _files

FIXTURE_DIR = Path(__file__).parent / "fixtures" / "conformance" / "canonical"


def _fixture(name: str) -> bytes:
    return (FIXTURE_DIR / name).read_bytes()


def _metadata_request(holder: str = "Acme Corp"):
    notice = (
        stegoeggo.RightsNotice()
        .with_copyright_holder(holder)
        .with_license_url("https://example.com/license")
    )
    return (
        stegoeggo.ProtectionRequest.metadata_only(
            notice, stegoeggo.RightsPolicy.ProhibitedAiMlTraining,
        )
        .with_output_format(stegoeggo.ImageOutputFormat.Png)
        .with_seed(42)
        .with_timestamp_override("2026-01-01T00:00:00Z")
    )


def test_protect_file_in_place(tmp_path):
    src = tmp_path / "in.png"
    original = _fixture("canonical_independent.png")
    src.write_bytes(original)

    stegoeggo.protect_file(str(src), _metadata_request())

    rewritten = src.read_bytes()
    assert rewritten != original
    report = stegoeggo.verify_file(str(src))
    assert report.rights_found is True
    assert report.copyright_holder == "Acme Corp"


def test_protect_file_explicit_output_leaves_source_unchanged(tmp_path):
    src = tmp_path / "in.png"
    out = tmp_path / "out.png"
    original = _fixture("canonical_independent.png")
    src.write_bytes(original)
    src_bytes_before = original

    stegoeggo.protect_file(str(src), _metadata_request(), str(out))

    assert src.read_bytes() == src_bytes_before
    assert out.exists()
    rewritten = out.read_bytes()
    assert rewritten != src_bytes_before
    report = stegoeggo.verify_file(str(out))
    assert report.rights_found is True
    assert report.copyright_holder == "Acme Corp"


def test_protect_file_processing_failure_leaves_source_unchanged(tmp_path):
    src = tmp_path / "invalid.bin"
    src.write_bytes(b"definitely not an image")

    with pytest.raises(stegoeggo.StegoEggoError):
        stegoeggo.protect_file(str(src), _metadata_request())
    assert src.read_bytes() == b"definitely not an image"


def test_protect_file_forced_write_failure_leaves_source_unchanged(tmp_path, monkeypatch):
    src = tmp_path / "in.png"
    original = _fixture("canonical_independent.png")
    src.write_bytes(original)
    original_bytes = src.read_bytes()

    def bad_writer(target, data):
        raise OSError("simulated write failure")

    monkeypatch.setattr(_files, "_atomic_write_bytes", bad_writer)
    with pytest.raises(OSError, match="simulated write failure"):
        stegoeggo.protect_file(str(src), _metadata_request())

    assert src.read_bytes() == original_bytes


def test_protect_file_forced_failure_cleans_temp_files(tmp_path, monkeypatch):
    src = tmp_path / "in.png"
    src.write_bytes(_fixture("canonical_independent.png"))

    def bad_writer(target, data):
        raise OSError("simulated write failure")

    monkeypatch.setattr(_files, "_atomic_write_bytes", bad_writer)
    with pytest.raises(OSError):
        stegoeggo.protect_file(str(src), _metadata_request())

    leftovers = [p for p in tmp_path.iterdir() if p.name.startswith(".stegoeggo-")]
    assert leftovers == [], f"unexpected temp files left behind: {leftovers}"


def test_verify_file_equivalent_to_byte_verify(tmp_path):
    src = tmp_path / "in.png"
    original = _fixture("canonical_independent.png")
    src.write_bytes(original)
    stegoeggo.protect_file(str(src), _metadata_request())

    file_report = stegoeggo.verify_file(str(src))
    bytes_report = stegoeggo.verify(src.read_bytes())
    assert file_report.rights_found is True
    assert bytes_report.rights_found is True
    assert file_report.copyright_holder == bytes_report.copyright_holder


def test_atomic_write_replaces_existing_file(tmp_path):
    target = tmp_path / "out.bin"
    target.write_bytes(b"original-contents")
    _files._atomic_write_bytes(target, b"new-contents")
    assert target.read_bytes() == b"new-contents"


def test_atomic_write_leaves_no_temp_files(tmp_path):
    target = tmp_path / "out.bin"
    _files._atomic_write_bytes(target, b"hello world")
    leftovers = [p for p in tmp_path.iterdir() if p.name.startswith(".stegoeggo-")]
    assert leftovers == []


def test_atomic_write_failure_cleans_temp(tmp_path, monkeypatch):
    target = tmp_path / "out.bin"

    def broken_replace(src, dst):
        raise OSError("replace failed")

    monkeypatch.setattr(_files.os, "replace", broken_replace)
    with pytest.raises(OSError, match="replace failed"):
        _files._atomic_write_bytes(target, b"hello world")
    leftovers = [p for p in tmp_path.iterdir() if p.name.startswith(".stegoeggo-")]
    assert leftovers == [], f"unexpected temp files left behind: {leftovers}"
    assert not target.exists()
