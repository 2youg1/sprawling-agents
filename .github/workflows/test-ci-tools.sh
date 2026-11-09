#!/usr/bin/env bash
# Verify the CI tool cache contract and exercise its staging boundary without
# contacting GitHub or depending on a hosted runner.
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
action="$root/.github/actions/ci-tools"
fail() {
    echo "test-ci-tools: $*" >&2
    exit 1
}
require() {
    grep -F -- "$1" "$2" >/dev/null || fail "$2 lacks: $1"
}

if grep -n 'uses: taiki-e/install-action' \
    "$root/.github/workflows/ci.yml" \
    "$root/.github/actions/client-artifacts/action.yml"; then
    fail "CI callers must share the cached tool action"
fi
[[ -f "$action/action.yml" ]] || fail "ci-tools action.yml is missing"
[[ -f "$action/prepare.sh" ]] || fail "ci-tools prepare.sh is missing"
[[ -f "$action/finish.sh" ]] || fail "ci-tools finish.sh is missing"
require 'actions/cache/restore@' "$action/action.yml"
require 'actions/cache/save@' "$action/action.yml"
require 'uses: taiki-e/install-action@' "$action/action.yml"
require 'if: steps.cache.outputs.cache-hit != ' "$action/action.yml"
require 'hashFiles(' "$action/action.yml"
require 'ci-tools-' "$root/.github/workflows/cache-prune.yml"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/original" "$work/cache" "$work/runner"
export RUNNER_TEMP="$work/runner"
export RUNNER_OS=Linux
for case in compact spaced other; do
    export GITHUB_ENV="$work/$case.env"
    : > "$GITHUB_ENV"
    case $case in
        compact) request='cargo-nextest,just' ;;
        spaced) request='cargo-nextest, just' ;;
        other) request=just ;;
    esac
    bash "$action/prepare.sh" "$request" contract
    grep '^CI_TOOLS_BIN=' "$GITHUB_ENV" | cut -d= -f2- > "$work/$case.bin"
    grep '^CI_TOOLS_KEY=' "$GITHUB_ENV" | cut -d= -f2- > "$work/$case.key"
done
cmp -s "$work/compact.key" "$work/spaced.key" || fail "separator spelling changed the cache key"
[[ $(< "$work/compact.bin") != $(< "$work/other.bin") ]] || fail "different tool sets share a staging directory"
for tool in just cargo-nextest; do
    printf '#!/usr/bin/env bash\nexit 0\n' > "$work/original/$tool"
    chmod +x "$work/original/$tool"
done
original_path=$PATH
export PATH="$work/original:$PATH"
export CI_TOOLS_BIN="$work/cache"
export GITHUB_PATH="$work/github-path"
: > "$GITHUB_PATH"

bash "$action/finish.sh" false 'cargo-nextest,just'
for tool in just cargo-nextest; do
    [[ -x "$work/cache/$tool" ]] || fail "$tool was not staged"
done
require "$work/cache" "$GITHUB_PATH"

rm -rf "$work/original"
export PATH=$original_path
bash "$action/finish.sh" true 'cargo-nextest,just'
rm "$work/cache/just"
export PATH=$original_path
if bash "$action/finish.sh" true just >/dev/null 2>&1; then
    fail "an incomplete restored cache passed verification"
fi

echo "test-ci-tools: cache staging, restoration and incomplete-cache refusal passed"
