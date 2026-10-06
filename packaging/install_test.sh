#!/usr/bin/env bash
# Regression tests for packaging/install.sh: platform mapping and installed name.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INSTALLER="$ROOT_DIR/packaging/install.sh"
TEMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/stegoeggo-install-test.XXXXXX")"
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

# A faked uname must not leak into unrelated tools the installer runs, so the
# installer's temp cleanup gets the real rm ahead of any platform shim.
REAL_BIN="$TEMP_ROOT/real-bin"
mkdir -p "$REAL_BIN"
ln -s /bin/rm "$REAL_BIN/rm"

make_uname() {
    local dir="$1"
    local os="$2"
    local arch="$3"
    mkdir -p "$dir"
    # shellcheck disable=SC2016 # single quotes are intentional: ${1:-} is expanded by the shim
    printf '#!/usr/bin/env bash\nif [[ "${1:-}" == -s ]]; then printf "%%s\\n" "%s"; else printf "%%s\\n" "%s"; fi\n' "$os" "$arch" > "$dir/uname"
    chmod 0755 "$dir/uname"
}

uname_dir() {
    local os="$1"
    local arch="$2"
    local slug
    slug="$(printf '%s-%s' "$os" "$arch" | tr -c 'A-Za-z0-9' '-')"
    printf '%s/uname-%s' "$TEMP_ROOT" "$slug"
}

expect_target() {
    local os="$1"
    local arch="$2"
    local expected="$3"
    local dir
    dir="$(uname_dir "$os" "$arch")"
    make_uname "$dir" "$os" "$arch"
    local actual
    actual="$(PATH="$dir:$PATH" bash -c "source '$INSTALLER'; target_for_platform")" || fail "$os:$arch did not resolve to a target"
    [[ "$actual" == "$expected" ]] || fail "$os:$arch resolved to '$actual', expected '$expected'"
}

expect_unsupported() {
    local os="$1"
    local arch="$2"
    local dir
    dir="$(uname_dir "bad-$os" "$arch")"
    make_uname "$dir" "$os" "$arch"
    if PATH="$dir:$PATH" bash -c "source '$INSTALLER'; target_for_platform" >/dev/null 2>&1; then
        fail "$os:$arch unexpectedly resolved to a target"
    fi
}

# Windows under Git Bash / MSYS / Cygwin must resolve to the published asset target.
expect_target "MINGW64_NT-10.0-19045" "x86_64" "x86_64-pc-windows-msvc"
expect_target "MINGW64_NT-6.1" "amd64" "x86_64-pc-windows-msvc"
expect_target "MSYS_NT-10.0-22631" "x86_64" "x86_64-pc-windows-msvc"
expect_target "CYGWIN_NT-10.0" "amd64" "x86_64-pc-windows-msvc"
expect_target "Windows_NT" "x86_64" "x86_64-pc-windows-msvc"

# Existing Linux/Darwin mappings must stay unchanged.
expect_target "Linux" "x86_64" "x86_64-unknown-linux-gnu"
expect_target "Linux" "aarch64" "aarch64-unknown-linux-gnu"
expect_target "Linux" "arm64" "aarch64-unknown-linux-gnu"
expect_target "Darwin" "x86_64" "x86_64-apple-darwin"
expect_target "Darwin" "arm64" "aarch64-apple-darwin"
expect_target "Darwin" "aarch64" "aarch64-apple-darwin"

# Anything else still falls back to Cargo.
expect_unsupported "FreeBSD" "x86_64"
expect_unsupported "MINGW64_NT-10.0-19045" "aarch64"

make_binary() {
    local path="$1"
    local version="$2"
    mkdir -p "$(dirname "$path")"
    printf '%s\n' '#!/usr/bin/env bash' "if [[ \"\${1:-}\" == version ]]; then echo \"stegoeggo $version\"; fi" > "$path"
    chmod 0755 "$path"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$path" > "$path.sha256"
    else
        shasum -a 256 "$path" > "$path.sha256"
    fi
}

RELEASE_ROOT="$TEMP_ROOT/releases"
make_binary "$RELEASE_ROOT/latest/download/stegoeggo-x86_64-unknown-linux-gnu" "1.2.3"
make_binary "$RELEASE_ROOT/latest/download/stegoeggo-x86_64-pc-windows-msvc.exe" "1.2.3"

PORT_FILE="$TEMP_ROOT/port"
python3 - "$RELEASE_ROOT" "$PORT_FILE" <<'PY' &
import functools
import http.server
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
port_file = pathlib.Path(sys.argv[2])


class Handler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, format, *args):
        pass


server = http.server.ThreadingHTTPServer(
    ("127.0.0.1", 0), functools.partial(Handler, directory=str(root))
)
port_file.write_text(str(server.server_port))
server.serve_forever()
PY
SERVER_PID=$!
until [[ -s "$PORT_FILE" ]]; do sleep 0.05; done
BASE_URL="http://127.0.0.1:$(<"$PORT_FILE")"

# The Windows asset installs as stegoeggo.exe; every other target keeps stegoeggo.
MINGW_BIN="$(uname_dir mingw x86_64)"
make_uname "$MINGW_BIN" "MINGW64_NT-10.0-19045" "x86_64"
HOME="$TEMP_ROOT/home-windows" PATH="$MINGW_BIN:$REAL_BIN:$PATH" STEGOEGGO_RELEASES_URL="$BASE_URL" "$INSTALLER" >"$TEMP_ROOT/windows.out"
[[ -x "$TEMP_ROOT/home-windows/.local/bin/stegoeggo.exe" ]] || fail "Windows install did not create stegoeggo.exe"
[[ ! -e "$TEMP_ROOT/home-windows/.local/bin/stegoeggo" ]] || fail "Windows install created an extension-less stegoeggo"

LINUX_BIN="$(uname_dir linux x86_64)"
make_uname "$LINUX_BIN" "Linux" "x86_64"
HOME="$TEMP_ROOT/home-linux" PATH="$LINUX_BIN:$REAL_BIN:$PATH" STEGOEGGO_RELEASES_URL="$BASE_URL" "$INSTALLER" >"$TEMP_ROOT/linux.out"
[[ -x "$TEMP_ROOT/home-linux/.local/bin/stegoeggo" ]] || fail "Linux install did not create stegoeggo"
[[ ! -e "$TEMP_ROOT/home-linux/.local/bin/stegoeggo.exe" ]] || fail "Linux install unexpectedly created stegoeggo.exe"

# A Windows-shaped uname with no published asset must still fall back to Cargo.
MOCK_CARGO="$TEMP_ROOT/mock-cargo"
mkdir -p "$MOCK_CARGO"
# shellcheck disable=SC2016 # single quotes are intentional: ${CARGO_MARKER:?} is expanded by the mock
printf '#!/usr/bin/env bash\ntouch "${CARGO_MARKER:?}"\n' > "$MOCK_CARGO/cargo"
chmod 0755 "$MOCK_CARGO/cargo"
UNSUPPORTED_BIN="$(uname_dir unsupported aarch64)"
make_uname "$UNSUPPORTED_BIN" "MINGW64_NT-10.0-19045" "aarch64"
CARGO_MARKER="$TEMP_ROOT/cargo.marker" HOME="$TEMP_ROOT/home-unsupported" PATH="$UNSUPPORTED_BIN:$MOCK_CARGO:$REAL_BIN:$PATH" STEGOEGGO_RELEASES_URL="$BASE_URL" "$INSTALLER" >/dev/null
[[ -f "$TEMP_ROOT/cargo.marker" ]] || fail "unsupported Windows arch did not fall back to Cargo"

echo "install.sh tests passed"