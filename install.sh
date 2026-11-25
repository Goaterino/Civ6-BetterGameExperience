#!/bin/sh
# BetterGameExperience installer for Linux (Steam / Proton).
#
#   curl -fsSL https://github.com/Goaterino/Civ6-BetterGameExperience/releases/latest/download/install.sh | sh
#
# Downloads the static Linux binary and its signature (`<binary>.sig`, ed25519, made by the release
# workflow with the key that also signs the manifest) to a temporary directory, checks the signature
# with openssl, then runs the binary's `install` command. The binary copies itself to
# ${XDG_DATA_HOME:-~/.local/share}/BetterGameExperience and verifies the signed manifest and every
# file it installs. The temporary download is deleted afterwards.
#
# Extra arguments are passed to the binary:   sh install.sh --game-dir /path/to/game
# Uninstall (removes the proxies from the game, then the installer's own files):
#   curl -fsSL https://github.com/Goaterino/Civ6-BetterGameExperience/releases/latest/download/install.sh | sh -s -- uninstall
# Check a binary already downloaded, against `<file>.sig` next to it:
#   sh install.sh verify BetterGameExperience-linux-x86_64
# Environment:
#   BGE_CHANNEL=prerelease   install from the pre-release channel instead of the latest release.
#                            The channel is saved for in-game updates; running the script without
#                            it switches back to the release channel.
set -eu

REPO="Goaterino/Civ6-BetterGameExperience"
ASSET="BetterGameExperience-linux-x86_64"

# Public key of the release signatures: release/public_key.txt in PEM form
# (`python release/manifest.py public-pem`; the CI checks that both match).
PUBLIC_KEY_PEM="-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEAxQxNrSEW5GiVuTN+x+EAjZrnHBTHtgvwpXqJlU+jdhw=
-----END PUBLIC KEY-----"

die() {
    printf 'install.sh: %s\n' "$*" >&2
    exit 1
}

# verify_signature <file> <base64 signature file> <work dir>: ed25519 over the file's exact bytes.
# OpenSSL 3 (`pkeyutl -rawin`); 1.1.1 cannot verify ed25519 from the command line.
verify_signature() {
    command -v openssl >/dev/null 2>&1 || die "openssl is required to verify the installer's signature."
    printf '%s\n' "$PUBLIC_KEY_PEM" >"$3/key.pem"
    openssl base64 -d -A -in "$2" -out "$3/sig.bin" 2>/dev/null || die "unreadable signature: $2"
    openssl pkeyutl -verify -pubin -inkey "$3/key.pem" -rawin -in "$1" -sigfile "$3/sig.bin" >/dev/null 2>&1 ||
        die "invalid signature for $1: the file was not published by BetterGameExperience (or openssl is older than 3.0)."
}

command=install
case "${1:-}" in
    uninstall)
        command=uninstall
        shift
        ;;
    verify)
        [ $# -eq 2 ] || die "usage: sh install.sh verify <file>"
        work=$(mktemp -d "${TMPDIR:-/tmp}/bge-verify.XXXXXX")
        trap 'rm -rf "$work"' EXIT HUP INT TERM
        verify_signature "$2" "$2.sig" "$work"
        printf 'Valid signature: %s\n' "$2"
        exit 0
        ;;
esac

[ "$(uname -s)" = "Linux" ] || die "Linux only (found $(uname -s))."
case "$(uname -m)" in
    x86_64 | amd64) ;;
    *) die "x86_64 only (found $(uname -m))." ;;
esac

if command -v curl >/dev/null 2>&1; then
    fetch() { curl -fsSL --retry 3 "$@"; }
elif command -v wget >/dev/null 2>&1; then
    fetch() { wget -qO- "$@"; }
else
    die "curl or wget is required."
fi
command -v openssl >/dev/null 2>&1 || die "openssl is required to verify the installer's signature."

# Reads the GitHub API release listing on stdin and prints the download URL of $ASSET from the
# published release with the highest version number, pre-releases included. Same choice as the
# installer (version_key in updater/src/release.rs) and the startup proxy's update check
# (parse_releases in startup_proxy/src/maj.rs) on the pre-release channel. GitHub sorts the list
# by date, so its first entry can be older. Order: X.Y.Z numerically, then a pre-release
# (vX.Y.Z-rc.1) before the final release of the same number, then suffixes as text.
newest_asset_url() {
    sed -n 's/.*"browser_download_url": *"\([^"]*\/download\/v[^"/]*\/'"$ASSET"'\)".*/\1/p' |
        awk -v asset="$ASSET" '{
            tag = $0
            sub("/" asset "$", "", tag)
            sub(/.*\/download\/v/, "", tag)
            core = tag
            suffix = ""
            i = index(tag, "-")
            if (i) { core = substr(tag, 1, i - 1); suffix = substr(tag, i + 1) }
            if (core !~ /^[0-9]+\.[0-9]+\.[0-9]+$/) next
            split(core, v, ".")
            printf "%010d %010d %010d %d %s %s\n", v[1], v[2], v[3], suffix == "", suffix, $0
        }' | LC_ALL=C sort | tail -n 1 | sed 's/.* //'
}

channel="${BGE_CHANNEL:-release}"
case "$channel" in
    release) url="https://github.com/$REPO/releases/latest/download/$ASSET" ;;
    prerelease)
        url=$(fetch "https://api.github.com/repos/$REPO/releases?per_page=30" | newest_asset_url)
        [ -n "$url" ] || die "no pre-release found with a Linux binary."
        ;;
    *) die "BGE_CHANNEL must be 'release' or 'prerelease'." ;;
esac

tmp=$(mktemp -d "${TMPDIR:-/tmp}/bge-install.XXXXXX")
trap 'rm -rf "$tmp"' EXIT HUP INT TERM
bin="$tmp/BetterGameExperience"

printf 'Downloading %s\n' "$url"
fetch "$url" >"$bin" || die "download failed."
[ -s "$bin" ] || die "downloaded file is empty."
fetch "$url.sig" >"$bin.sig" || die "download of the signature failed."
# Before chmod: an unsigned file is never made executable.
verify_signature "$bin" "$bin.sig" "$tmp"
chmod 700 "$bin"

# Not exec: the trap must still remove the temporary directory.
# Always set: a channel saved by an earlier run would otherwise override this one.
if [ "$command" = "install" ]; then
    "$bin" channel "$channel" >/dev/null
fi
"$bin" "$command" "$@"
