#!/bin/sh
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
# Copyright (c) 2026 2youg1 and the sprawling contributors
#
# One command that leaves `sprawling` on your PATH:
#
#     curl -fsSL https://raw.githubusercontent.com/2youg1/sprawling/main/install.sh | sh
#
# This script fetches and unpacks. **Where the binary goes, and what
# happens to PATH, is decided by `sprawling install`** - the binary's own
# installer, which runs at the end. Two installers choosing a directory
# would be two authorities for one rule, and the one a person can re-run
# later is the one that has to win.
#
# Three facts about this repository's releases that the obvious script
# gets wrong:
#
#   * Every release so far is a pre-release, and GitHub's `releases/latest`
#     endpoint excludes those - it answers 404 here. The list endpoint is
#     asked for its newest entry instead.
#   * The tag and the archive carry different versions: tag
#     `v0.0.3-Pre-alpha-260903` ships `sprawling-0.0.3-macos-aarch64.zip`.
#     An asset is chosen by its platform suffix, never by a name built out
#     of the tag.
#   * Only the platforms the release workflow builds are installable. A
#     platform with no archive is reported with the list the release does
#     carry, rather than guessed at.
#
# `sprawling` is the short name GitHub still resolves to this repository,
# whose canonical path is `2youg1/sprawling-agents`. Set SPRAWLING_REPO if
# that ever stops being true.

set -eu

REPO="${SPRAWLING_REPO:-2youg1/sprawling}"
API="https://api.github.com/repos/${REPO}/releases"
RELEASES="https://github.com/${REPO}/releases"

say() { printf '%s\n' "$*"; }
die() { printf 'sprawling install: %s\n' "$*" >&2; exit 1; }

need() {
    command -v "$1" >/dev/null 2>&1 || die "$1 is required and was not found"
}

# The system and the architecture, in the words the release archives use.
#
# One machine can be spelled more than one way, and Linux is. An archive
# built for the runner it ran on is named after that runner - `macos-
# aarch64`, `windows-x86_64` - while an archive built for a target named
# on the command line carries that target's whole triple, so the static
# Linux one is `x86_64-unknown-linux-musl`. The triple is in that name on
# purpose: a gnu build may join it later, and two archives differing only
# in their libc would otherwise claim one name.
#
# So a platform names every suffix its release may carry, most specific
# first, and the first one actually present wins. A list rather than a
# rule, because the naming is a fact about how each row is built rather
# than something a reader could derive.
#
# The Linux row builds `x86_64-unknown-linux-musl`, so the first suffix
# below is the one a release carries today; the second is kept for a gnu
# build that may join it later. A release cut before that row existed
# matches neither, and a Linux run against one of those is told what the
# release does carry, which is the same answer it would give for any
# platform nobody builds for.
platform() {
    system=$(uname -s)
    machine=$(uname -m)
    case "$system" in
        Darwin) os=macos ;;
        Linux) os=linux ;;
        *) die "no archive is built for $system; build from source with \`just dist\`" ;;
    esac
    case "$machine" in
        x86_64 | amd64) arch=x86_64 ;;
        arm64 | aarch64) arch=aarch64 ;;
        *) die "no archive is built for $machine; build from source with \`just dist\`" ;;
    esac
    case "${os}-${arch}" in
        linux-x86_64) suffixes="-x86_64-unknown-linux-musl.zip -linux-x86_64.zip" ;;
        *) suffixes="-${os}-${arch}.zip" ;;
    esac
}

# The newest release, or the one SPRAWLING_VERSION names. A pinned tag is
# what makes an install reproducible, so it is asked for by tag rather
# than searched for in the list.
release_json() {
    if [ -n "${SPRAWLING_VERSION:-}" ]; then
        curl -fsSL "${API}/tags/${SPRAWLING_VERSION}" ||
            die "no release tagged ${SPRAWLING_VERSION}; the tags are listed at ${RELEASES}"
    else
        curl -fsSL "${API}?per_page=1" ||
            die "cannot reach ${API}; check the network, or download from ${RELEASES}"
    fi
}

# One asset's fields, kept together: the download URL and the digest that
# covers it land in the same chunk, so a checksum can never be read off a
# different asset than the bytes it is checked against.
#
# The response is flattened before it is split. GitHub sends this JSON
# pretty-printed, so every field already sits on its own line, and
# splitting on `{` alone leaves grep matching one line at a time - which
# finds the URL, loses the digest beside it, and reports the release as
# publishing no checksum when it publishes one.
#
# `-e` is load-bearing too: the suffix begins with `-`, and grep reads a
# bare `-macos-aarch64.zip` as a run of options rather than as a pattern.
asset_chunk() {
    printf '%s' "$1" | tr -d '\n\r' | tr '{' '\n' | grep -F -e "$2" |
        grep -F -e 'browser_download_url' | head -n 1
}

# `|` delimits the sed expressions here because every value being read is
# a URL or a path: with `/` the pattern would need escaping in the one
# place where an escaping mistake reads as a missing asset.
field() {
    printf '%s\n' "$1" | tr ',' '\n' | sed -n "s|.*\"$2\": *\"$3\".*|\\1|p" | head -n 1
}

# The archive's size in bytes, which the release API reports beside the
# URL. `field` reads quoted values; this one is a bare number.
asset_size() {
    printf '%s\n' "$1" | tr ',' '\n' |
        sed -n 's|.*"size": *\([0-9][0-9]*\).*|\1|p' | head -n 1
}

# Bytes as megabytes with one decimal, rounded. Shell arithmetic rather
# than awk or bc, so one printed number adds no tool this script would
# then have to `need`.
megabytes() {
    tenths=$(( ($1 * 10 + 524288) / 1048576 ))
    printf '%s.%s' "$(( tenths / 10 ))" "$(( tenths % 10 ))"
}

# Every archive this release carries, for the message a person gets when
# theirs is not among them.
offered() {
    printf '%s\n' "$1" | tr ',' '\n' |
        sed -n 's|.*"browser_download_url": *"[^"]*/\([^"]*\.zip\)".*|  \1|p'
}

checksum() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | cut -d' ' -f1
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | cut -d' ' -f1
    elif command -v openssl >/dev/null 2>&1; then
        openssl dgst -sha256 "$1" | sed 's/.*= *//'
    else
        die "no sha256 tool found (sha256sum, shasum or openssl); \
an unverified download is not installed here"
    fi
}

unpack() {
    if command -v unzip >/dev/null 2>&1; then
        unzip -q "$1" -d "$2"
    elif command -v bsdtar >/dev/null 2>&1; then
        bsdtar -x -f "$1" -C "$2"
    elif tar -x -f "$1" -C "$2" 2>/dev/null; then
        :
    else
        die "no unzip, bsdtar, or tar that reads zip; install unzip and run this again"
    fi
}

need curl
need uname
need sed
need tr
platform

say "sprawling: asking ${REPO} what it has for ${os}-${arch}"
json=$(release_json)
tag=$(field "$json" 'tag_name' '\([^"]*\)')
[ -n "$tag" ] || die "the release list came back without a tag; see ${RELEASES}"

chunk=""
for suffix in $suffixes; do
    chunk=$(asset_chunk "$json" "$suffix" || true)
    [ -n "$chunk" ] && break
done
if [ -z "$chunk" ]; then
    say "sprawling: ${tag} carries no archive ending ${suffixes}. What it does carry:"
    offered "$json"
    die "download one of those from ${RELEASES}, or build from source with \`just dist\`"
fi

url=$(field "$chunk" 'browser_download_url' '\([^"]*\)')
digest=$(field "$chunk" 'digest' 'sha256:\([0-9a-f]*\)')
[ -n "$url" ] || die "the asset came back without a download URL; see ${RELEASES}"
[ -n "$digest" ] || die "\
${tag} publishes no sha256 for that archive, so the bytes cannot be checked. \
Download it yourself from ${RELEASES} if you accept that."

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT INT TERM
archive="${tmp}/archive.zip"

size=$(asset_size "$chunk")
if [ -n "$size" ]; then
    say "sprawling ${tag} | ${os}-${arch} | $(megabytes "$size") MB"
else
    say "sprawling ${tag} | ${os}-${arch}"
fi

# curl draws its bar on stderr, which is a terminal for the usual
# `curl ... | sh` run. Where it is not, the bar would fill a log with
# control characters, so there the download reports only failures.
if [ -t 2 ]; then meter='--progress-bar'; else meter='-sS'; fi
# shellcheck disable=SC2086 # one option, deliberately unquoted
curl -fL $meter "$url" -o "$archive" || die "download failed: $url"

got=$(checksum "$archive")
[ "$got" = "$digest" ] || die "\
the archive does not match the sha256 the release publishes.
  expected ${digest}
  received ${got}
Nothing was installed."
say "sprawling: verified sha256 ${digest}"

unpack "$archive" "$tmp"
binary=$(find "$tmp" -type f -name sprawling | head -n 1)
[ -n "$binary" ] || die "the archive holds no file named sprawling"
chmod +x "$binary"

# What was installed, said by the thing that was installed: the first
# line of `status` is the binary's own version.
reported=$("$binary" status 2>/dev/null | head -n 1 || true)
if [ -n "$reported" ]; then
    say "sprawling: ${reported}"
fi

# The binary places itself. Everything this script knows about
# directories and PATH ends here, and what it prints below comes from the
# installer a person can run again by hand.
"$binary" install

say "Next: run \`sprawling up\` to raise a city and open it in your browser."
say "It needs a model to call before it can do anything - \`sprawling help\` lists the rest."
