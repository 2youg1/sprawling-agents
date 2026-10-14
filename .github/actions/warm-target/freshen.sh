#!/usr/bin/env bash
# freshen.sh record <manifest>  write the content hash of every build input of this tree
# freshen.sh apply <manifest>   age every input whose content the manifest holds, so cargo's
#                               mtime comparison reads it as unchanged since the target was built
#
# Cargo calls a workspace unit fresh when no input it read is newer than the
# unit's own output. A checkout writes every file now, so a restored target
# reads as stale throughout. `apply` gives every input whose bytes equal the
# recorded ones a time older than any build, and leaves everything else at
# now; cargo's other half of the decision, the fingerprint of compiler,
# flags, features, environment and dependencies, is untouched.
#
# Content is read from disk, not from the index, so an edit not yet staged
# is still an edit. Directory times are part of the answer: cargo walks a
# `rerun-if-changed` directory for its newest entry, so every directory
# ages and every directory above an added, removed or edited path is
# renewed, which keeps a removed file a change.
set -euo pipefail
mode=$1
manifest=$2
# Untracked inputs: the bundle crates/sprawling/build.rs embeds, built from
# client/ by the step before this one, and identical byte for byte when
# client/ is.
extra=crates/sprawling/web-dist
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

inputs() {
    { git ls-files; if [ -d "$extra" ]; then find "$extra" -type f; fi; } |
        while IFS= read -r path; do if [ -f "$path" ]; then printf '%s\n' "$path"; fi; done > "$work/paths"
    git hash-object --stdin-paths < "$work/paths" | paste -d' ' - "$work/paths"
}

ancestors() {
    sed -n 's|/[^/]*$||p' | awk -F/ '{p=""; for (i=1;i<=NF;i++){p=(i==1?$1:p"/"$i); print p}}' | LC_ALL=C sort -u
}

case $mode in
record)
    inputs | LC_ALL=C sort > "$manifest" ;;
apply)
    inputs | LC_ALL=C sort > "$work/now"
    LC_ALL=C sort "$manifest" > "$work/then"
    LC_ALL=C comm -12 "$work/then" "$work/now" | cut -d' ' -f2- > "$work/same"
    LC_ALL=C comm -3 "$work/then" "$work/now" | sed 's/^\t//' | cut -d' ' -f2- | LC_ALL=C sort -u > "$work/moved"
    old='2001-01-01 00:00:00'
    tr '\n' '\0' < "$work/same" | xargs -0 -r touch -m -d "$old"
    cut -d' ' -f2- "$work/now" | ancestors | tr '\n' '\0' | xargs -0 -r touch -m -d "$old"
    ancestors < "$work/moved" | while IFS= read -r dir; do if [ -d "$dir" ]; then printf '%s\0' "$dir"; fi; done | xargs -0 -r touch -m
    tr '\n' '\0' < "$work/moved" | xargs -0 -r touch -m --no-create
    echo "freshen: $(wc -l < "$work/same") inputs as the warm target saw them; $(wc -l < "$work/moved") added, removed or edited" ;;
*)
    echo "freshen.sh: mode is record or apply, not '$mode'" >&2
    exit 2 ;;
esac
