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
anchor = '      - name: decide whether this exact snapshot advances the channel\n'
block = workflow.split(anchor, 1)[1].split('        run: |\n', 1)[1]
block = block.split('\n\n  archive:', 1)[0]
block = block.replace('${{ github.event.repository.fork }}', 'false')
pathlib.Path(sys.argv[1], 'prepare.sh').write_text(block, encoding='utf-8')
PY

cat > "$root/bin/gh" <<'SH'
#!/bin/sh
set -eu
[ "$1" = api ] || exit 91
case "$2" in
    *'/releases?per_page=100') cat "$MOCK_RELEASES" ;;
    *'/actions/workflows/ci.yml/runs?'*) cat "$MOCK_CI" ;;
    *'/compare/'*) printf '{"status":"%s"}\n' "$MOCK_STANDING" ;;
    *) printf 'unexpected gh api call: %s\n' "$2" >&2; exit 92 ;;
esac
SH
chmod +x "$root/bin/gh"

# A minimal stand-in for only the jq queries prepare uses. GitHub runners
# supply jq; this also lets the test run on a machine without installing it.
cat > "$root/bin/jq" <<'PY'
#!/usr/bin/env python
import json, re, sys
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

printf '%s\n' 'nightly prepare scenarios passed'
