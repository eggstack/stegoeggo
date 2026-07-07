#!/usr/bin/env bash
# verify_metadata_conformance.sh — External metadata conformance check
#
# Verifies that a protected image contains the expected metadata markers
# using external tools (exiftool, ImageMagick, vips).
#
# Usage: ./scripts/verify_metadata_conformance.sh <protected-image> [expected-copyright-holder]
#
# Requirements: exiftool (required), one of: magick (ImageMagick) or vips (optional)
#
# Exit codes:
#   0 — All checks passed
#   1 — Missing required tool
#   2 — One or more checks failed

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

pass() { echo -e "  ${GREEN}PASS${NC}: $1"; }
fail() { echo -e "  ${RED}FAIL${NC}: $1"; FAILURES=$((FAILURES + 1)); }
warn() { echo -e "  ${YELLOW}WARN${NC}: $1"; }

if [ $# -lt 1 ]; then
    echo "Usage: $0 <protected-image> [expected-copyright-holder]"
    exit 1
fi

IMAGE="$1"
EXPECTED_COPYRIGHT="${2:-}"
FAILURES=0

if [ ! -f "$IMAGE" ]; then
    echo "Error: File not found: $IMAGE"
    exit 1
fi

echo "=== Metadata Conformance Check: $IMAGE ==="
echo ""

# ── Check required tools ──────────────────────────────────────────────

if ! command -v exiftool &>/dev/null; then
    echo "Error: exiftool is required but not found in PATH."
    echo "Install with: brew install exiftool (macOS) or apt install libimage-exiftool-perl (Linux)"
    exit 1
fi

HAS_MAGICK=false
HAS_VIPS=false
if command -v magick &>/dev/null || command -v convert &>/dev/null; then
    HAS_MAGICK=true
fi
if command -v vips &>/dev/null; then
    HAS_VIPS=true
fi

# ── exiftool checks ───────────────────────────────────────────────────

echo "--- exiftool checks ---"

# Check for stegoeggo protection seed in metadata
SEED=$(exiftool -s3 -UserComment "$IMAGE" 2>/dev/null || true)
if echo "$SEED" | grep -q "Protection seed:"; then
    pass "UserComment contains protection seed"
else
    warn "UserComment does not contain protection seed (may be in other metadata)"
fi

# Check for DMI/copyright in XMP
XMP_DMI=$(exiftool -s3 -XMP:DataMining "$IMAGE" 2>/dev/null || true)
if [ -n "$XMP_DMI" ]; then
    pass "XMP contains DataMining attribute: $XMP_DMI"
else
    warn "XMP DataMining attribute not found via exiftool"
fi

# Check for IPTC DataMining
IPTC_DMI=$(exiftool -s3 -IPTC:ObjectName "$IMAGE" 2>/dev/null || true)
if [ -n "$IPTC_DMI" ]; then
    pass "IPTC ObjectName found: $IPTC_DMI"
else
    warn "IPTC ObjectName not found"
fi

# Check for copyright in any metadata
COPYRIGHT=$(exiftool -s3 -Copyright "$IMAGE" 2>/dev/null || true)
if [ -n "$COPYRIGHT" ]; then
    pass "Copyright found: $COPYRIGHT"
    if [ -n "$EXPECTED_COPYRIGHT" ]; then
        if echo "$COPYRIGHT" | grep -q "$EXPECTED_COPYRIGHT"; then
            pass "Copyright matches expected: $EXPECTED_COPYRIGHT"
        else
            fail "Copyright '$COPYRIGHT' does not contain expected '$EXPECTED_COPYRIGHT'"
        fi
    fi
else
    warn "Copyright not found in standard exiftool fields"
fi

# Check for XMP tdm:reserve_tdm
TDM=$(exiftool -s3 -XMP-dc:Description "$IMAGE" 2>/dev/null || true)
if echo "$TDM" | grep -q "tdm:reserve_tdm"; then
    pass "XMP contains TDM reservation"
else
    warn "TDM reservation not visible via exiftool XMP-dc:Description"
fi

# Check for custom stegoeggo namespace
STEGOEGGO=$(exiftool -a -G1 "$IMAGE" 2>/dev/null | grep -i "stegoeggo" || true)
if [ -n "$STEGOEGGO" ]; then
    pass "stegoeggo namespace found in metadata"
else
    warn "stegoeggo namespace not found via exiftool"
fi

# ── Format-specific checks ────────────────────────────────────────────

FORMAT=$(exiftool -s3 -FileType "$IMAGE" 2>/dev/null || echo "unknown")
echo ""
echo "--- Format-specific checks ($FORMAT) ---"

case "$FORMAT" in
    PNG)
        if $HAS_MAGICK; then
            CMD=$(command -v magick 2>/dev/null || command -v convert 2>/dev/null)
            TEXT_CHUNKS=$($CMD identify -verbose "$IMAGE" 2>/dev/null | grep -i "comment\|text" || true)
            if echo "$TEXT_CHUNKS" | grep -qi "copyright"; then
                pass "PNG tEXt chunk contains copyright"
            else
                warn "Could not verify PNG tEXt copyright via ImageMagick"
            fi
        else
            warn "ImageMagick not available, skipping PNG chunk verification"
        fi
        ;;
    JPEG)
        COM=$(exiftool -s3 -Comment "$IMAGE" 2>/dev/null || true)
        if [ -n "$COM" ]; then
            pass "JPEG COM marker found: $(echo "$COM" | head -c 80)..."
            if echo "$COM" | grep -q "Protection seed:"; then
                pass "JPEG COM contains protection seed"
            fi
        else
            warn "JPEG COM marker not found"
        fi

        # Check for IPTC
        IPTC_CHECK=$(exiftool -s3 -IPTC:By-line "$IMAGE" 2>/dev/null || true)
        if [ -n "$IPTC_CHECK" ]; then
            pass "JPEG IPTC data found"
        else
            warn "JPEG IPTC data not found"
        fi
        ;;
    WebP)
        if $HAS_VIPS; then
            WEBP_META=$(vips header "$IMAGE" 2>/dev/null | grep -i "xmp\|exif" || true)
            if [ -n "$WEBP_META" ]; then
                pass "WebP XMP/EXIF chunk found"
            else
                warn "WebP XMP/EXIF chunk not verified via vips"
            fi
        else
            warn "vips not available, skipping WebP chunk verification"
        fi
        ;;
    *)
        warn "Unknown format: $FORMAT"
        ;;
esac

# ── ImageMagick checks (if available) ─────────────────────────────────

if $HAS_MAGICK; then
    echo ""
    echo "--- ImageMagick checks ---"
    CMD=$(command -v magick 2>/dev/null || command -v convert 2>/dev/null)
    PROFILE=$($CMD identify -verbose "$IMAGE" 2>/dev/null | head -50 || true)
    if echo "$PROFILE" | grep -qi "comment"; then
        pass "ImageMagick reports image comment"
    else
        warn "No image comment visible via ImageMagick"
    fi
fi

# ── Summary ───────────────────────────────────────────────────────────

echo ""
echo "=== Summary ==="
if [ "$FAILURES" -eq 0 ]; then
    echo -e "${GREEN}All checks passed.${NC}"
    exit 0
else
    echo -e "${RED}$FAILURES check(s) failed.${NC}"
    exit 2
fi
