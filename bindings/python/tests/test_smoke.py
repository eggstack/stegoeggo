"""Smoke tests for stegoeggo Python binding import and module surface."""

import stegoeggo


def test_import():
    assert stegoeggo.__version__ == "0.4.2"
    assert stegoeggo.__stegoeggo_version__ == "0.4.2"


def test_exceptions_are_subclasses_of_stegoeggo_error():
    assert issubclass(stegoeggo.InvalidConfigError, stegoeggo.StegoEggoError)
    assert issubclass(stegoeggo.InvalidFormatError, stegoeggo.StegoEggoError)
    assert issubclass(stegoeggo.EncodeDecodeError, stegoeggo.StegoEggoError)
    assert issubclass(stegoeggo.MetadataError, stegoeggo.StegoEggoError)
    assert issubclass(stegoeggo.SteganographyError, stegoeggo.StegoEggoError)
    assert issubclass(stegoeggo.InsufficientCapacityError, stegoeggo.StegoEggoError)
    assert issubclass(stegoeggo.VerificationError, stegoeggo.StegoEggoError)
    assert issubclass(stegoeggo.ResourceLimitError, stegoeggo.StegoEggoError)


def test_rights_policy_constants_present():
    for name in (
        "Unspecified",
        "Allowed",
        "ProhibitedAiMlTraining",
        "ProhibitedGenerativeAiTraining",
        "ProhibitedExceptSearchIndexing",
        "ProhibitedAllDataMining",
        "ProhibitedSeeConstraints",
    ):
        assert hasattr(stegoeggo.RightsPolicy, name), name


def test_image_output_format_constants():
    for name in ("Png", "Jpeg", "WebP"):
        assert hasattr(stegoeggo.ImageOutputFormat, name), name


def test_detection_for_known_magic_bytes():
    assert stegoeggo.detect_format(b"\x89PNG\r\n\x1a\n") == "png"
    assert stegoeggo.detect_format(b"\xff\xd8\xff\xe0") == "jpg"
    assert stegoeggo.detect_format(b"RIFF\x00\x00\x00\x00WEBP") == "webp"
    assert stegoeggo.detect_format(b"not an image") is None
    assert stegoeggo.detect_format(b"") is None


def test_image_output_format_from_extension():
    assert stegoeggo.ImageOutputFormat.from_extension("png") == stegoeggo.ImageOutputFormat.Png
    assert stegoeggo.ImageOutputFormat.from_extension("jpg") == stegoeggo.ImageOutputFormat.Jpeg
    assert stegoeggo.ImageOutputFormat.from_extension("JPEG") == stegoeggo.ImageOutputFormat.Jpeg
    assert stegoeggo.ImageOutputFormat.from_extension("webp") == stegoeggo.ImageOutputFormat.WebP
    assert stegoeggo.ImageOutputFormat.from_extension("gif") is None
