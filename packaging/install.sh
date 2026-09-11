#!/usr/bin/env bash

if [ -z "${BASH_VERSION:-}" ]; then
    echo "stegoeggo installer requires Bash; run the downloaded script with bash" >&2
    exit 2
fi
if [ "$(set -o | awk '$1 == "posix" {print $2}')" = "on" ]; then
    echo "stegoeggo installer requires Bash; run the downloaded script with bash" >&2
    exit 2
fi

set -euo pipefail

readonly DEFAULT_RELEASES_URL="https://github.com/eggstack/stegoeggo/releases"
readonly REPOSITORY_RELEASES_URL="${STEGOEGGO_RELEASES_URL:-$DEFAULT_RELEASES_URL}"

usage() {
    cat <<'EOF'
Usage: install.sh [--version X.Y.Z]

Install the prebuilt stegoeggo CLI. Without --version, the latest GitHub
Release asset is used. Unsupported targets and missing binary assets may fall
back to `cargo install stegoeggo-cli --locked`.
EOF
}

fatal() {
    echo "stegoeggo installer: $*" >&2
    exit 1
}

download() {
    local url="$1"
    local destination="$2"
    local status

    if ! status="$(curl --location --silent --show-error --output "$destination" --write-out '%{http_code}' "$url")"; then
        printf '%s\n' "000"
        return 0
    fi
    printf '%s\n' "$status"
}

sha256() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | awk '{print $1}'
    else
        fatal "neither sha256sum nor shasum is available"
    fi
}

validate_version() {
    [[ "$1" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fatal "invalid version '$1'; expected X.Y.Z"
}

target_for_platform() {
    local os
    local arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os:$arch" in
        Linux:x86_64) printf '%s\n' "x86_64-unknown-linux-gnu" ;;
        Linux:aarch64|Linux:arm64) printf '%s\n' "aarch64-unknown-linux-gnu" ;;
        Darwin:x86_64) printf '%s\n' "x86_64-apple-darwin" ;;
        Darwin:arm64|Darwin:aarch64) printf '%s\n' "aarch64-apple-darwin" ;;
        *) return 1 ;;
    esac
}

install_destination() {
    if [[ "$(id -u)" == "0" ]]; then
        printf '%s\n' "/usr/local/bin"
    else
        [[ -n "${HOME:-}" ]] || fatal 'HOME is not set for a user-local install'
        printf '%s\n' "$HOME/.local/bin"
    fi
}

cargo_fallback() {
    command -v cargo >/dev/null 2>&1 || fatal "no binary is published for this target and Cargo is not installed"
    echo "No compatible prebuilt binary was found; falling back to Cargo." >&2
    if [[ -n "${REQUESTED_VERSION:-}" ]]; then
        cargo install stegoeggo-cli --locked --version "$REQUESTED_VERSION"
    else
        cargo install stegoeggo-cli --locked
    fi
}

main() {
    local requested_version=""
    local target=""
    local asset=""
    local base_url=""
    local binary_path=""
    local checksum_path=""
    local status=""
    local expected_hash=""
    local actual_hash=""
    local candidate_identity=""
    local candidate_version=""
    local destination=""

    while (($# > 0)); do
        case "$1" in
            --help|-h)
                usage
                return 0
                ;;
            --version)
                (($# >= 2)) || fatal "--version requires X.Y.Z"
                requested_version="$2"
                shift 2
                ;;
            --version=*)
                requested_version="${1#--version=}"
                shift
                ;;
            *) fatal "unknown argument '$1'" ;;
        esac
    done

    if [[ -n "$requested_version" ]]; then
        validate_version "$requested_version"
    fi
    REQUESTED_VERSION="$requested_version"

    if ! target="$(target_for_platform)"; then
        cargo_fallback
        return 0
    fi
    asset="stegoeggo-$target"
    [[ "$target" == *-pc-windows-msvc ]] && asset+=".exe"
    if [[ -n "$requested_version" ]]; then
        base_url="$REPOSITORY_RELEASES_URL/download/v$requested_version"
    else
        base_url="$REPOSITORY_RELEASES_URL/latest/download"
    fi

    temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/stegoeggo-install.XXXXXX")"
    trap 'rm -rf "$temp_dir"' EXIT
    binary_path="$temp_dir/$asset"
    checksum_path="$temp_dir/$asset.sha256"

    status="$(download "$base_url/$asset" "$binary_path")"
    if [[ "$status" == "404" ]]; then
        cargo_fallback
        return 0
    fi
    [[ "$status" =~ ^2[0-9][0-9]$ ]] || fatal "failed to download binary asset (HTTP $status)"

    status="$(download "$base_url/$asset.sha256" "$checksum_path")"
    [[ "$status" =~ ^2[0-9][0-9]$ ]] || fatal "failed to download checksum sidecar (HTTP $status)"
    expected_hash="$(awk 'NF {print tolower($1); exit}' "$checksum_path")"
    [[ "$expected_hash" =~ ^[0-9a-f]{64}$ ]] || fatal "checksum sidecar does not contain a SHA-256 digest"
    actual_hash="$(sha256 "$binary_path")"
    [[ "$actual_hash" == "$expected_hash" ]] || fatal "checksum verification failed"

    chmod 0755 "$binary_path"
    if ! candidate_identity="$("$binary_path" version)"; then
        fatal "candidate version command failed"
    fi
    [[ "$candidate_identity" =~ ^stegoeggo\ ([0-9]+\.[0-9]+\.[0-9]+)$ ]] || fatal "candidate version identity is invalid: $candidate_identity"
    candidate_version="${BASH_REMATCH[1]}"
    if [[ -n "$requested_version" && "$candidate_version" != "$requested_version" ]]; then
        fatal "candidate version is $candidate_version, expected $requested_version"
    fi

    destination="$(install_destination)"
    mkdir -p "$destination"
    install -m 0755 "$binary_path" "$destination/stegoeggo"
    echo "Installed stegoeggo $candidate_version to $destination/stegoeggo"
    case ":${PATH:-}:" in
        *":$destination:"*) ;;
        *) echo "Warning: $destination is not on PATH; add it for direct use." >&2 ;;
    esac
}

if [[ "${BASH_SOURCE[0]:-$0}" == "$0" ]]; then
    main "$@"
fi
