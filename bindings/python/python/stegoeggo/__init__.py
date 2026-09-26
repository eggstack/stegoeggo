"""StegoEggo — Python bindings for rights-reservation and steganography."""

from ._native import (  # noqa: F401
    AuthenticationMode,
    DmiValue,
    EncodeDecodeError,
    EvidenceStrength,
    ExecutionReport,
    HiddenMarkerMode,
    ImageOutputFormat,
    InsufficientCapacityError,
    InvalidConfigError,
    InvalidFormatError,
    MetadataError,
    MetadataUpdatePolicy,
    ProcessingOptions,
    ProtectionPreset,
    ProtectionRequest,
    ProtectionWarning,
    ResourceLimits,
    ResourceUsage,
    ResourceLimitError,
    RightsNotice,
    RightsPolicy,
    SteganographyError,
    StegoEggoError,
    VerificationError,
    VerificationReport,
    VerificationStatus,
    detect_format,
    protect,
    protect_file,
    protect_with_report,
    protect_with_warnings,
    verify,
    verify_file,
)
from ._native import __version__ as __binding_version__
from ._native import __stegoeggo_version__ as __stegoeggo_version__

__version__ = __binding_version__
