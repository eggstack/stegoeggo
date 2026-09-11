#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INSTALLER="$ROOT_DIR/packaging/install.sh"
TEMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/stegoeggo-installer-tests.XXXXXX")"
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

assert_contains() {
    local needle="$1"
    local file="$2"
    rg -F "$needle" "$file" >/dev/null || {
        echo "Contents of $file:" >&2
        sed -n '1,40p' "$file" >&2
        fail "'$needle' not found in $file"
    }
}

make_binary() {
    local path="$1"
    local version="$2"
    mkdir -p "$(dirname "$path")"
    printf '%s\n' '#!/usr/bin/env bash' "if [[ \"\${1:-}\" == version ]]; then echo \"stegoeggo $version\"; elif [[ \"\${1:-}\" == --help ]]; then echo help; fi" > "$path"
    chmod 0755 "$path"
}

make_release_tree() {
    local root="$1"
    local version="$2"
    local targets=(
        x86_64-unknown-linux-gnu
        aarch64-unknown-linux-gnu
        x86_64-apple-darwin
        aarch64-apple-darwin
        x86_64-pc-windows-msvc
    )
    for target in "${targets[@]}"; do
        local asset="stegoeggo-$target"
        [[ "$target" == *-pc-windows-msvc ]] && asset+=".exe"
        make_binary "$root/releases/latest/download/$asset" "$version"
        make_binary "$root/releases/download/v$version/$asset" "$version"
        for path in "$root/releases/latest/download/$asset" "$root/releases/download/v$version/$asset"; do
            sha256sum "$path" > "$path.sha256"
        done
    done
    cp "$ROOT_DIR/packaging/install.sh" "$root/releases/latest/download/install.sh"
    cp "$ROOT_DIR/packaging/install.ps1" "$root/releases/latest/download/install.ps1"
}

PORT_FILE="$TEMP_ROOT/port"
LOG_FILE="$TEMP_ROOT/requests.log"
python3 - "$TEMP_ROOT" "$PORT_FILE" "$LOG_FILE" <<'PY' &
import functools
import http.server
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
port_file = pathlib.Path(sys.argv[2])
log_file = pathlib.Path(sys.argv[3])

class Handler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, format, *args):
        with log_file.open("a") as handle:
            handle.write(self.path + "\n")

server = http.server.ThreadingHTTPServer(
    ("127.0.0.1", 0), functools.partial(Handler, directory=str(root))
)
port_file.write_text(str(server.server_port))
server.serve_forever()
PY
SERVER_PID=$!
until [[ -s "$PORT_FILE" ]]; do sleep 0.05; done
BASE_URL="http://127.0.0.1:$(<"$PORT_FILE")/good/releases"

RELEASE_ROOT="$TEMP_ROOT/good"
make_release_tree "$RELEASE_ROOT" "1.2.3"
HOME="$TEMP_ROOT/home-good" STEGOEGGO_RELEASES_URL="$BASE_URL" "$INSTALLER" --version 1.2.3 >"$TEMP_ROOT/good.out"
[[ -x "$TEMP_ROOT/home-good/.local/bin/stegoeggo" ]] || fail "user-local install was not created"
assert_contains "stegoeggo 1.2.3" "$TEMP_ROOT/good.out"
current_target=$(bash -c "source '$INSTALLER'; target_for_platform")
assert_contains "/good/releases/download/v1.2.3/stegoeggo-$current_target" "$LOG_FILE"

for platform in "Linux x86_64" "Linux aarch64" "Darwin arm64"; do
    mock_bin="$TEMP_ROOT/mock-$RANDOM"
    mkdir -p "$mock_bin"
    printf "#!/usr/bin/env bash\nif [[ \"\${1:-}\" == -s ]]; then printf \"%%s\\n\" \"%s\"; else printf \"%%s\\n\" \"%s\"; fi\n" "${platform% *}" "${platform#* }" > "$mock_bin/uname"
    chmod 0755 "$mock_bin/uname"
    HOME="$TEMP_ROOT/home-${platform// /-}" PATH="$mock_bin:$PATH" STEGOEGGO_RELEASES_URL="$BASE_URL" "$INSTALLER" >/dev/null
done

BAD_IDENTITY="$TEMP_ROOT/bad-identity"
make_release_tree "$BAD_IDENTITY" "1.2.3"
make_binary "$BAD_IDENTITY/releases/download/v1.2.3/stegoeggo-$current_target" "9.9.9"
sha256sum "$BAD_IDENTITY/releases/download/v1.2.3/stegoeggo-$current_target" > "$BAD_IDENTITY/releases/download/v1.2.3/stegoeggo-$current_target.sha256"
if HOME="$TEMP_ROOT/home-bad-identity" STEGOEGGO_RELEASES_URL="http://127.0.0.1:$(<"$PORT_FILE")/bad-identity/releases" "$INSTALLER" --version 1.2.3 >/dev/null 2>"$TEMP_ROOT/bad-identity.err"; then
    fail "candidate identity mismatch unexpectedly succeeded"
fi
assert_contains "candidate version" "$TEMP_ROOT/bad-identity.err"
[[ ! -e "$TEMP_ROOT/home-bad-identity/.local/bin/stegoeggo" ]] || fail "identity failure installed a binary"

BAD_CHECKSUM="$TEMP_ROOT/bad-checksum"
make_release_tree "$BAD_CHECKSUM" "1.2.3"
printf '%s\n' "00" > "$BAD_CHECKSUM/releases/latest/download/stegoeggo-$current_target.sha256"
if HOME="$TEMP_ROOT/home-bad-checksum" STEGOEGGO_RELEASES_URL="http://127.0.0.1:$(<"$PORT_FILE")/bad-checksum/releases" "$INSTALLER" >/dev/null 2>"$TEMP_ROOT/bad-checksum.err"; then
    fail "checksum mismatch unexpectedly succeeded"
fi
assert_contains "checksum" "$TEMP_ROOT/bad-checksum.err"
[[ ! -e "$TEMP_ROOT/home-bad-checksum/.local/bin/stegoeggo" ]] || fail "checksum failure installed a binary"

MISSING="$TEMP_ROOT/missing"
mkdir -p "$MISSING/releases/latest/download"
MOCK_CARGO="$TEMP_ROOT/mock-cargo"
mkdir -p "$MOCK_CARGO"
printf "#!/usr/bin/env bash\ntouch \"\${CARGO_MARKER:?}\"\n" > "$MOCK_CARGO/cargo"
chmod 0755 "$MOCK_CARGO/cargo"
CARGO_MARKER="$TEMP_ROOT/cargo-404.marker" HOME="$TEMP_ROOT/home-404" PATH="$MOCK_CARGO:$PATH" STEGOEGGO_RELEASES_URL="http://127.0.0.1:$(<"$PORT_FILE")/missing/releases" "$INSTALLER" >/dev/null
[[ -f "$TEMP_ROOT/cargo-404.marker" ]] || fail "404 did not fall back to Cargo"

if CARGO_MARKER="$TEMP_ROOT/cargo-network.marker" HOME="$TEMP_ROOT/home-network" PATH="$MOCK_CARGO:$PATH" STEGOEGGO_RELEASES_URL="http://127.0.0.1:9/releases" "$INSTALLER" >/dev/null 2>"$TEMP_ROOT/network.err"; then
    fail "network failure unexpectedly succeeded"
fi
[[ ! -f "$TEMP_ROOT/cargo-network.marker" ]] || fail "network failure incorrectly fell back to Cargo"

UNSUPPORTED="$TEMP_ROOT/mock-unsupported"
mkdir -p "$UNSUPPORTED"
printf "#!/usr/bin/env bash\nif [[ \"\${1:-}\" == -s ]]; then echo FreeBSD; else echo x86_64; fi\n" > "$UNSUPPORTED/uname"
chmod 0755 "$UNSUPPORTED/uname"
CARGO_MARKER="$TEMP_ROOT/cargo-unsupported.marker" HOME="$TEMP_ROOT/home-unsupported" PATH="$UNSUPPORTED:$MOCK_CARGO:$PATH" STEGOEGGO_RELEASES_URL="$BASE_URL" "$INSTALLER" >/dev/null
[[ -f "$TEMP_ROOT/cargo-unsupported.marker" ]] || fail "unsupported target did not fall back to Cargo"

MOCK_ID="$TEMP_ROOT/mock-id"
mkdir -p "$MOCK_ID"
printf '#!/usr/bin/env bash\necho 0\n' > "$MOCK_ID/id"
chmod 0755 "$MOCK_ID/id"
destination=$(PATH="$MOCK_ID:$PATH" bash -c "source '$INSTALLER'; install_destination")
[[ "$destination" == "/usr/local/bin" ]] || fail "root destination was '$destination'"

assert_contains "requires Bash" "$INSTALLER"
echo "Release installer tests passed"
