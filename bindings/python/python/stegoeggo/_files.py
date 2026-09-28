"""Failure-safe file helpers for the Python binding.

These helpers stay above the native extension so the Rust layer only deals in
bytes, requests, reports, and structured exceptions. The same-directory
temporary file + ``os.replace`` pattern guarantees that the destination is
either the new bytes or the original bytes — never a truncated or partially
written file.
"""

from __future__ import annotations

import os
import tempfile
from pathlib import Path
from typing import Optional

from . import _native

_TEMP_PREFIX = ".stegoeggo-"


def _atomic_write_bytes(target: Path, data: bytes) -> None:
    target_dir = target.parent
    target_dir.mkdir(parents=True, exist_ok=True)
    fd, tmp_name = tempfile.mkstemp(prefix=_TEMP_PREFIX, suffix=".tmp", dir=str(target_dir))
    tmp_path = Path(tmp_name)
    try:
        with os.fdopen(fd, "wb") as fp:
            fp.write(data)
            fp.flush()
            os.fsync(fp.fileno())
        os.replace(tmp_path, target)
    except BaseException:
        try:
            os.unlink(tmp_path)
        except OSError:
            pass
        raise


def protect_file(
    path: str,
    request: _native.ProtectionRequest,
    output_path: Optional[str] = None,
) -> None:
    """Protect an encoded image on disk.

    The bytes are processed in full before any write to the destination. The
    completed output is staged as a same-directory temporary file and only
    swapped into place with :func:`os.replace` after a successful ``fsync``,
    so the destination either contains the original bytes or the new bytes —
    never a truncated or partially replaced file. On any failure the original
    file is untouched and any leftover temporary file is best-effort cleaned
    up.

    ``output_path`` is optional: when supplied, the protected bytes are
    written to that path and the input ``path`` is never mutated. When
    omitted, the protected bytes replace the input file in place.
    """
    src = Path(path)
    dst = Path(output_path) if output_path is not None else src
    data = src.read_bytes()
    protected = _native.protect(data, request)
    _atomic_write_bytes(dst, protected)


def verify_file(
    path: str,
    mac_key: Optional[bytes] = None,
    resource_limits: Optional[_native.ResourceLimits] = None,
) -> _native.VerificationReport:
    """Read ``path`` and report against the canonical byte verify path."""
    data = Path(path).read_bytes()
    return _native.verify(data, mac_key=mac_key, resource_limits=resource_limits)
