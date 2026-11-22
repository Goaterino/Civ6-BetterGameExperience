#!/bin/sh
# BetterGameExperience installer for Linux (Steam / Proton).
#
#   curl -fsSL https://github.com/Goaterino/Civ6-BetterGameExperience/releases/latest/download/install.sh | sh
#
# Downloads the static Linux binary to a temporary directory and runs its `install`
# command. The binary copies itself to ${XDG_DATA_HOME:-~/.local/share}/BetterGameExperience
# and verifies the signed manifest and every file it installs; this script does nothing
# security-relevant beyond fetching it over HTTPS. The temporary download is deleted afterwards.
#
# Extra arguments are passed to the binary:   sh install.sh --game-dir /path/to/game
# Uninstall (removes the proxies from the game, then the installer's own files):
#   curl -fsSL https://github.com/Goaterino/Civ6-BetterGameExperience/releases/latest/download/install.sh | sh -s -- uninstall
# Environment:
#   BGE_CHANNEL=prerelease   install from the pre-release channel instead of the latest release.
#                            The channel is saved for in-game updates; running the script without
#                            it switches back to the release channel.
set -eu

REPO="Goaterino/Civ6-BetterGameExperience"
ASSET="BetterGameExperience-linux-x86_64"

command=install
if [ "${1:-}" = "uninstall" ]; then
    command=uninstall
    shift
fi

die() {
    printf 'install.sh: %s\n' "$*" >&2
    exit 1
}

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

# Reads the GitHub API release listing on stdin and prints the download URL of $ASSET from the
# published release with the highest version number, pre-releases included. Same choice as the
# installer (version_key in updater_linux/src/release.rs) and the startup proxy's update check
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
chmod 700 "$bin"

# Not exec: the trap must still remove the temporary directory.
# Always set: a channel saved by an earlier run would otherwise override this one.
if [ "$command" = "install" ]; then
    "$bin" channel "$channel" >/dev/null
fi
"$bin" "$command" "$@"
