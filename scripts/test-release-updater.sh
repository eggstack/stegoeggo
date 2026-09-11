#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/stegoeggo-updater-tests.XXXXXX")"
SERVER_PID=""

cleanup() {
    if [[ -n "$SERVER_PID" ]]; then
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi
    rm -rf "$TEMP_ROOT"
}
trap cleanup EXIT

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

sha256() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    else
        shasum -a 256 "$1" | awk '{print $1}'
    fi
}

case "$(uname -s):$(uname -m)" in
    Linux:x86_64) TARGET="x86_64-unknown-linux-gnu" ;;
    Linux:aarch64|Linux:arm64) TARGET="aarch64-unknown-linux-gnu" ;;
    Darwin:x86_64) TARGET="x86_64-apple-darwin" ;;
    Darwin:arm64|Darwin:aarch64) TARGET="aarch64-apple-darwin" ;;
    *) fail "updater rehearsal requires a supported Unix host target" ;;
esac
ASSET_NAME="stegoeggo-$TARGET"

write_registry() {
    local version="$1"
    printf '{"versions":[{"num":"%s","yanked":false}]}\n' "$version" > \
        "$TEMP_ROOT/api/v1/crates/stegoeggo-cli"
}

write_candidate() {
    local version="$1"
    local asset_path="$TEMP_ROOT/releases/download/v0.4.1/$ASSET_NAME"
    mkdir -p "$(dirname "$asset_path")"
    printf '#!/usr/bin/env bash\nif [[ "${1:-}" == version ]]; then echo "stegoeggo %s"; fi\n' \
        "$version" > "$asset_path"
    chmod 0755 "$asset_path"
    sha256 "$asset_path" | awk -v name="$(basename "$asset_path")" '{print $1 "  " name}' > \
        "$asset_path.sha256"
}

write_bad_checksum() {
    local asset_path="$TEMP_ROOT/releases/download/v0.4.1/$ASSET_NAME"
    printf '%064d  %s\n' 0 "$(basename "$asset_path")" > "$asset_path.sha256"
}

write_bad_identity() {
    write_candidate "9.9.9"
}

PORT_FILE="$TEMP_ROOT/port"
python3 - "$TEMP_ROOT" "$PORT_FILE" <<'PY' &
import functools
import http.server
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
port_file = pathlib.Path(sys.argv[2])

server = http.server.ThreadingHTTPServer(
    ("127.0.0.1", 0),
    functools.partial(http.server.SimpleHTTPRequestHandler, directory=str(root)),
)
port_file.write_text(str(server.server_port))
server.serve_forever()
PY
SERVER_PID=$!
until [[ -s "$PORT_FILE" ]]; do sleep 0.05; done

BASE_URL="http://127.0.0.1:$(<"$PORT_FILE")"
CLI_BINARY="$ROOT_DIR/target/debug/stegoeggo"
if [[ ! -x "$CLI_BINARY" ]]; then
    cargo build --locked --package stegoeggo-cli --bin stegoeggo
fi

mkdir -p "$TEMP_ROOT/api/v1/crates"
write_registry "0.4.0"
CURRENT="$TEMP_ROOT/current"
cp "$CLI_BINARY" "$CURRENT"
chmod 0755 "$CURRENT"
output="$(STEGOEGGO_CRATES_API_URL="$BASE_URL/api/v1/crates/stegoeggo-cli" \
    STEGOEGGO_RELEASES_URL="$BASE_URL/releases" "$CURRENT" update)"
[[ "$output" == *"up to date"* ]] || fail "current-version update was not a no-op"

write_registry "0.4.1"
write_candidate "0.4.1"
cp "$CLI_BINARY" "$CURRENT"
chmod 0755 "$CURRENT"
STEGOEGGO_CRATES_API_URL="$BASE_URL/api/v1/crates/stegoeggo-cli" \
    STEGOEGGO_RELEASES_URL="$BASE_URL/releases" "$CURRENT" update >/dev/null
[[ "$($CURRENT version)" == "stegoeggo 0.4.1" ]] || fail "verified update did not replace the executable"

cp "$CLI_BINARY" "$CURRENT"
chmod 0755 "$CURRENT"
write_candidate "0.4.1"
write_bad_checksum
if STEGOEGGO_CRATES_API_URL="$BASE_URL/api/v1/crates/stegoeggo-cli" \
    STEGOEGGO_RELEASES_URL="$BASE_URL/releases" "$CURRENT" update >/dev/null 2>"$TEMP_ROOT/checksum.err"; then
    fail "checksum mismatch unexpectedly succeeded"
fi
[[ "$($CURRENT version)" == "stegoeggo 0.4.0" ]] || fail "checksum failure replaced the executable"

write_bad_identity
sha256 "$TEMP_ROOT/releases/download/v0.4.1/$ASSET_NAME" | \
    awk -v name="$ASSET_NAME" '{print $1 "  " name}' > \
    "$TEMP_ROOT/releases/download/v0.4.1/$ASSET_NAME.sha256"
if STEGOEGGO_CRATES_API_URL="$BASE_URL/api/v1/crates/stegoeggo-cli" \
    STEGOEGGO_RELEASES_URL="$BASE_URL/releases" "$CURRENT" update >/dev/null 2>"$TEMP_ROOT/identity.err"; then
    fail "candidate identity mismatch unexpectedly succeeded"
fi
[[ "$($CURRENT version)" == "stegoeggo 0.4.0" ]] || fail "identity failure replaced the executable"

echo "Release updater tests passed"
