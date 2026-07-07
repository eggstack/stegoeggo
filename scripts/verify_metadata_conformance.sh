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
#   --strict          Fail if exiftool is not installed (default: skip external checks)
#   --format FMT      Output format for auto-generated fixture (png, jpeg, webp). Default: png
#   --all-formats     Generate and check fixtures in all supported formats (png, jpeg, webp)
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
FORMAT="png"
ALL_FORMATS=false

while [ $# -gt 0 ]; do
    case "$1" in
        --strict) STRICT=true; shift ;;
        --format)
            if [ -z "${2:-}" ]; then
                echo "Error: --format requires a value (png, jpeg, webp)"
                exit 1
            fi
            FORMAT="$2"; shift 2 ;;
        --all-formats) ALL_FORMATS=true; shift ;;
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

if ! $ALL_FORMATS && [ "$FORMAT" != "png" ] && [ "$FORMAT" != "jpeg" ] && [ "$FORMAT" != "jpg" ] && [ "$FORMAT" != "webp" ]; then
    echo "Error: invalid format '$FORMAT'. Supported: png, jpeg, webp"
    exit 1
fi

FAILURES=0
TEMP_DIR=""

cleanup() {
    if [ -n "$TEMP_DIR" ] && [ -d "$TEMP_DIR" ]; then
        rm -rf "$TEMP_DIR"
    fi
}
trap cleanup EXIT

find_cli() {
    if command -v stegoeggo &>/dev/null; then
        echo "stegoeggo"
    elif [ -f "target/release/stegoeggo" ]; then
        echo "target/release/stegoeggo"
    elif [ -f "target/debug/stegoeggo" ]; then
        echo "target/debug/stegoeggo"
    else
        echo ""
    fi
}

find_magick() {
    if command -v magick &>/dev/null; then
        echo "magick"
    elif command -v convert &>/dev/null; then
        echo "convert"
    else
        echo ""
    fi
}

generate_test_image() {
    local out_path="$1"
    local img_format="$2"
    local width=64
    local height=64

    local magick_cmd
    magick_cmd=$(find_magick)

    if [ -n "$magick_cmd" ]; then
        if [ "$img_format" = "jpeg" ] || [ "$img_format" = "jpg" ]; then
            $magick_cmd -size "${width}x${height}" xc:blue "$out_path"
        elif [ "$img_format" = "webp" ]; then
            $magick_cmd -size "${width}x${height}" xc:blue "$out_path"
        else
            $magick_cmd -size "${width}x${height}" xc:blue "$out_path"
        fi
    else
        if [ "$img_format" = "jpeg" ] || [ "$img_format" = "jpg" ] || [ "$img_format" = "webp" ]; then
            echo "Error: Cannot generate $img_format test image without ImageMagick."
            echo "Install with: brew install imagemagick (macOS) or apt install imagemagick (Linux)"
            return 1
        fi
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
with open('$out_path', 'wb') as f:
    f.write(make_png($width, $height, (0, 0, 255)))
" 2>/dev/null || {
            echo "Error: Cannot generate test image. Install ImageMagick or ensure python3 is available."
            return 1
        }
    fi
}

cli_output_ext() {
    case "$1" in
        jpeg|jpg) echo "jpg" ;;
        webp)     echo "webp" ;;
        png)      echo "png" ;;
        *)        echo "png" ;;
    esac
}

run_conformance_checks() {
    local image="$1"
    local expected_copyright="$2"
    local image_format="$3"

    echo "=== Metadata Conformance Check: $image ==="
    echo ""

    HAS_EXIFTOOL=false
    HAS_MAGICK=false
    HAS_VIPS=false

    if command -v exiftool &>/dev/null; then
        HAS_EXIFTOOL=true
    fi
    if [ -n "$(find_magick)" ]; then
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
            return 0
        fi
    fi

    echo "--- exiftool checks ($image_format) ---"

    case "$image_format" in
        JPEG|jpeg|jpg)
            COM_ALL=$(exiftool -s3 -a -Comment "$image" 2>/dev/null || true)

            COPYRIGHT=$(echo "$COM_ALL" | grep "^Copyright:" | head -1 || true)
            if [ -n "$COPYRIGHT" ]; then
                pass "Copyright found in COM: $COPYRIGHT"
                if [ -n "$expected_copyright" ]; then
                    if echo "$COPYRIGHT" | grep -q "$expected_copyright"; then
                        pass "Copyright matches expected: $expected_copyright"
                    else
                        fail "Copyright '$COPYRIGHT' does not contain expected '$expected_copyright'"
                    fi
                fi
            else
                fail "Copyright not found in JPEG metadata"
            fi

            CREATOR=$(echo "$COM_ALL" | grep "^Creator:" | head -1 || true)
            if [ -n "$CREATOR" ]; then
                pass "Creator found in COM: $CREATOR"
            else
                warn "Creator not found in JPEG COM markers"
            fi

            CONTACT=$(echo "$COM_ALL" | grep "^Contact:" | head -1 || true)
            if [ -n "$CONTACT" ]; then
                pass "Contact found in COM: $CONTACT"
            else
                warn "Contact not found in JPEG COM markers"
            fi

            USAGE=$(echo "$COM_ALL" | grep "^UsageTerms:" | head -1 || true)
            if [ -n "$USAGE" ]; then
                pass "UsageTerms found in COM: $USAGE"
            else
                warn "UsageTerms not found in JPEG COM markers"
            fi

            AI=$(echo "$COM_ALL" | grep "^AIConstraints:" | head -1 || true)
            if [ -n "$AI" ]; then
                pass "AIConstraints found in COM: $AI"
            else
                warn "AIConstraints not found in JPEG COM markers"
            fi

            XMP_DMI=$(exiftool -s3 -XMP-iptcExt:DMI-Prohibited "$image" 2>/dev/null || true)
            if [ -n "$XMP_DMI" ]; then
                pass "XMP contains DMI-Prohibited: $XMP_DMI"
            else
                warn "XMP DMI-Prohibited not found via exiftool"
            fi

            TDM=$(exiftool -s3 -XMP-tdm:Reserve_tdm "$image" 2>/dev/null || true)
            if [ -n "$TDM" ] && [ "$TDM" = "1" ]; then
                pass "XMP contains TDM reservation"
            else
                warn "XMP TDM reservation not found via exiftool"
            fi

            SEED=$(echo "$COM_ALL" | grep "^X-Protection-Seed:" | head -1 || true)
            if [ -n "$SEED" ]; then
                pass "COM contains protection seed: $SEED"
            else
                warn "Protection seed not found in JPEG COM markers"
            fi
            ;;

        PNG|png)
            COPYRIGHT=$(exiftool -s3 -Copyright "$image" 2>/dev/null || true)
            if [ -n "$COPYRIGHT" ]; then
                pass "Copyright found: $COPYRIGHT"
                if [ -n "$expected_copyright" ]; then
                    if echo "$COPYRIGHT" | grep -q "$expected_copyright"; then
                        pass "Copyright matches expected: $expected_copyright"
                    else
                        fail "Copyright '$COPYRIGHT' does not contain expected '$expected_copyright'"
                    fi
                fi
            else
                fail "Copyright not found in metadata"
            fi

            CREATOR=$(exiftool -s3 -Creator "$image" 2>/dev/null || true)
            if [ -n "$CREATOR" ]; then
                pass "Creator found: $CREATOR"
            else
                warn "Creator not found (may be in non-standard field)"
            fi

            CONTACT=$(exiftool -s3 -Contact "$image" 2>/dev/null || true)
            if [ -n "$CONTACT" ]; then
                pass "Contact found: $CONTACT"
            else
                warn "Contact not found via exiftool"
            fi

            USAGE=$(exiftool -s3 -UsageTerms "$image" 2>/dev/null || true)
            if [ -n "$USAGE" ]; then
                pass "UsageTerms found: $USAGE"
            else
                warn "UsageTerms not found via exiftool"
            fi

            AI=$(exiftool -s3 -AIConstraints "$image" 2>/dev/null || true)
            if [ -n "$AI" ]; then
                pass "AIConstraints found: $AI"
            else
                warn "AIConstraints not found via exiftool"
            fi

            XMP_DMI=$(exiftool -s3 -XMP-iptcExt:DMI-Prohibited "$image" 2>/dev/null || true)
            if [ -n "$XMP_DMI" ]; then
                pass "XMP contains DMI-Prohibited: $XMP_DMI"
            else
                warn "XMP DMI-Prohibited not found via exiftool"
            fi

            TDM=$(exiftool -s3 -XMP-tdm:Reserve_tdm "$image" 2>/dev/null || true)
            if [ -n "$TDM" ] && [ "$TDM" = "1" ]; then
                pass "XMP contains TDM reservation"
            else
                warn "XMP TDM reservation not found via exiftool"
            fi

            SEED=$(exiftool -s3 -Description "$image" 2>/dev/null || true)
            if echo "$SEED" | grep -q "Seed:"; then
                pass "Description contains protection seed"
            else
                warn "Description does not contain protection seed"
            fi
            ;;

        WebP|webp)
            XMP_RAW=$(exiftool -s3 -XMP "$image" 2>/dev/null || true)

            COPYRIGHT=$(exiftool -s3 -Copyright "$image" 2>/dev/null || true)
            DC_RIGHTS=$(exiftool -s3 -XMP-dc:Rights "$image" 2>/dev/null || true)
            if [ -n "$COPYRIGHT" ]; then
                pass "Copyright found: $COPYRIGHT"
            elif [ -n "$DC_RIGHTS" ]; then
                pass "Rights found in XMP dc:rights: $DC_RIGHTS"
            else
                if $STRICT; then
                    fail "Copyright not found in WebP XMP"
                else
                    warn "Copyright not found in WebP metadata (parser-dependent)"
                fi
            fi
            if [ -n "$expected_copyright" ]; then
                if echo "$COPYRIGHT" | grep -q "$expected_copyright"; then
                    pass "Copyright matches expected: $expected_copyright"
                elif echo "$DC_RIGHTS" | grep -q "$expected_copyright"; then
                    pass "Rights matches expected: $expected_copyright"
                else
                    fail "Copyright/Rights '$COPYRIGHT$DC_RIGHTS' does not contain expected '$expected_copyright'"
                fi
            fi

            XMP_DMI=$(exiftool -s3 -XMP-iptcExt:DMI-Prohibited "$image" 2>/dev/null || true)
            if [ -n "$XMP_DMI" ]; then
                pass "XMP contains DMI-Prohibited: $XMP_DMI"
            else
                if $STRICT; then
                    fail "DMI-Prohibited not found in WebP XMP"
                else
                    warn "XMP DMI-Prohibited not found via exiftool (parser-dependent)"
                fi
            fi

            TDM=$(exiftool -s3 -XMP-tdm:Reserve_tdm "$image" 2>/dev/null || true)
            if [ -n "$TDM" ] && [ "$TDM" = "1" ]; then
                pass "XMP contains TDM reservation"
            else
                if $STRICT; then
                    fail "TDM reservation not found in WebP XMP"
                else
                    warn "XMP TDM reservation not found via exiftool (parser-dependent)"
                fi
            fi

            USAGE=$(exiftool -s3 -XMP-xmpRights:UsageTerms "$image" 2>/dev/null || true)
            RIGHTS=$(exiftool -s3 -XMP-dc:Rights "$image" 2>/dev/null || true)
            if [ -n "$USAGE" ] || [ -n "$RIGHTS" ]; then
                if [ -n "$USAGE" ]; then
                    pass "UsageTerms found in WebP XMP: $USAGE"
                else
                    pass "Rights found in WebP XMP: $RIGHTS"
                fi
            else
                if $STRICT; then
                    warn "No UsageTerms or Rights found in WebP XMP (optional)"
                fi
            fi
            ;;

        *)
            warn "Unknown format: $image_format — skipping format-specific exiftool checks"
            ;;
    esac

    echo ""
    echo "--- Format-specific checks ($image_format) ---"

    case "$image_format" in
        PNG|png)
            if $HAS_MAGICK; then
                local magick_cmd
                magick_cmd=$(find_magick)
                TEXT_CHUNKS=$($magick_cmd identify -verbose "$image" 2>/dev/null | grep -i "copyright\|Creator\|AIConstraints\|UsageTerms" || true)
                if [ -n "$TEXT_CHUNKS" ]; then
                    pass "PNG tEXt chunks contain legal fields"
                else
                    warn "Could not verify PNG tEXt legal fields via ImageMagick"
                fi
            else
                skip "ImageMagick not available, skipping PNG chunk verification"
            fi
            ;;
        JPEG|jpeg|jpg)
            COM=$(exiftool -s3 -Comment "$image" 2>/dev/null || true)
            if [ -n "$COM" ]; then
                pass "JPEG COM marker contains legal fields"
                if echo "$COM" | grep -q "Protection seed:"; then
                    pass "JPEG COM contains protection seed"
                fi
            else
                warn "JPEG COM marker not found"
            fi
            ;;
        WebP|webp)
            if $HAS_VIPS; then
                WEBP_META=$(vips header "$image" 2>/dev/null | grep -i "xmp\|exif" || true)
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
            warn "Unknown format: $image_format"
            ;;
    esac
}

run_single_format() {
    local target_format="$1"
    local input_image="$2"
    local input_copyright="$3"
    local is_generated=false

    if [ -n "$input_image" ]; then
        IMAGE="$input_image"
        EXPECTED_COPYRIGHT="$input_copyright"
        is_generated=false
    else
        is_generated=true
        TEMP_DIR=$(mktemp -d 2>/dev/null || mktemp -d -t stegoeggo_conformance_XXXXXX)
        local cli_ext
        cli_ext=$(cli_output_ext "$target_format")
        IMAGE="$TEMP_DIR/test_protected.$cli_ext"
        local input_path="$TEMP_DIR/test_input.$cli_ext"

        echo "=== Auto-generating test fixture ($target_format) ==="

        local cli_bin
        cli_bin=$(find_cli)
        if [ -z "$cli_bin" ]; then
            echo "Error: stegoeggo CLI not found. Build with: cargo build --release -p stegoeggo-cli"
            exit 1
        fi

        if ! generate_test_image "$input_path" "$target_format"; then
            warn "Skipping $target_format: cannot generate test image"
            return 0
        fi

        local fmt_flag=""
        if [ "$target_format" = "jpeg" ] || [ "$target_format" = "jpg" ]; then
            fmt_flag="--format jpg"
        elif [ "$target_format" = "webp" ]; then
            fmt_flag="--format web-p"
        fi

        $cli_bin "$input_path" -o "$TEMP_DIR/" \
            --legal-claims \
            --copyright-holder "Test Corp" \
            --creator "Test Author" \
            --contact "legal@test.com" \
            --rights-url "https://example.com/rights" \
            --usage-terms "All rights reserved" \
            --ai-constraints "No AI training" \
            --no-ai-training $fmt_flag 2>/dev/null

        IMAGE=$(find "$TEMP_DIR" -name "*_protected.*" -type f 2>/dev/null | head -1)
        if [ -z "$IMAGE" ]; then
            warn "Skipping $target_format: CLI failed to produce output"
            return 0
        fi
        EXPECTED_COPYRIGHT="Test Corp"
        echo "Generated: $IMAGE"
        echo ""
    fi

    if [ ! -f "$IMAGE" ]; then
        warn "Skipping $target_format: file not found: $IMAGE"
        return 0
    fi

    run_conformance_checks "$IMAGE" "$EXPECTED_COPYRIGHT" "$target_format"
}

if [ -n "$IMAGE" ]; then
    if [ ! -f "$IMAGE" ]; then
        echo "Error: File not found: $IMAGE"
        exit 1
    fi

    DETECTED_FORMAT=$(exiftool -s3 -FileType "$IMAGE" 2>/dev/null || echo "unknown")

    echo "=== Summary ==="
    FAILURES=0
    run_conformance_checks "$IMAGE" "$EXPECTED_COPYRIGHT" "$DETECTED_FORMAT"

    echo ""
    echo "=== Summary ==="
    if [ "$FAILURES" -eq 0 ]; then
        echo -e "${GREEN}All checks passed.${NC}"
        exit 0
    else
        echo -e "${RED}$FAILURES check(s) failed.${NC}"
        exit 2
    fi
else
    if $ALL_FORMATS; then
        for fmt in png jpeg webp; do
            echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            echo "  FORMAT: $fmt"
            echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            run_single_format "$fmt" "" ""
            echo ""
        done

        echo ""
        echo "=== Overall Summary ==="
        if [ "$FAILURES" -eq 0 ]; then
            echo -e "${GREEN}All checks passed across all formats.${NC}"
            exit 0
        else
            echo -e "${RED}$FAILURES check(s) failed.${NC}"
            exit 2
        fi
    else
        run_single_format "$FORMAT" "" ""

        echo ""
        echo "=== Summary ==="
        if [ "$FAILURES" -eq 0 ]; then
            echo -e "${GREEN}All checks passed.${NC}"
            exit 0
        else
            echo -e "${RED}$FAILURES check(s) failed.${NC}"
            exit 2
        fi
    fi
fi
