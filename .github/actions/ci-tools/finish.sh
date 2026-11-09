#!/usr/bin/env bash
# Stage newly installed tools, or verify a restored set. The first argument is
# the cache action's cache-hit value; the second is install-action's tool list.
set -euo pipefail

hit=$1
request=$2
bin=${CI_TOOLS_BIN:?CI_TOOLS_BIN must name the staging directory}
mkdir -p "$bin"
printf '%s\n' "$bin" >> "$GITHUB_PATH"
export PATH="$bin:$PATH"
hash -r

found=false
for spec in ${request//,/ }; do
    tool=${spec%%@*}
    tool=${tool%%+*}
    [[ $tool =~ ^[a-z0-9][a-z0-9-]*$ ]] || {
        echo "ci-tools: unsupported tool name '$tool'" >&2
        exit 2
    }
    found=true
    if [[ $hit != true ]]; then
        source=$(command -v "$tool") || {
            echo "ci-tools: install-action did not put '$tool' on PATH" >&2
            exit 1
        }
        cp -f "$source" "$bin/"
    fi
    if [[ ! -f "$bin/$tool" && ! -f "$bin/$tool.exe" ]]; then
        echo "ci-tools: restored cache is incomplete: '$tool' is missing" >&2
        exit 1
    fi
    command -v "$tool" >/dev/null || {
        echo "ci-tools: '$tool' is not executable on PATH" >&2
        exit 1
    }
done
[[ $found == true ]] || {
    echo "ci-tools: the tool list is empty" >&2
    exit 2
}
