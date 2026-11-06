#!/bin/sh
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
# Copyright (c) 2026 2youg1 and the sprawling contributors
#
# Installs this run's own archive the way a person does - `install.sh`
# asking a release list - with the list and the archive served from
# 127.0.0.1, then runs the binary the installer placed.
#
#     sh .github/install-loopback.sh <directory holding one archive>
#
# Green means three things at once: install.sh exited 0, the file it
# placed is byte for byte the `sprawling` inside this archive, and that
# file answers `status` with its version line. Comparing bytes rather
# than the version line is what tells this build apart from a published
# one of the same version (`crates/sprawling/Spec.lean` §8-94).

set -eu

die() { printf 'install-loopback: %s\n' "$*" >&2; exit 1; }

[ $# -eq 1 ] || die "usage: install-loopback.sh <directory holding one archive>"
set -- "$1"/*.zip
[ $# -eq 1 ] && [ -f "$1" ] || die "expected exactly one .zip in the directory, found: $*"
archive=$1
installer="$(cd "$(dirname "$0")/.." && pwd)/install.sh"

root=$(mktemp -d)
server=""
cleanup() {
    [ -n "$server" ] && kill "$server" 2>/dev/null
    rm -rf "$root"
}
trap cleanup EXIT INT TERM
mkdir "$root/www" "$root/sandbox" "$root/unpacked"
cp "$archive" "$root/www/"
name=$(basename "$archive")

# Port 0: the kernel picks a free one, so parallel runs never collide.
python3 -u -m http.server 0 --bind 127.0.0.1 --directory "$root/www" \
    >"$root/server.log" 2>&1 &
server=$!
port=""
for _ in $(seq 1 600); do
    port=$(sed -n 's/^Serving HTTP on 127\.0\.0\.1 port \([0-9][0-9]*\).*/\1/p' "$root/server.log")
    [ -n "$port" ] && break
    kill -0 "$server" 2>/dev/null || die "the loopback server exited: $(cat "$root/server.log")"
    sleep 0.1
done
[ -n "$port" ] || die "the loopback server named no port within 60 s"
origin="http://127.0.0.1:${port}"

# The list in the shape GitHub's API answers with: an array of releases,
# each asset carrying its size, its digest and its download URL.
mkdir -p "$root/www/releases"
python3 - "$root/www/$name" "$origin" "${LOOPBACK_RELEASE_TAG:-v0.0.0-Alpha-260101}" "$root/www" \
    >"$root/www/releases/index.html" <<'PY'
import hashlib, json, os, sys
path, origin, tag, www = sys.argv[1:5]
with open(path, "rb") as f:
    digest = hashlib.sha256(f.read()).hexdigest()
name = os.path.basename(path)
release = {
    "tag_name": tag,
    "published_at": "2026-01-01T00:00:00Z",
    "draft": False,
    "assets": [{
        "name": name,
        "size": os.path.getsize(path),
        "digest": "sha256:" + digest,
        "browser_download_url": origin + "/" + name,
    }],
}
json.dump([release], sys.stdout, indent=2)
os.makedirs(os.path.join(www, "releases", "tags", tag), exist_ok=True)
with open(os.path.join(www, "releases", "tags", tag, "index.html"), "w", encoding="utf-8") as f:
    json.dump(release, f, indent=2)
PY

# Every request that would leave this machine goes to a port nothing
# listens on, so an installer that ignored SPRAWLING_API fails here
# instead of quietly installing the published release.
HOME="$root/sandbox" SPRAWLING_API="${origin}/releases" \
    https_proxy=http://127.0.0.1:9 http_proxy=http://127.0.0.1:9 \
    no_proxy=127.0.0.1 NO_PROXY=127.0.0.1 \
    sh "$installer" || die "install.sh failed against ${origin}"

installed="$root/sandbox/.local/bin/sprawling"
[ -f "$installed" ] || die "install.sh placed nothing at ${installed}"
unzip -q "$archive" -d "$root/unpacked"
built=$(find "$root/unpacked" -type f -name sprawling | head -n 1)
[ -n "$built" ] || die "the archive holds no file named sprawling"
cmp -s "$built" "$installed" || die "the installed binary is not the one in ${name}"

version=$("$installed" status | head -n 1)
case "$version" in
    "sprawling "*) printf 'install-loopback: %s installed and answering: %s\n' "$name" "$version" ;;
    *) die "the installed binary did not name itself: ${version}" ;;
esac
