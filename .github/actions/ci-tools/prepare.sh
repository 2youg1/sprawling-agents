#!/usr/bin/env bash
# Turn the requested tool set and the action content digest into one cache key
# and a staging directory that cannot mix different tool sets.
set -euo pipefail

request=$1
contract=$2
normalized=
for spec in ${request//,/ }; do
    normalized+="${normalized:+,}$spec"
done
id=$(printf '%s' "$normalized" | sha256sum | cut -d' ' -f1)
{
    echo "CI_TOOLS_BIN=$RUNNER_TEMP/ci-tools-$id"
    echo "CI_TOOLS_KEY=ci-tools-${RUNNER_OS}-${id}-${contract}"
} >> "$GITHUB_ENV"
