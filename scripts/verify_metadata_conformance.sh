#!/usr/bin/env bash
# verify_metadata_conformance.sh — External metadata conformance check
#
# Verifies that a protected image contains the expected legal-notice metadata
# using external tools (exiftool, ImageMagick, vips).
#
# Usage:
#   ./scripts/verify_metadata_conformance.sh [OPTIONS] [protected-image] [expected-copyright-holder]
#
# Options:
#   --strict    Fail if exiftool is not installed (default: skip external checks)
#
# If no image is provided, generates a test fixture using the CLI.
#
# Exit codes:
#   0 — All checks passed
#   1 — Missing required tool (strict mode) or usage error
#   2 — One or more checks failed

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass() { echo -e "  ${GREEN}PASS${NC}: $1"; }
fail() { echo -e "  ${RED}FAIL${NC}: $1"; FAILURES=$((FAILURES + 1)); }
warn() { echo -e "  ${YELLOW}WARN${NC}: $1"; }
skip() { echo -e "  ${YELLOW}SKIP${NC}: $1"; }

STRICT=false
IMAGE=""
EXPECTED_COPYRIGHT=""

while [ $# -gt 0 ]; do
    case "$1" in
        --strict) STRICT=true; shift ;;
        *)
            if [ -z "$IMAGE" ]; then
                IMAGE="$1"
            elif [ -z "$EXPECTED_COPYRIGHT" ]; then
                EXPECTED_COPYRIGHT="$1"
            else
                echo "Error: unexpected argument: $1"
                exit 1
            fi
            shift
            ;;
    esac
done

FAILURES=0
TEMP_DIR=""

cleanup() {
    if [ -n "$TEMP_DIR" ] && [ -d "$TEMP_DIR" ]; then
        rm -rf "$TEMP_DIR"
    fi
}
trap cleanup EXIT

if [ -z "$IMAGE" ]; then
    TEMP_DIR=$(mktemp -d -t stegoeggo_conformance_XXXXXX)
    IMAGE="$TEMP_DIR/test_protected.png"
    echo "=== Auto-generating test fixture ==="

    CLI_BIN=""
    if command -v stegoeggo &>/dev/null; then
        CLI_BIN="stegoeggo"
    elif [ -f "target/release/stegoeggo" ]; then
        CLI_BIN="target/release/stegoeggo"
    elif [ -f "target/debug/stegoeggo" ]; then
        CLI_BIN="target/debug/stegoeggo"
    else
        echo "Error: stegoeggo CLI not found. Build with: cargo build --release -p stegoeggo-cli"
        exit 1
    fi

    # Generate a small test image via ImageMagick
    if command -v magick &>/dev/null || command -v convert &>/dev/null; then
        CMD=$(command -v magick 2>/dev/null || command -v convert 2>/dev/null)
        $CMD -size 64x64 xc:blue "$TEMP_DIR/test_input.png"
    else
        # Fallback: create minimal PNG manually (1x1 blue pixel)
        python3 -c "
import struct, zlib
def make_png(w, h, rgb):
    sig = b'\x89PNG\r\n\x1a\n'
    def chunk(ctype, data):
        c = ctype + data
        return struct.pack('>I', len(data)) + c + struct.pack('>I', zlib.crc32(c) & 0xffffffff)
    ihdr = struct.pack('>IIBBBBB', w, h, 8, 2, 0, 0, 0)
    raw = b''
    for _ in range(h):
        raw += b'\x00' + bytes(rgb) * w
    idat = zlib.compress(raw)
    return sig + chunk(b'IHDR', ihdr) + chunk(b'IDAT', idat) + chunk(b'IEND', b'')
with open('$TEMP_DIR/test_input.png', 'wb') as f:
    f.write(make_png(64, 64, (0, 0, 255)))
" 2>/dev/null || {
            echo "Error: Cannot generate test image. Install ImageMagick or ensure python3 is available."
            exit 1
        }
    fi

    $CLI_BIN "$TEMP_DIR/test_input.png" -o "$TEMP_DIR/" \
        --legal-claims \
        --copyright-holder "Test Corp" \
        --creator "Test Author" \
        --contact "legal@test.com" \
        --rights-url "https://example.com/rights" \
        --usage-terms "All rights reserved" \
        --ai-constraints "No AI training" \
        --no-ai-training 2>/dev/null

    # Find the output file
    IMAGE=$(find "$TEMP_DIR" -name "*_protected.*" -type f | head -1)
    if [ -z "$IMAGE" ]; then
        echo "Error: Failed to generate test fixture"
        exit 1
    fi
    EXPECTED_COPYRIGHT="Test Corp"
    echo "Generated: $IMAGE"
    echo ""
fi

if [ ! -f "$IMAGE" ]; then
    echo "Error: File not found: $IMAGE"
    exit 1
fi

echo "=== Metadata Conformance Check: $IMAGE ==="
echo ""

# ── Check tools ──────────────────────────────────────────────────────

HAS_EXIFTOOL=false
HAS_MAGICK=false
HAS_VIPS=false

if command -v exiftool &>/dev/null; then
    HAS_EXIFTOOL=true
fi
if command -v magick &>/dev/null || command -v convert &>/dev/null; then
    HAS_MAGICK=true
fi
if command -v vips &>/dev/null; then
    HAS_VIPS=true
fi

if ! $HAS_EXIFTOOL; then
    if $STRICT; then
        echo "Error: exiftool is required in strict mode but not found."
        echo "Install with: brew install exiftool (macOS) or apt install libimage-exiftool-perl (Linux)"
        exit 1
    else
        skip "exiftool not installed; external parser checks skipped"
        echo ""
        echo "=== Summary ==="
        echo -e "${YELLOW}Skipped: external tools not available.${NC}"
        exit 0
    fi
fi

# ── exiftool checks ───────────────────────────────────────────────────

echo "--- exiftool checks ---"

# Check for copyright in metadata
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
    fail "Copyright not found in metadata"
fi

# Check for Creator
CREATOR=$(exiftool -s3 -Creator "$IMAGE" 2>/dev/null || true)
if [ -n "$CREATOR" ]; then
    pass "Creator found: $CREATOR"
else
    warn "Creator not found (may be in non-standard field)"
fi

# Check for Contact
CONTACT=$(exiftool -s3 -Contact "$IMAGE" 2>/dev/null || true)
if [ -n "$CONTACT" ]; then
    pass "Contact found: $CONTACT"
else
    warn "Contact not found via exiftool"
fi

# Check for usage terms
USAGE=$(exiftool -s3 -UsageTerms "$IMAGE" 2>/dev/null || true)
if [ -n "$USAGE" ]; then
    pass "UsageTerms found: $USAGE"
else
    warn "UsageTerms not found via exiftool"
fi

# Check for AI constraints
AI=$(exiftool -s3 -AIConstraints "$IMAGE" 2>/dev/null || true)
if [ -n "$AI" ]; then
    pass "AIConstraints found: $AI"
else
    warn "AIConstraints not found via exiftool"
fi

# Check for DMI/copyright in XMP
XMP_DMI=$(exiftool -s3 -XMP-iptcExt:DMI-Prohibited "$IMAGE" 2>/dev/null || true)
if [ -n "$XMP_DMI" ]; then
    pass "XMP contains DMI-Prohibited: $XMP_DMI"
else
    warn "XMP DMI-Prohibited not found via exiftool"
fi

# Check for TDM reservation in XMP
TDM=$(exiftool -s3 -XMP-tdm:Reserve_tdm "$IMAGE" 2>/dev/null || true)
if [ -n "$TDM" ] && [ "$TDM" = "1" ]; then
    pass "XMP contains TDM reservation"
else
    warn "XMP TDM reservation not found via exiftool"
fi

# Check for protection seed in Description
SEED=$(exiftool -s3 -Description "$IMAGE" 2>/dev/null || true)
if echo "$SEED" | grep -q "Seed:"; then
    pass "Description contains protection seed"
else
    warn "Description does not contain protection seed"
fi

# ── Format-specific checks ────────────────────────────────────────────

FORMAT=$(exiftool -s3 -FileType "$IMAGE" 2>/dev/null || echo "unknown")
echo ""
echo "--- Format-specific checks ($FORMAT) ---"

case "$FORMAT" in
    PNG)
        if $HAS_MAGICK; then
            CMD=$(command -v magick 2>/dev/null || command -v convert 2>/dev/null)
            TEXT_CHUNKS=$($CMD identify -verbose "$IMAGE" 2>/dev/null | grep -i "copyright\|Creator\|AIConstraints\|UsageTerms" || true)
            if [ -n "$TEXT_CHUNKS" ]; then
                pass "PNG tEXt chunks contain legal fields"
            else
                warn "Could not verify PNG tEXt legal fields via ImageMagick"
            fi
        else
            skip "ImageMagick not available, skipping PNG chunk verification"
        fi
        ;;
    JPEG)
        COM=$(exiftool -s3 -Comment "$IMAGE" 2>/dev/null || true)
        if [ -n "$COM" ]; then
            pass "JPEG COM marker contains legal fields"
            if echo "$COM" | grep -q "Protection seed:"; then
                pass "JPEG COM contains protection seed"
            fi
        else
            warn "JPEG COM marker not found"
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
            skip "vips not available, skipping WebP chunk verification"
        fi
        ;;
    *)
        warn "Unknown format: $FORMAT"
        ;;
esac

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
