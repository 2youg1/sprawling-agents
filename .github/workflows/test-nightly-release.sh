#!/bin/sh
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
# Copyright (c) 2026 2youg1 and the sprawling contributors
#
# Drives nightly-release.yml's prepare step with a mocked GitHub API.
# No network, tag, release, issue or repository write is possible here.

set -eu

root=$(mktemp -d)
trap 'rm -rf "$root"' EXIT INT TERM
mkdir -p "$root/bin"
source_sha=$(git rev-parse HEAD)
other=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa

python - "$root" <<'PY'
import pathlib, sys
workflow = pathlib.Path(".github/workflows/nightly-release.yml").read_text(encoding="utf-8")


def step(name, stop):
    anchor = f'      - name: {name}\n'
    block = workflow.split(anchor, 1)[1].split('        run: |\n', 1)[1]
    return block.split(stop, 1)[0] if stop else block


prepare = step('decide whether this exact snapshot advances the channel', '\n\n  archive:')
prepare = prepare.replace('${{ github.event.repository.fork }}', 'false')
pathlib.Path(sys.argv[1], 'prepare.sh').write_text(prepare, encoding='utf-8')
# The retention step is the last one in the file, so nothing bounds it.
retain = step('retain the newest nightlies', None)
pathlib.Path(sys.argv[1], 'retain.sh').write_text(retain, encoding='utf-8')
PY

# GitHub validates the complete reusable-workflow graph before job conditions
# can skip its publication jobs. The call needs their grants as a ceiling,
# while every job that can actually build or advise stays read-only.
python - <<'PY'
import pathlib, re


def job(path, name):
    text = pathlib.Path(path).read_text(encoding='utf-8')
    match = re.search(
        rf'(?ms)^  {re.escape(name)}:\n(.*?)(?=^  [a-z][a-z0-9_-]*:\n|\Z)',
        text,
    )
    if match is None:
        raise AssertionError(f'{path} has no {name} job')
    return match.group(1)


def permissions(block):
    lines = block.splitlines()
    try:
        start = lines.index('    permissions:')
    except ValueError as error:
        raise AssertionError('job has no explicit permissions') from error
    grants = {}
    for line in lines[start + 1:]:
        if not line.startswith('      '):
            break
        key, value = line.strip().split(':', 1)
        grants[key] = value.strip()
    return grants


caller = permissions(job('.github/workflows/nightly-release.yml', 'archive'))
expected = {'contents': 'write', 'id-token': 'write', 'attestations': 'write'}
if caller != expected:
    raise AssertionError(f'nightly call ceiling is {caller}, expected {expected}')
for name in ('verify', 'advisories', 'archive'):
    block = job('.github/workflows/release.yml', name)
    grants = permissions(block)
    if grants != {'contents': 'read'}:
        raise AssertionError(f'release {name} can inherit publication grants: {grants}')
print('nightly reusable-call permissions passed')
PY

cat > "$root/bin/gh" <<'SH'
#!/bin/sh
set -eu
case "$1" in
    api)
        case "$2" in
            *'/releases?per_page=100') cat "$MOCK_RELEASES" ;;
            *'/actions/workflows/ci.yml/runs?'*) cat "$MOCK_CI" ;;
            *'/git/ref/tags/'*)
                test -n "${MOCK_TAG_REF:-}" || { printf 'not found\n' >&2; exit 1; }
                cat "$MOCK_TAG_REF"
                ;;
            *'/git/tags/'*)
                test -n "${MOCK_TAG_OBJECT:-}" || { printf 'not found\n' >&2; exit 1; }
                cat "$MOCK_TAG_OBJECT"
                ;;
            *'/compare/'*)
                test "${MOCK_STANDING:-}" != error || { printf 'not found\n' >&2; exit 1; }
                printf '{"status":"%s"}\n' "$MOCK_STANDING"
                ;;
            *) printf 'unexpected gh api call: %s\n' "$2" >&2; exit 92 ;;
        esac
        ;;
    release)
        test "$2" = delete || exit 93
        printf '%s\n' "$3" >> "$MOCK_DELETIONS"
        ;;
    *) exit 91 ;;
esac
SH
chmod +x "$root/bin/gh"

# A minimal stand-in for only the jq queries prepare uses. GitHub runners
# supply jq; this also lets the test run on a machine without installing it.
cat > "$root/bin/jq" <<'PY'
#!/usr/bin/env python
import json, re, sys

# jq writes LF on every platform; this runs on Windows too, where the
# default text stream would hand the shell a carriage return before each
# newline and the step would delete a tag with a trailing `\r`.
sys.stdout.reconfigure(newline="\n")

args = sys.argv[1:]
query = args[-1]
data = json.load(sys.stdin)
if 'length > 0' in query:
    rows = [row for row in data['workflow_runs']
            if row.get('status') == 'completed' and row.get('conclusion') == 'success'
            and row.get('event') in ('push', 'workflow_dispatch')]
    sys.exit(0 if rows else 1)
elif query == 'length':
    print(len(data))
elif query == '.[0] // empty':
    if data:
        print(json.dumps(data[0], separators=(',', ':')))
elif query == '.tag_name':
    print(data['tag_name'])
elif query == '.status':
    print(data['status'])
elif query == '.target_commitish':
    print(data['target_commitish'])
elif query == '.[1] // empty':
    if len(data) > 1:
        print(json.dumps(data[1], separators=(',', ':')))
elif query == '.[2:][] | .tag_name':
    for row in data[2:]:
        print(row['tag_name'])
elif query == '.object.type':
    print(data['object']['type'])
elif query == '.object.sha':
    print(data['object']['sha'])
elif query.startswith('[.[]') and 'test("^v[0-9]")' in query:
    rows = [row for row in data if not row.get('draft', False)
            and re.match(r'v[0-9]', row.get('tag_name', ''))]
    rows.sort(key=lambda row: row['published_at'], reverse=True)
    if rows:
        print(json.dumps(rows[0], separators=(',', ':')))
elif query.startswith('[.[]'):
    rows = [row for row in data if not row.get('draft', False)
            and row.get('tag_name', '').startswith('nightly-')
            and re.fullmatch(r'[0-9a-f]{40}', row.get('target_commitish', ''))]
    rows.sort(key=lambda row: row['published_at'], reverse=True)
    print(json.dumps(rows, separators=(',', ':')))
else:
    sys.exit('unsupported jq mock query: ' + query)
PY
chmod +x "$root/bin/jq"

export MOCK_CI="$root/ci.json"
printf '%s\n' '{"workflow_runs":[{"status":"completed","conclusion":"success","event":"push"}]}' > "$MOCK_CI"

run_case() {
    name=$1
    event=$2
    requested=$3
    standing=$4
    published=$5
    expected_build=$6
    expected_publish=$7
    expected_cleanup=$8

    export MOCK_RELEASES="$root/releases-$name.json"
    export MOCK_STANDING=$standing
    if [ "$name" = pending ]; then
        printf '%s\n' '{"workflow_runs":[]}' > "$MOCK_CI"
    else
        printf '%s\n' '{"workflow_runs":[{"status":"completed","conclusion":"success","event":"push"}]}' > "$MOCK_CI"
    fi
    python - "$source_sha" "$other" "$published" "$MOCK_RELEASES" <<'PY'
import json, sys
source, other, published, output = sys.argv[1:5]
nightlies = []
for index, sha in enumerate(published.split(',')):
    if not sha:
        continue
    nightlies.append({
        "tag_name": "nightly-26110" + str(index + 1) + "-" + sha,
        "draft": False,
        "published_at": "2026-11-0" + str(index + 1) + "T00:00:00Z",
        "target_commitish": sha,
    })
with open(output, "w", encoding="utf-8") as f:
    json.dump(nightlies, f)
PY

    export GITHUB_EVENT_NAME=$event
    export GITHUB_REF=refs/heads/main
    export GITHUB_SHA=$source_sha
    export GITHUB_REPOSITORY=example/example
    export GITHUB_OUTPUT="$root/output-$name"
    export GITHUB_STEP_SUMMARY="$root/summary-$name"
    export INPUT_PUBLISH=$requested
    : > "$GITHUB_OUTPUT"
    : > "$GITHUB_STEP_SUMMARY"
    # The prepare step runs with `shell: bash` in the workflow, so the
    # extracted block runs under bash here too: a here-string is not POSIX.
    PATH="$root/bin:$PATH" GH_TOKEN=test bash "$root/prepare.sh"

    check() {
        key=$1
        wanted=$2
        grep -q "^$key=$wanted$" "$GITHUB_OUTPUT" || {
            printf '%s: expected %s=%s, got:\n' "$name" "$key" "$wanted" >&2
            cat "$GITHUB_OUTPUT" >&2
            exit 1
        }
    }
    check build "$expected_build"
    check publish "$expected_publish"
    check cleanup "$expected_cleanup"
    check source "$source_sha"
    if [ "$expected_build" = true ]; then
        check tag "nightly-$(date -u +%y%m%d)-$source_sha"
        grep -q '^baseline=v' "$GITHUB_OUTPUT"
    fi
}

run_case new schedule '' ahead "$other" true true false
run_case same schedule '' ahead "$source_sha" false true false
run_case pending schedule '' ahead "$other" false false false
run_case stale schedule '' behind "$other" false false false
run_case dry-run workflow_dispatch '' identical "$source_sha" true false false
run_case retention workflow_dispatch true identical "$other,aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaab,$source_sha" false true true

# The retention step decides which published snapshots survive. Each case
# writes the list the step reads, the tag-ref answers it asks for, and the
# deletions it should reach, named by their position in the oldest-first
# list the fixture builder writes.
run_retention() {
    name=$1
    standing=$2
    published=$3
    curated=$4
    ref_kind=$5
    expected=$6

    export MOCK_RELEASES="$root/releases-$name.json"
    export MOCK_STANDING=$standing
    export MOCK_DELETIONS="$root/deletions-$name"
    export MOCK_TAG_REF="$root/tag-ref-$name.json"
    export MOCK_TAG_OBJECT="$root/tag-object-$name.json"
    : > "$MOCK_DELETIONS"
    rm -f "$MOCK_TAG_REF" "$MOCK_TAG_OBJECT"
    case "$ref_kind" in
        commit) printf '{"object":{"type":"commit","sha":"%s"}}\n' cccccccccccccccccccccccccccccccccccccccc > "$MOCK_TAG_REF" ;;
        tag)
            printf '{"object":{"type":"tag","sha":"%s"}}\n' dddddddddddddddddddddddddddddddddddddddd > "$MOCK_TAG_REF"
            printf '{"object":{"sha":"%s"}}\n' cccccccccccccccccccccccccccccccccccccccc > "$MOCK_TAG_OBJECT"
            ;;
    esac
    python - "$published" "$curated" "$MOCK_RELEASES" "$root/tags-$name" <<'PY'
import json, sys
published, curated, output, tags = sys.argv[1:5]
rows = []
names = []
for index, sha in enumerate(x for x in published.split(',') if x):
    tag = "nightly-2611" + str(index + 1) + "-" + sha
    names.append(tag)
    rows.append({
        "tag_name": tag,
        "draft": False,
        "published_at": "2026-11-%02dT00:00:00Z" % (10 + index),
        "target_commitish": sha,
    })
if curated:
    rows.append({
        "tag_name": curated,
        "draft": False,
        "published_at": "2026-11-02T07:33:42Z",
        "target_commitish": "main",
    })
with open(output, "w", encoding="utf-8") as file:
    json.dump(rows, file)
with open(tags, "w", encoding="utf-8") as file:
    file.write("\n".join(names) + ("\n" if names else ""))
PY
    PATH="$root/bin:$PATH" GH_TOKEN=test GITHUB_REPOSITORY=example/example \
        bash "$root/retain.sh"
    : > "$root/expected-$name"
    for index in $expected; do
        sed -n "${index}p" "$root/tags-$name" >> "$root/expected-$name"
    done
    sort "$root/expected-$name" -o "$root/expected-$name"
    sort "$MOCK_DELETIONS" -o "$root/deleted-$name"
    cmp -s "$root/expected-$name" "$root/deleted-$name" || {
        printf '%s: deletions differ from the expected set\n' "$name" >&2
        diff -u "$root/expected-$name" "$root/deleted-$name" >&2 || true
        exit 1
    }
}

sha1=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaab
sha2=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaac
sha3=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaad
release=v0.0.15-Alpha-261102

# Three snapshots and no curated release: the cap deletes the oldest.
run_retention three behind "$sha1,$sha2,$sha3" '' commit 1
# The newest curated release contains the second snapshot's commit.
run_retention superseded ahead "$sha1,$sha2" "$release" commit 1
run_retention identical identical "$sha1,$sha2" "$release" commit 1
run_retention annotated ahead "$sha1,$sha2" "$release" tag 1
# The snapshot carries commits the release lacks: it stays.
run_retention newer behind "$sha1,$sha2" "$release" commit ''
run_retention diverged diverged "$sha1,$sha2" "$release" commit ''
# No curated release, no readable tag ref, or no answer from the compare:
# nothing beyond the cap, because a failed cleanup must not delete.
run_retention uncurated behind "$sha1,$sha2" '' commit ''
run_retention missing-ref ahead "$sha1,$sha2" "$release" missing ''
run_retention failed-compare error "$sha1,$sha2" "$release" commit ''
# The cap and the superseded rule act together, and the newest stays.
run_retention three-superseded ahead "$sha1,$sha2,$sha3" "$release" commit '1 2'

# The advisory parser accepts real advisory-shaped diagnostics, but a
# cargo-deny error without an advisory remains a checker failure.
repo=$(pwd)
python - "$repo" "$root" <<'PY'
import contextlib, io, json, os, pathlib, sys
repo, root = map(pathlib.Path, sys.argv[1:3])
os.chdir(root)
workflow = (repo / '.github/workflows/nightly.yml').read_text(encoding='utf-8')
code = 'import json, sys\n' + workflow.split('          import json, sys\n', 1)[1].split('\n          PY\n', 1)[0]
code = '\n'.join(line[10:] if line.startswith('          ') else line for line in code.splitlines())
advisory = {
    'fields': {
        'advisory': {'id': 'RUSTSEC-TEST-0001', 'package': 'example', 'title': 'example advisory'},
        'graphs': [{'Krate': {'version': '1.0.0'}}],
        'notes': ['Solution: upgrade to 1.0.1'],
    },
    'type': 'diagnostic',
}
(root / 'deny.json').write_text(json.dumps(advisory) + '\n', encoding='utf-8')
out = io.StringIO()
with contextlib.redirect_stdout(out):
    exec(compile(code, 'nightly-advisories', 'exec'), {'__name__': '__main__', 'sys': sys})
(root / 'advisories.tsv').write_text(out.getvalue(), encoding='utf-8')
assert len(out.getvalue().splitlines()) == 1
(root / 'deny.json').write_text(json.dumps({'fields': {'severity': 'error', 'message': 'offline'}, 'type': 'diagnostic'}) + '\n', encoding='utf-8')
try:
    exec(compile(code, 'nightly-advisories', 'exec'), {'__name__': '__main__', 'sys': sys})
except SystemExit as error:
    assert error.code, 'checker failure was swallowed'
else:
    raise AssertionError('a checker failure became a tracked advisory')
print('advisory parser scenarios passed')
PY

printf '%s\n' 'nightly prepare and retention scenarios passed'
