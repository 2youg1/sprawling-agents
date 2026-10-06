# sprawling's daily loop. AGENTS.md says when each recipe runs; this file
# is what it runs.
set shell := ["bash", "-uc"]

default: check

# The whole check, run on the tree that merges a batch of branches into
# `main` (AGENTS.md, Verification tier 3).
#
# `build-web` sits before `gates` because two of the gates - render and
# npm - judge artifacts rather than sources: they need `crates/sprawling/web-dist`
# and `client/node_modules` to exist, and they refuse rather than skip
# when those are absent, so a check that never built them would be red,
# not silently green. CI says the same thing through
# `.github/actions/client-artifacts`, which runs `build-web` for the jobs
# that run those gates. It sits before `clippy` as well, because
# `crates/sprawling/build.rs` embeds the bundle: built after the compile,
# a stale bundle is what clippy and the tests would see.
#
# Formatting runs right after `prereqs`, because it answers in seconds
# and every later step compiles for minutes. The source gates and the
# Lean models follow for the same reason: each answers in seconds, before
# clippy's minutes.
#
# `ci.yml` runs the same recipes, one job per slice of this line, so a
# green pull request implies exactly this and no less.
check: prereqs fmt-check gates-sources models build-web clippy features test gates check-client check-desktop

# The merge's whole check: the phases of `check`, but every phase runs
# even after another has failed, so one run names every red rather than
# the first. build-web runs first and once, because the compile embeds
# the bundle and the gates read it. The phases that share the workspace
# target run as one chain, and the client and the desktop's Zig leaf
# run beside it, so the run takes as long as its longest chain.
# Each phase logs to <target>/check-all/<phase>.log, and phases.tsv holds
# one `phase<TAB>exit<TAB>seconds` row per phase. Naming phases runs
# those alone, which is how a red phase is rerun after its fix.
check-all *phases:
    #!/usr/bin/env bash
    set -uo pipefail
    out="${CARGO_TARGET_DIR:-target}/check-all"
    rm -rf "$out" && mkdir -p "$out" || exit 2
    wanted=" {{phases}} "
    phase() {
        local name=$1 start=$SECONDS rc
        shift
        [ "$wanted" = "  " ] || [[ "$wanted" == *" $name "* ]] || return 0
        "$@" > "$out/$name.log" 2>&1
        rc=$?
        printf '%s\t%s\t%s\n' "$name" "$rc" "$((SECONDS - start))" >> "$out/phases.tsv"
    }
    just prereqs || exit 2
    phase fmt just fmt-check
    phase build-web just build-web
    { phase clippy just clippy; phase features just features
      phase test just test --no-fail-fast; phase gates just gates; } &
    { phase models just models; } &
    { phase client just client-checks; } &
    { phase desktop just check-desktop; } &
    wait
    touch "$out/phases.tsv"
    column -t -s $'\t' "$out/phases.tsv"
    awk -F'\t' '$2 != 0 { red = 1 } END { exit red }' "$out/phases.tsv"

# The branch check (AGENTS.md, Verification tier 2) on <base>...HEAD:
# the guard over the branch's own commits, formatting, clippy on the
# workspace when a package changed, every test of a changed package and
# the tests of its dependents that name an item or module the diff
# touched, every gate that reads sources, and the client, its artifact
# gates and the desktop's Zig leaf when the branch touched them. Every step runs even
# after another failed. Clippy lints the whole workspace on all features,
# the set `check` compiles, because a `-p` selection unifies features
# differently and recompiles shared dependencies for every selection.
# A green result is recorded against the tree, and a clean worktree on a
# recorded tree returns at once, so one tree is never checked twice.
check-branch base="main":
    #!/usr/bin/env bash
    set -uo pipefail
    tree=$(git rev-parse 'HEAD^{tree}') && mb=$(git merge-base '{{base}}' HEAD) || exit 2
    mark="${CARGO_TARGET_DIR:-target}/check-branch/$tree.ok"
    clean=$([ -z "$(git status --porcelain)" ] && echo yes)
    if [ -n "$clean" ] && [ -f "$mark" ]; then echo "check-branch: tree $tree is already green"; exit 0; fi
    changed=$(git diff --name-only "$mb" HEAD)
    touched() { printf '%s\n' "$changed" | grep -q "^$1/"; }
    # Which package holds a path is cargo metadata's answer (tools/xtask/Spec.lean
    # §8-39); the paths go through stdin, since a rename branch
    # lists more of them than one command line holds.
    packages=$(printf '%s\n' "$changed" | cargo xtask members --owning | tr -d '\r') || exit 2
    rc=0
    step() { echo "== $1"; shift; "$@" || { rc=1; echo "== red: $*"; }; }
    gates=$(cargo xtask gates --list | tr -d '\r' | grep -Evx 'guard|features|render|budget|npm') || exit 2
    step guard cargo xtask gates guard --range "$mb..HEAD"
    step fmt just fmt-check
    touched client && step build-web just build-web
    if [ -n "$packages" ]; then
        step clippy just clippy
        step tests just test --no-fail-fast --no-tests=warn -E "$(just branch-tests "$mb" $packages)"
    fi
    # shellcheck disable=SC2086
    step gates cargo xtask gates $gates --range "$mb..HEAD"
    step deny just deny
    touched client && step client just client-checks && step artifacts cargo xtask gates render budget npm
    touched crates/desktop/ffi && step desktop just check-desktop
    # A Lean specification is proved by `models`, which no other step runs;
    # the Lean package's own three files change what it proves
    # (tools/xtask/Spec.lean §8-43).
    printf '%s\n' "$changed" | grep -qE '\.lean$|^(lakefile\.toml|lean-toolchain|lake-manifest\.json)$' \
        && step models just models
    [ "$rc" = 0 ] && [ -n "$clean" ] && mkdir -p "$(dirname "$mark")" && touch "$mark"
    exit "$rc"

# The nextest filterset `check-branch` runs: every test of each changed
# package, and in each workspace package that depends on one, the test
# files that name an item or module the diff since <base> added, removed
# or edited. Names under four characters are too common to mean
# anything and are left out.
branch-tests base +packages:
    #!/usr/bin/env bash
    set -uo pipefail
    changed=" {{packages}} "
    names=$( { git diff -U0 '{{base}}' HEAD -- '*.rs' | grep -E '^[+-][^+-]' \
                 | grep -oE '\b(fn|struct|enum|trait|const|static|type|mod)\s+[A-Za-z_][A-Za-z0-9_]*' | awk '{print $2}'
               git diff --name-only '{{base}}' HEAD -- '*.rs' | sed -n 's#.*/src/##p' | sed 's#\.rs$##' | tr '/' '\n'; } \
             | grep -Evx 'main|lib|mod|tests?|.{0,3}' | sort -u | paste -sd'|')
    filters=$(for p in {{packages}}; do printf 'package(%s)\n' "$p"; done)
    if [ -n "$names" ]; then
        for p in {{packages}}; do
            cargo tree --workspace --locked -i "$p" -e normal,dev,build --depth 1 --prefix none --format '{p}' | awk '{print $1}'
        done | sort -u | while read -r dep; do
            [[ "$changed" == *" $dep "* ]] && continue
            dir=$(cargo xtask members --dir "$dep" | tr -d '\r') || exit 2
            git grep -lwE "$names" -- "$dir/tests/*.rs" "$dir/src/**/tests.rs" "$dir/src/**/*_tests.rs" | while read -r file; do
                rel=${file#"$dir"/}; rel=${rel%.rs}
                case $rel in
                    # A directory under tests/ is a test binary only with a main.rs
                    # or a sibling <dir>.rs; otherwise it holds fixtures.
                    tests/*/*) binary=$(echo "$rel" | cut -d/ -f2)
                               if [ -f "$dir/tests/$binary/main.rs" ] || [ -f "$dir/tests/$binary.rs" ]; then
                                   printf 'binary_id(%s::%s)\n' "$dep" "$binary"
                               fi ;;
                    tests/*) printf 'binary_id(%s::%s)\n' "$dep" "${rel#tests/}" ;;
                    *) module=${rel#src/}; module=${module%/tests}; module=${module#main/}; module=${module#lib/}
                       case $module in bin/*/*) module=${module#bin/*/} ;; esac
                       case $module in
                           main|lib|tests) printf 'package(%s)\n' "$dep" ;;
                           *) printf '(package(%s) & test(/^%s::/))\n' "$dep" "${module//\//::}" ;;
                       esac ;;
                esac
            done
        done
    fi | cat <(printf '%s\n' "$filters") - | sort -u | paste -sd'|' | sed 's/|/ | /g'

# What this repository's loop needs installed, read from the doctor's
# develop tier.
#
# `crates/sprawling/src/doctor/table.rs` is the one list: `sprawling
# doctor` and the settings page install from it, and a test there holds
# `prereqs.tsv` beside it to exactly what the table renders
# (`crates/sprawling/Spec.lean` §8-58). This recipe reads that file rather
# than keeping a list of its own, because two lists of one fact is how a
# person installed everything one of them named and still met a red
# gate. Each row is one probe, so the whole recipe costs milliseconds and
# `check` opens with it - a missing tool is named before a compile rather
# than twenty minutes into one. A row whose probe is `-` is one only the
# doctor can look for, a browser family, and the gate that needs it
# names it itself.
#
# A required row missing fails this recipe; an optional one is printed
# and passes, because `just check` skips it or a recipe run on purpose
# needs it. `just prereqs list` prints one `class<TAB>name<TAB>probe`
# line per row, which flake.nix's `devshell-covers-just-check` checks
# its devshell against, so a tool added to the table and not to the
# devshell turns that check red.
prereqs mode="check":
    #!/usr/bin/env bash
    set -uo pipefail
    mode='{{mode}}'
    case "$mode" in
        check|list) ;;
        *) echo "prereqs takes 'check' or 'list', not '$mode'" >&2; exit 2 ;;
    esac
    case "$(uname -s)" in
        MINGW*|MSYS*|CYGWIN*) platform=windows ;;
        Darwin) platform=macos ;;
        *) platform=linux ;;
    esac
    missing=0
    while IFS=$'\t' read -r class name probe windows macos linux purpose; do
        case "$class" in
            '#'*|'') continue ;;
        esac
        if [ "$mode" = list ]; then
            printf '%s\t%s\t%s\n' "$class" "$name" "$probe"
            continue
        fi
        if [ "$probe" = - ] || eval "$probe" >/dev/null 2>&1; then
            continue
        fi
        case "$platform" in
            windows) install=$windows ;;
            macos) install=$macos ;;
            linux) install=$linux ;;
        esac
        printf '%-8s %-17s %s\n         install: %s\n' "$class" "$name" "$purpose" "$install"
        if [ "$class" = required ]; then
            missing=$((missing + 1))
        fi
    done < '{{justfile_directory()}}/crates/sprawling/src/doctor/table/prereqs.tsv'
    if [ "$mode" = list ]; then
        exit 0
    fi
    if [ "$missing" -gt 0 ]; then
        echo "$missing required tool(s) absent: just check would go red before it compiles anything"
        exit 1
    fi
    echo "prereqs: every required tool answers"

fmt:
    cargo fmt --all

# The desktop server's Zig leaf is formatted by `zig fmt`, on Windows,
# the one platform the leaf is built on and the one where the doctor
# makes Zig required (`crates/desktop/Spec.lean` section 8-12).
fmt-check:
    cargo fmt --all --check
    {{ if os() == "windows" { "zig fmt --check crates/desktop/ffi/zig" } else { "echo 'zig fmt: the Zig leaf is built on Windows only'" } }}

# --all-features is load-bearing: code behind a feature (runtime/wasm, */conformance)
# escapes the zero-warning gate without it.
clippy:
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

# The feature combinations this repository checks: the workspace on its
# default features, test targets included; `sprawling` without its
# default features, which is the binary with no execution engine -
# `sandbox` is a default feature, so nothing else compiles that build
# (`crates/sprawling/Spec.lean` §8-157); and `wire` with `server` off - which is the
# reason that feature exists, since it keeps the TCP stack out of a
# wasm32 build. Code behind a feature is compiled the day somebody turns
# that feature on, and a combination that does not build is what the
# first person to turn it on meets.
#
# This recipe is the definition of that check. `just check` runs it and
# `ci.yml`'s clippy job calls it, so the check a person runs at their
# desk and the check a pull request gets are one command rather than two
# spellings of it, which could lose a flag apart from each other.
#
# --all-targets on the first pass because `cargo check` alone does not
# compile test targets, and a test target that uses an item behind a
# feature without declaring that feature compiles only under
# `--all-features` - the configuration a person never builds.
#
# Three check-mode passes, seconds each on a warm cache; the zero-warning
# rule stays with `clippy`, which sees every feature at once.
features:
    cargo check --workspace --locked --all-targets
    cargo check -p sprawling --no-default-features --locked --all-targets
    cargo check -p sprawling-wire --no-default-features --locked

# `just prereqs` names cargo-nextest; `just test-std` is the fallback.
[positional-arguments]
test *args:
    cargo nextest run --workspace --locked --all-features "$@"

test-std:
    cargo test --workspace --locked

# Compile the suite once; every CI slice consumes this archive.
test-archive file:
    cargo nextest archive --workspace --locked --all-features --archive-file '{{file}}'

# Names, filtersets and the environment each slice needs live here; CI
# consumes the checked matrix rather than maintaining a second roster.
# List from the archive, then reuse its metadata for each filterset, so
# coverage needs no second compilation.
#
# A part is a disjoint set of tests; a slice is the parts one runner
# takes. A slice's runner pays ~60 s of setup before its first test while
# most parts run in 10-80 s, so the parts are bundled into four slices
# balanced on their measured nextest time rather than given a runner
# each. trybuild is split by package and citysim stands alone as parts;
# the other unit tests are grouped by crate, the integration tests by
# package, and the remaining kinds fall into one part. The unit
# scenarios of accounting are split into execution, collaboration and
# the rest, and the first two share a slice that carries no other heavy
# part, because their file writes and worker scenarios are the heaviest
# load one runner takes. The domain module names live only in these
# predicates, and the rest is generated from them.
#
# `environment` names what a slice installs beyond the toolchain and
# the archive: `compile` builds projects of its own (trybuild), so it
# needs Zig, the registry and its scratch cache; `client` holds the xtask
# tests that read `client/node_modules` and the bundle; `plain` needs
# neither. A part moves between slices by editing one list below.
test-slice-plan file out:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p '{{out}}'
    cargo nextest list --archive-file '{{file}}' --workspace-remap . --extract-to . --extract-overwrite --run-ignored all --message-format json > '{{out}}/full.json'
    python - '{{out}}' <<'PY'
    import json, pathlib, subprocess, sys
    out = pathlib.Path(sys.argv[1])
    parts = {}
    for package in ['kernel', 'storage', 'collab', 'runtime', 'wire']:
        parts['trybuild-' + package] = f'package(=sprawling-{package}) & binary(=trybuild)'
    parts['citysim'] = 'package(=citysim)'
    unit = '(kind(lib) | kind(bin) | kind(proc-macro)) & !package(=citysim)'
    groups = {
        'runtime': 'package(=sprawling-runtime)',
        'sprawling': 'package(=sprawling)',
        'xtask': 'package(=xtask)',
        'kernel-storage': '(package(=sprawling-kernel) | package(=sprawling-storage))',
        'gateway-protocols': '(package(=sprawling-gateway) | package(=sprawling-agent-protocols))',
    }
    accounting = 'package(=sprawling-accounting)'
    accounting_domains = {
        'execution': 'test(/^worker::(driving|dispatching|freezing|mcp|workbench)::/)',
        'collaboration': 'test(/^worker::(waking|plans|reviewing|settling|commanding)::/)',
    }
    for domain, predicate in accounting_domains.items():
        groups['accounting-' + domain] = f'({accounting} & {predicate})'
    groups['accounting-other'] = f'({accounting} & !(' + ' | '.join(accounting_domains.values()) + '))'
    for name, packages in groups.items():
        parts['unit-' + name] = f'{unit} & {packages}'
    parts['unit-other'] = unit + ' & !(' + ' | '.join(groups.values()) + ')'
    integration = 'kind(test) & !binary(=trybuild) & !package(=citysim)'
    parts['integration-sprawling'] = integration + ' & package(=sprawling)'
    parts['integration-other'] = integration + ' & !package(=sprawling)'
    parts['other-kinds'] = '!(kind(lib) | kind(bin) | kind(proc-macro) | kind(test)) & !package(=citysim)'
    bundles = [
        ('trybuild', 'compile', ['trybuild-kernel', 'trybuild-storage', 'trybuild-collab', 'trybuild-runtime',
                                 'trybuild-wire', 'unit-kernel-storage', 'other-kinds']),
        ('sprawling', 'client', ['unit-xtask', 'unit-sprawling', 'integration-sprawling', 'integration-other']),
        ('accounting', 'plain', ['unit-accounting-other', 'unit-runtime', 'unit-other', 'unit-gateway-protocols']),
        ('worker', 'plain', ['unit-accounting-execution', 'unit-accounting-collaboration', 'citysim']),
    ]
    bundled = [part for _, _, members in bundles for part in members]
    if sorted(bundled) != sorted(parts) or len(bundled) != len(set(bundled)):
        sys.exit('test-slice-plan: every part belongs to exactly one slice')
    active = []
    for name, environment, members in bundles:
        row = {'name': name, 'environment': environment,
               'filter': ' | '.join(f'({parts[member]})' for member in members)}
        with (out / (name + '.json')).open('w', encoding='utf-8') as output:
            subprocess.run(['cargo', 'nextest', 'list', '--cargo-metadata', 'target/nextest/cargo-metadata.json',
                            '--binaries-metadata', 'target/nextest/binaries-metadata.json', '--workspace-remap', '.',
                            '--run-ignored', 'all', '--message-format', 'json', '-E', row['filter']], stdout=output, check=True)
        listing = json.loads((out / (name + '.json')).read_text(encoding='utf-8'))
        if any(case['filter-match']['status'] == 'matches' for suite in listing['rust-suites'].values()
               for case in suite.get('testcases', {}).values()):
            active.append(row)
    (out / 'matrix.json').write_text(json.dumps({'include': active}), encoding='utf-8')
    PY
    just test-slice-coverage '{{out}}'

# Compare identities, not counts: equal totals can hide a missing test
# behind a duplicate. Ignored tests stay in the proof and remain ignored
# when run, exactly as in the unsliced suite.
test-slice-coverage out:
    #!/usr/bin/env bash
    set -euo pipefail
    python - '{{out}}' <<'PY'
    import collections, json, pathlib, sys
    out = pathlib.Path(sys.argv[1])
    def identities(path):
        suites = json.loads(path.read_text(encoding='utf-8'))['rust-suites']
        return {(suite['binary-id'], name) for suite in suites.values()
                for name, case in suite.get('testcases', {}).items() if case['filter-match']['status'] == 'matches'}
    full = identities(out / 'full.json')
    matrix = json.loads((out / 'matrix.json').read_text(encoding='utf-8'))['include']
    names = [row['name'] for row in matrix]
    if not full or not names or len(names) != len(set(names)):
        sys.exit('test-slice-coverage: empty suite/matrix or duplicate slice name')
    owners = collections.defaultdict(list)
    for name in names:
        selected = identities(out / (name + '.json'))
        print(f'{name}: {len(selected)} tests')
        for identity in selected:
            owners[identity].append(name)
    defects = [f'unassigned: {identity}' for identity in sorted(full - owners.keys())]
    defects += [f'outside full suite: {identity}' for identity in sorted(owners.keys() - full)]
    defects += [f'assigned twice: {identity}: {owners[identity]}' for identity in sorted(owners) if len(owners[identity]) != 1]
    if defects:
        sys.exit('test-slice-coverage: ' + chr(10).join(defects))
    print(f'test-slice-coverage: {len(full)} tests assigned exactly once across {len(names)} slices')
    PY

# Unpack at the checkout path compiled into tests that spawn workspace
# binaries. A `compile` slice fetches first, because trybuild compiles its
# own project offline; the other environments read no registry.
test-slice file environment filter:
    #!/usr/bin/env bash
    set -euo pipefail
    case '{{environment}}' in
        compile) cargo fetch --locked ;;
        client | plain) ;;
        *) echo "test-slice: unknown environment '{{environment}}'; test-slice-plan names compile, client and plain" >&2; exit 2 ;;
    esac
    cargo nextest run --archive-file '{{file}}' --workspace-remap . --extract-to . --extract-overwrite -E '{{filter}}' --no-fail-fast --no-tests=warn

# Compare completed successful Actions inventories, exported with:
# gh run view <id> --json conclusion,headSha,startedAt,updatedAt,jobs
# Queueing stays in run and test-chain wall time; job durations are
# reported separately rather than presented as the algorithm's gain.
test-timings baseline current:
    #!/usr/bin/env bash
    set -euo pipefail
    python - '{{baseline}}' '{{current}}' <<'PY'
    import datetime, json, pathlib, sys
    def instant(value):
        return datetime.datetime.fromisoformat(value.replace('Z', '+00:00'))
    def seconds(start, end):
        elapsed = (instant(end) - instant(start)).total_seconds()
        if elapsed < 0:
            sys.exit('test-timings: end precedes start')
        return elapsed
    def inventory(path):
        run = json.loads(pathlib.Path(path).read_text(encoding='utf-8'))
        if run['conclusion'] != 'success':
            sys.exit(f'test-timings: {path} is not a successful completed run')
        jobs = {job['name']: job for job in run['jobs']}
        slices = [job for job in run['jobs'] if job['name'].startswith('test (')]
        if len(jobs) != len(run['jobs']) or not slices or 'test build' not in jobs or 'test' not in jobs:
            sys.exit('test-timings: duplicate job names or missing test build, slices or verdict')
        for job in [jobs['test build'], jobs['test'], *slices]:
            if job['conclusion'] != 'success' or not job['completedAt']:
                sys.exit(f"test-timings: {job['name']} did not finish successfully")
        duration = lambda job: seconds(job['startedAt'], job['completedAt'])
        return run, slices, {
            'ci-wall-seconds': seconds(run['startedAt'], run['updatedAt']),
            'test-chain-wall-seconds': seconds(jobs['test build']['startedAt'], jobs['test']['completedAt']),
            'test-build-seconds': duration(jobs['test build']),
            'longest-slice-job-seconds': max(map(duration, slices)),
        }
    before, before_slices, baseline = inventory(sys.argv[1])
    after, after_slices, current = inventory(sys.argv[2])
    print('metric	baseline	current	delta-seconds')
    for metric in baseline:
        print(f'{metric}	{baseline[metric]:g}	{current[metric]:g}	{current[metric] - baseline[metric]:+g}')
    for label, run, slices in [('baseline', before, before_slices), ('current', after, after_slices)]:
        print(f"{label} HEAD: {run['headSha']}")
        for job in sorted(slices, key=lambda job: job['name']):
            print(f"{label} {job['name']}: {seconds(job['startedAt'], job['completedAt']):g}s")
    PY

# The packages every platform builds and tests. The desktop server
# and its FFI seam serve a Windows desktop only, and xtask judges the
# repository rather than shipping in it, so `ci.yml` lints and tests
# those three on Windows alone; on macOS and Linux the two desktop
# packages compile only as what the binary links.
core_packages := "--workspace --exclude sprawling-desktop --exclude sprawling-desktop-ffi --exclude xtask"

# Clippy and the suite over the core packages, on whatever platform runs it.
check-core:
    cargo clippy {{core_packages}} --all-targets --all-features --locked -- -D warnings
    cargo nextest run {{core_packages}} --locked --all-features --no-fail-fast

# The three gates that judge a built artifact rather than a source:
# `render` opens `crates/sprawling/web-dist`, `npm` reads `client/node_modules`,
# `budget` weighs both. The one list of them; `gates-sources` runs every
# other gate on the roster.
artifact_gates := "render npm budget"

# All machine gates (xtask), then the supply-chain read. Split in two so
# `check` can run the source half before anything compiles; `just` runs a
# dependency once per invocation, so every gate runs exactly once.
gates: gates-sources gates-artifacts
    just deny

# The gates that read sources alone, which answer in seconds, so `check`
# runs them before clippy's minutes and a red arrives first.
gates-sources:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo xtask gates $(cargo xtask gates --list | grep -vxE '{{replace(artifact_gates, " ", "|")}}')

# The gates that judge the client bundle, after it is built.
gates-artifacts: build-web
    cargo xtask gates {{artifact_gates}}

# The Lean specifications: every module under `crates/`, each a part of a
# crate's specification (ARCHITECTURE.md section 11, "Specifications in
# Lean"). `lakefile.toml` sets `warningAsError`, so a `sorry` or an `admit`,
# which Lean reports as a warning, fails the build; an `axiom` raises no
# warning at all, so the `spec` gate refuses it by its shape, where Lean is
# not needed to see it (tools/xtask/Spec.lean §8-42). The checker under
# `tools/adversary/` is not built here: it attacks the binary and stays out
# of every required check.
#
# Lean is a required tool (`just prereqs`). Where `lake` is absent this
# recipe fails and says how to install it, because a proof nobody ran is not
# a pass.
models:
    #!/usr/bin/env bash
    set -euo pipefail
    if ! command -v lake >/dev/null 2>&1; then
        echo "models: lake is absent, so no specification was proved; \`just prereqs\` prints the lines that install elan and the toolchain lean-toolchain pins" >&2
        exit 1
    fi
    lake build Spec

# Every commit subject and ruling trailer in a range of history
# (tools/xtask/Spec.lean §8-35). Not in `check`, because a tree has no
# range: CI names it, and at a desk it is `just commits main..HEAD`.
commits range:
    cargo xtask commits --range '{{range}}'

# The supply-chain read: licences, banned crates, and where they came
# from. This recipe is the one place that says which checks the daily
# loop runs, and `ci.yml`'s supply job calls it instead of listing them
# again.
#
# `advisories` is deliberately absent. An advisory published today
# against a dependency nobody touched would turn a change red for a
# reason its author did not write, which is how people learn to ignore a
# red build; `nightly.yml` runs that check, where red means "go and
# look". What each check judges is `deny.toml`'s to say.
#
# Without cargo-deny installed this prints one line and succeeds, because
# CI installs it on every push. **A cargo-deny that answers and refuses
# fails this recipe**: the recipe tells a missing tool from a refusal by
# asking `command -v` first, because a single `&& … || echo` would report
# "not installed" for a real violation too.
deny:
    #!/usr/bin/env bash
    set -euo pipefail
    if ! command -v cargo-deny >/dev/null 2>&1; then
        echo "cargo-deny not installed locally; CI runs it on every push"
        exit 0
    fi
    cargo deny check bans licenses sources

# What no cargo command runs for `crates/desktop`: the Zig leaf's own
# tests - its unit tests, its seeded property tests and one input of its
# fuzz test - on Windows, the one platform the leaf is built on
# (`crates/desktop/Spec.lean` section 8-12). The server and its seam are workspace
# members, so clippy, the suite and the licence read judge them with
# everything else.
check-desktop:
    {{ if os() == "windows" { "zig test crates/desktop/ffi/zig/leaf.zig --cache-dir target/zig-test" } else { "echo 'zig test: the Zig leaf is built on Windows only'" } }}

# The Rust side's fuzz of crates/desktop/ffi's Zig leaf: its four buffer rules
# against their Rust reference, for `rounds` drawn inputs from `seed`
# (hexadecimal), on Windows, the one platform the leaf is built on. A
# disagreement prints the seed and the round it replays from. Drawn
# rather than coverage-guided: libFuzzer has no platform for this leaf
# today (`crates/desktop/Spec.lean` D12).
fuzz-desktop rounds="1000000" seed="5eedf022":
    DESKTOP_FFI_FUZZ_ROUNDS={{rounds}} DESKTOP_FFI_FUZZ_SEED={{seed}} cargo nextest run -p sprawling-desktop-ffi --locked --run-ignored only -E 'test(/for_as_long_as_asked/)'

# The browser client (client/Spec.lean): Svelte + Effect, built by
# Vite under bun, bundled into crates/sprawling/web-dist where
# crates/sprawling/build.rs reads it and where `cargo package` archives it. `just prereqs` names bun. `--frozen-lockfile` makes bun.lock
# the authority, so a build cannot resolve a version nobody committed.
build-web:
    cd client && bun install --frozen-lockfile && bun run build

# The bundle in crates/sprawling/web-dist, built only when it is absent or
# some file under client/ (its sources, its lockfile, its configuration)
# is newer than the bundle's index.html, which Vite writes on every
# build after emptying the directory. A recipe that measures the binary
# depends on this one, so a repeated run keeps the bundle's mtimes and
# cargo keeps the binary. `check` and `dist` keep `build-web`, which
# always builds. `find -newer`, `-prune` and `-quit` behave the same in
# Git Bash on Windows and in the find of macOS and Linux.
web-bundle:
    if [ -f crates/sprawling/web-dist/index.html ] && [ -z "$(find client -path client/node_modules -prune -o -type f -newer crates/sprawling/web-dist/index.html -print -quit)" ]; then echo "web-bundle: crates/sprawling/web-dist is newer than every file under client/, kept"; else '{{just_executable()}}' --justfile '{{justfile()}}' build-web; fi

# The client's own gates: lint (no `any`, no `as`, no throw, no try, no
# non-exhaustive switch), typecheck, and its tests.
#
# `build-web` owns the dependency install, so this recipe depends on it
# instead of holding a second `bun install` line that could drift from
# it. `just` runs a dependency once per invocation, so `just check`
# installs and bundles exactly once even though both recipes are in it.
check-client: build-web client-checks

# The generated offline playback page, then the client's own gates on
# the dependencies build-web installed. Both checks run under Bun.
client-checks:
    bun crates/city/skills/playback/assemble.js --check
    cd client && bun run lint && bun run typecheck && bun run test

# The gate that opens the gallery in a real engine, on its own: roles,
# accessible names, landmarks, one left edge, nothing outside its box.
# Needs `just build-web` first, and says so when the bundle is absent.
render:
    cargo xtask render

# The kernel propositions kani holds against real MIR. Not in
# `just check`, for the same reason as `adversary`: kani has no Windows
# host, and a check that always skips on a Windows desk would read as
# green without having run. Without kani installed it prints one line
# and succeeds; CI's Linux `proof` job is where it must pass.
# `cargo xtask proof --list` prints the harness names it will run.
proof:
    cargo xtask proof

# The citysim scenarios; the crate's test suite is the entry point.
# No seed argument: the scenarios are fixed scripts driven by a counting
# clock on one thread, so a failure replays from the script rather than
# from a number. A seed returns when a random scenario batch does.
sim:
    cargo test --package citysim --locked

# The stand-in provider (tools/citysim/Spec.lean section 8-10):
# plays a wire script on a loopback port, appends every exchange to the
# record, and prints `SPRAWLING_PROVIDER=<url>` before it answers, so the
# check started beside it reads the URL from that line. `listen` is a
# loopback address, a free port when left out.
provider script record *listen:
    cargo run --package citysim --bin provider --locked -- {{script}} {{record}} {{listen}}

# Every page of the client at 1440 and 1920, dark and light, as PNG files
# and an index under target/shots (tools/xtask/Spec.lean
# §8-44). Not a gate: a person reads the pictures. `--origin <url>`
# photographs a served city instead of the bundle.
shots *args: build-web
    cargo xtask shots {{args}}

# The half of the shots camera's check that needs an engine that draws
# frames: a page whose state is written by the frame after the scroll
# (tools/xtask/Spec.lean §8-44, D25). It is `#[ignore]`d for `just check`,
# because a hosted runner's engine draws no frame at all inside a dump
# run and the check was red on every CI run for it.
shots-frames:
    cargo nextest run -p xtask --locked --run-ignored only -E 'test(a_frame_after_the_scroll)'

# Create a crate's Spec.lean skeleton (tools/xtask/Spec.lean §8-41).
spec crate:
    cargo xtask spec {{crate}}

# Requires cargo-fuzz + nightly; `nightly.yml` runs the smoke batch.
fuzz target:
    cargo fuzz run {{target}} --fuzz-dir tools/fuzz

# Requires cargo-mutants. Every module held to a mutation score, run in one
# shard and scored. The modules, their file globs and their thresholds are the
# rows of tools/xtask/budgets.toml that carry `mutation_files`; they are
# enforced here rather than in `just check` because a full mutation run is
# hours, and a gate nobody waits for is a gate nobody runs. `on-demand.yml`
# runs the same two recipes in eight shards (`-f job=mutants-modules`).
mutants: (mutants-modules "0/1") (mutants-score "mutants.out")

# The mutants of every module budgets.toml scores, one `shard` (`k/n`) of
# them; the report lands in `mutants.out`. cargo-mutants exits 2 when a
# mutant survives and 3 when one timed out, and both are accepted, because
# the threshold is `mutants-score`'s decision over all shards together.
# The error value gives a function returning the kernel's error one more
# mutant; outside the functions whose error type it names it does not
# compile, and cargo-mutants counts it unviable, not missed.
# `--all-features` builds and tests each mutant as `test` does: code behind
# a feature (the kernel's `schema` and `conformance` modules) otherwise
# compiles to nothing, so its mutants pass untested and count as missed.
mutants-modules shard:
    #!/usr/bin/env bash
    set -euo pipefail
    py="$(command -v python || command -v python3)"
    mapfile -t globs < <("$py" -c 'import tomllib; rows = tomllib.load(open("tools/xtask/budgets.toml", "rb")); [print(glob) for row in rows.values() if isinstance(row, dict) for glob in row.get("mutation_files", [])]' | tr -d '\r')
    if [ "${#globs[@]}" -eq 0 ]; then
        echo "mutants-modules: no row of tools/xtask/budgets.toml carries mutation_files; add the module's file globs to its row" >&2
        exit 2
    fi
    files=()
    for glob in "${globs[@]}"; do files+=(--file "$glob"); done
    status=0
    cargo mutants --workspace --all-features "${files[@]}" --shard '{{shard}}' --test-tool nextest --minimum-test-timeout 60 --error 'kernel::AxError::failure(kernel::AxCode::InvalidArgs, "mutant", "mutant")' || status=$?
    case "$status" in
        0|2|3) ;;
        *) exit "$status" ;;
    esac

# Each scored module's caught, missed, timeout and unviable mutants and its
# score, read from every `outcomes.json` under `dir` (one per shard), with
# every surviving mutant listed; fails when a module scores under its
# `minimum_percent` or no mutant of it reached a verdict. The score is
# (caught + timeout) / (caught + missed + timeout): a mutant that hangs the
# tests is one the tests noticed, and an unviable mutant was never run.
mutants-score dir="mutants.out":
    #!/usr/bin/env bash
    set -euo pipefail
    py="$(command -v python || command -v python3)"
    "$py" - '{{dir}}' <<'PY'
    import fnmatch, json, pathlib, sys, tomllib
    register = tomllib.loads(pathlib.Path("tools/xtask/budgets.toml").read_text(encoding="utf-8"))
    rows = {name: row for name, row in register.items() if isinstance(row, dict) and "mutation_files" in row}
    reports = sorted(pathlib.Path(sys.argv[1]).rglob("outcomes.json"))
    if not reports:
        sys.exit(f"mutants-score: no outcomes.json under {sys.argv[1]}; run just mutants-modules first")
    verdicts = {"CaughtMutant": "caught", "MissedMutant": "missed", "Timeout": "timeout", "Unviable": "unviable"}
    counts = {name: dict.fromkeys(verdicts.values(), 0) for name in rows}
    survivors = {name: [] for name in rows}
    for report in reports:
        for outcome in json.loads(report.read_text(encoding="utf-8"))["outcomes"]:
            scenario = outcome["scenario"]
            if not isinstance(scenario, dict) or "Mutant" not in scenario:
                continue
            mutant = scenario["Mutant"]
            path = str(mutant.get("file") or mutant.get("source_file") or "").replace("\\", "/")
            verdict = verdicts.get(outcome["summary"])
            if verdict is None:
                sys.exit(f"mutants-score: {report} gives a mutant in {path} the verdict {outcome['summary']}, which this recipe does not know; teach it the verdict")
            for name, row in rows.items():
                if any(fnmatch.fnmatch(path, glob) for glob in row["mutation_files"]):
                    counts[name][verdict] += 1
                    if verdict == "missed":
                        line = mutant.get("span", {}).get("start", {}).get("line", "?")
                        survivors[name].append(f"{path}:{line}: {mutant.get('function', {}).get('function_name', '?') if isinstance(mutant.get('function'), dict) else mutant.get('function', '?')}: {mutant.get('replacement', '?')}")
    failed = []
    print(f"{'module':<28} {'caught':>7} {'missed':>7} {'timeout':>8} {'unviable':>9} {'score':>7} {'minimum':>8}")
    for name, row in rows.items():
        seen = counts[name]
        judged = seen["caught"] + seen["missed"] + seen["timeout"]
        killed = seen["caught"] + seen["timeout"]
        minimum = row["minimum_percent"]
        score = f"{100 * killed / judged:.1f}%" if judged else "none"
        print(f"{name:<28} {seen['caught']:>7} {seen['missed']:>7} {seen['timeout']:>8} {seen['unviable']:>9} {score:>7} {minimum:>7}%")
        if judged == 0 or killed * 100 < minimum * judged:
            failed.append(name)
    for name, lines in survivors.items():
        for line in lines:
            print(f"survived [{name}] {line}")
    if failed:
        sys.exit(f"mutants-score: {', '.join(failed)} under minimum_percent, or no mutant reached a verdict (a glob that names no file, or a shard whose outcomes are missing); kill the survivors listed above with a test, never by lowering the row")
    PY

# The mutants of the lines changed since `base`, one `shard` (`k/n`) of
# them, so a wave's change is judged without a whole-workspace run. The
# report lands in `mutants.out`. `base` is resolved as a branch on origin
# first, then a tag, then any commit git can name, so a dispatched run can
# diff against a release tag or a sha as well as a branch.
mutants-diff base shard:
    #!/usr/bin/env bash
    set -euo pipefail
    base='{{base}}'
    if git rev-parse --verify --quiet "refs/remotes/origin/$base^{commit}" >/dev/null; then
        tip="refs/remotes/origin/$base"
    elif git rev-parse --verify --quiet "refs/tags/$base^{commit}" >/dev/null; then
        tip="refs/tags/$base"
    elif git rev-parse --verify --quiet "$base^{commit}" >/dev/null; then
        tip="$base"
    else
        echo "mutants-diff: '$base' is not a branch on origin, a tag or a commit in this clone; fetch it, or name another base" >&2
        exit 2
    fi
    mkdir -p target
    git diff "$(git merge-base "$tip" HEAD)" HEAD -- '*.rs' > target/mutants.diff
    status=0
    cargo mutants --workspace --all-features --in-diff target/mutants.diff --shard '{{shard}}' --test-tool nextest --minimum-test-timeout 60 || status=$?
    case "$status" in
        0|2|3) ;;
        *) exit "$status" ;;
    esac

# The performance register: every budget, what it costs today, and what is gated.
budget:
    cargo xtask budget

# The wall-clock readings, never gated: citysim's load scenarios, then
# the two instruments that drive the city's own accounting loop - a relay
# round trip and the gap a second dispatch leaves in a running one
# (`crates/sprawling/Spec.lean` §8-84) - and the phases of each tool,
# its call and the secret scan of its result (`crates/runtime/Spec.lean`).
#
# The instruments are library tests, so `--lib` builds only the three
# library test binaries: an integration test would also link the
# `sprawling` executable with the dev-dependency features (kernel and
# agent_protocols `conformance`, storage `fault`), and on Windows and
# macOS an executable has one output path whatever its features, so the
# next product build relinks it.
bench:
    cargo run --release -p citysim --bin bench
    cargo nextest run -p sprawling -p sprawling-accounting -p sprawling-gateway -p sprawling-runtime --release --lib --profile bench --run-ignored only -E 'test(/::instrument_/)' --no-capture

# The four-action pressure reading (tools/citysim/Spec.lean 8-5) - install,
# startup, raise a city, open a session - measured, never gated.
#
# `web-bundle` first: without the bundle the binary embeds a placeholder
# page, and a placeholder is not the artifact a person installs - its
# weight is what the install action unpacks and the startup action loads.
# It is `web-bundle` rather than `build-web` because Vite rewrites every
# file of the bundle on each build, `crates/sprawling/build.rs` reruns on
# any bundle file whose mtime moved, and the release profile then relinks
# the whole binary; a second round on an unchanged client must measure
# the same binary rather than rebuild it.
#
# The shipped binary is built next and the bench measures the one that
# lands beside its own executable, so no run can be driven by a stale
# artifact. This recipe is where that ordering lives.
bench-startup: web-bundle
    cargo build --release -p sprawling --locked
    cargo run --release -p citysim --bin bench_startup --locked

# CycloneDX bill of materials for the release archive. Where it lands is
# written once, in `cargo xtask sbom`, and the archive's contents table
# reads that same constant.
sbom:
    cargo xtask sbom

# Two builds of one tree must be byte-identical.
repro:
    cargo xtask repro

# The whole deliverable: client bundle first, then the binary that embeds
# it, then its bill of materials.
# An optional target triple builds for a platform other than this
# machine's default.
# **The engine is part of what ships.** `sandbox`, which carries
# `runtime/wasm`, is a default feature of the sprawling package, so every
# recipe that builds or measures the product builds it without
# `--features` (citysim D9); `cargo xtask package` refuses a binary
# without the engine, so a build that lost it fails at the last step
# instead of publishing a crippled one. The size this produces is the size
# a person downloads.
dist target="": build-web
    cargo build --release -p sprawling --locked {{ if target == "" { "" } else { "--target " + target } }}
    cargo xtask sbom

# The release archive: the one file a person downloads, unpacks and runs.
# `dist` first, because the archive is assembled out of its artifacts and
# never out of whatever happened to be in target/ from an earlier build.
# The same optional triple: one recipe packages every row of the release
# matrix, so a cross-built artifact cannot be assembled by other steps.
package target="": (dist target)
    cargo xtask package {{ if target == "" { "" } else { "--target " + target } }}

# Offline chain verification (A2); strictly read-only.
replay log:
    cargo run -p sprawling --locked -- replay {{log}}

# Private, peak private and working set, in this platform's own counters
# (tools/xtask/Spec.lean §8-30). A pid reads that process. `long-turn
# [steps] [every]` reads the long-turn instrument at each of its pauses
# (citysim D10). Anything else serves a city - a fresh
# empty one, or `--city <dir>` - so the release binary is built first: a
# reading of a stale binary describes a tree nobody has.
mem *args:
    {{ if args =~ '^[0-9]+$' { "true" } else if args =~ '^long-turn' { "cargo build --release -p citysim --bin long_turn --locked" } else { "cargo build --release -p sprawling --locked" } }}
    {{ if args =~ '^long-turn' { "just mem-long-turn " + trim_start_match(args, "long-turn") } else { "cargo xtask mem " + args } }}

# The long turn's counters, read through `cargo xtask mem` each time the
# instrument pauses; the last pause's peak private is the run's peak
# (citysim D10). `just mem long-turn` builds it first.
[private]
mem-long-turn steps="500" every="100":
    #!/usr/bin/env bash
    set -euo pipefail
    coproc turn { "${CARGO_TARGET_DIR:-target}/release/long_turn" {{steps}} {{every}}; }
    while IFS= read -r line <&"${turn[0]}"; do
        echo "$line"
        case "$line" in
            step*) cargo xtask mem "${line##* }"; echo >&"${turn[1]}" ;;
        esac
    done

# The adversarial property checker in `tools/adversary/`, which lives outside the
# workspace, outside the release, and outside `just check`
# (tools/adversary/Spec.lean section 2). It is never a gate: on a machine with
# no Lean toolchain this prints one line and succeeds, so `just check` behaves
# exactly as it does where the directory is absent.
#
# This recipe is the only place that knows where the binary is. The checker is
# told through SPRAWLING_BIN and never searches for one, so an adversary run can
# never be driven by a stale binary somebody left in target/. It runs at the
# repository root, where the Lean package is, so the checker's paths are
# relative to the root.
adversary *args:
    #!/usr/bin/env bash
    set -euo pipefail
    if ! command -v lake >/dev/null 2>&1; then
        echo "skipped: Lean is not installed"
        exit 0
    fi
    cargo build -p sprawling --locked
    # Where cargo put the binary: CARGO_TARGET_DIR when it is set, which a
    # shared build directory sets, and `target/` beside this file otherwise.
    targets="${CARGO_TARGET_DIR:-target}"
    case "$targets" in /* | ?:*) ;; *) targets="$PWD/$targets" ;; esac
    binary="$targets/debug/sprawling"
    [ -f "$binary" ] || binary="$binary.exe"
    # The path is handed to a program that is not a shell, so it has to be one
    # the operating system can open. Under Git Bash `$PWD` is `/c/...`, which
    # nothing outside that shell resolves; `cygpath -m` turns it back into
    # `C:/...` and is absent everywhere it is not needed.
    ! command -v cygpath >/dev/null 2>&1 || binary="$(cygpath -m "$binary")"
    SPRAWLING_BIN="$binary" lake exe adversary {{args}}

# The usability walk a stranger takes with a release archive
# (tools/adversary/Spec.lean section 9, step 6). The archive is
# unpacked into target/acceptance, the stand-in provider is started on the
# script the acceptance world writes for the archive's skills, and the
# archive's own binary is walked through a first day, a process killed in
# the middle of a run, the morning after, and a review building's
# collaboration. The walk is handed the script as well as the record,
# because one run of it names a branch only the city knows, and the walk
# appends that run to the script once it has read the branch from the
# history (tools/adversary/Spec.lean D7). The servings that are not the
# crash end with SIGINT on macOS and Linux; on Windows each city is started
# through the launcher `serve_grouped` in a process group of its own, which
# the walk asks to send it Ctrl-Break. A city that does not exit within ten
# seconds is terminated, and the walk prints a `note` line for every such
# fallback (D8). Once every
# step held, the checklist a person works through by hand is written to
# target/acceptance/checklist.md.
#
# Never a gate, and not part of `just adversary`: the archive comes from
# `just package`, which is a release build. Name it from the repository
# root, `just acceptance target/package/<archive>.zip`, or by an absolute
# path. Lean is required here rather than skipped, because a person who
# names an archive has asked for this walk.
#
# This recipe is the only place that starts the stand-in: the checker under
# tools/adversary/ hosts no server (tools/adversary/Spec.lean section 13) and is
# told the stand-in's URL, as it is told the binary's path.
acceptance archive:
    #!/usr/bin/env bash
    set -euo pipefail
    for tool in lake unzip; do
        command -v "$tool" >/dev/null 2>&1 || {
            echo "acceptance: $tool is absent; \`just prereqs\` names the line that installs Lean, and unzip comes with Git for Windows and with every system package manager" >&2
            exit 1
        }
    done
    out="$PWD/target/acceptance"
    rm -rf "$out"
    mkdir -p "$out/archive"
    unzip -q "{{archive}}" -d "$out/archive"
    # The Windows name first: under Git Bash a glob for `sprawling` also
    # matches `sprawling.exe`, so one binary would be counted twice.
    shopt -s nullglob
    binaries=("$out"/archive/*/sprawling.exe)
    [ "${#binaries[@]}" -gt 0 ] || binaries=("$out"/archive/*/sprawling)
    [ "${#binaries[@]}" -eq 1 ] || {
        echo "acceptance: {{archive}} holds ${#binaries[@]} sprawling binaries where a release archive holds one" >&2
        exit 1
    }
    binary="${binaries[0]}"
    shelf="$(dirname "$binary")/skills"
    cargo build --package citysim --bin provider --bin serve_grouped --locked
    targets="${CARGO_TARGET_DIR:-target}"
    case "$targets" in /* | ?:*) ;; *) targets="$PWD/$targets" ;; esac
    provider="$targets/debug/provider"
    [ -f "$provider" ] || provider="$provider.exe"
    # The launcher is named only on Windows, where a city closes in order
    # only on a Ctrl-Break to a process group of its own; macOS and Linux
    # send SIGINT to the city itself (tools/adversary/Spec.lean D8).
    launcher=""
    # The paths are handed to programs that are not a shell; under Git Bash
    # they are `/c/...`, which only that shell resolves (see `adversary`).
    if command -v cygpath >/dev/null 2>&1; then
        out="$(cygpath -m "$out")"
        binary="$(cygpath -m "$binary")"
        shelf="$(cygpath -m "$shelf")"
        launcher="$(cygpath -m "$targets/debug/serve_grouped.exe")"
    fi
    lake exe acceptance script "$shelf" "$out/script.json"
    "$provider" "$out/script.json" "$out/record.jsonl" > "$out/provider.out" 2> "$out/provider.err" &
    stand_in=$!
    trap 'kill "$stand_in" 2>/dev/null || true' EXIT
    # The stand-in prints its URL before it answers anything; wait for that
    # line, and stop at once if the stand-in exits instead.
    url=""
    for _ in $(seq 1 100); do
        url="$(sed -n 's/^SPRAWLING_PROVIDER=//p' "$out/provider.out" | tr -d '\r')"
        [ -n "$url" ] && break
        kill -0 "$stand_in" 2>/dev/null || { cat "$out/provider.err" >&2; exit 1; }
        sleep 0.1
    done
    [ -n "$url" ] || { echo "acceptance: the stand-in printed no URL within ten seconds" >&2; exit 1; }
    SPRAWLING_BIN="$binary" SPRAWLING_PROVIDER="$url" SPRAWLING_LAUNCHER="$launcher" \
        lake exe acceptance walk "$shelf" "$out/script.json" "$out/record.jsonl" "$out/checklist.md"

# The PGO training load (Roadmap P2): what the instrumented release
# binary in `archive` is driven through before its profiles are merged.
# It is the acceptance walk above - a scripted city on the stand-in
# provider: an endpoint attached, a building raised, runs that edit,
# read and ask status, two runs sent at once, a plan divided, a review,
# a kill mid-run and the resume - followed by a city of its own: init,
# adopt, check, a served stretch closed in order, resume, the ledger views, playback
# export over three ranges and its check, and the offline verifier.
#
# LLVM_PROFILE_FILE must name where the profiles land, with `%p` and
# `%m` in it, because several instrumented processes run at once and a
# shared name would let the last one overwrite the rest. A process that
# is killed writes no profile: the walk closes its first and third
# servings in order, with SIGINT on macOS and Linux and through the
# launcher's Ctrl-Break on Windows, so they write theirs; a serving that
# had to be terminated instead is named by a `note` line of the walk
# (tools/adversary/Spec.lean D8). The city of its own is closed in order
# the same way, and the one-shot verbs exit normally. The held-out load below shares no
# step with this one, so a gain it shows is not a gain on the training
# script alone.
# The loopback port the training load serves its city on; one constant,
# moved when a runner already holds it.
pgo_port := "47613"

pgo-train archive:
    #!/usr/bin/env bash
    set -euo pipefail
    case "${LLVM_PROFILE_FILE:-}" in
        *%p*%m* | *%m*%p*) ;;
        *) echo "pgo-train: set LLVM_PROFILE_FILE to a path holding %p and %m, where the instrumented binary writes its profiles" >&2; exit 1 ;;
    esac
    '{{just_executable()}}' --justfile '{{justfile()}}' acceptance '{{archive}}'
    shopt -s nullglob
    binaries=("$PWD"/target/acceptance/archive/*/sprawling.exe)
    [ "${#binaries[@]}" -gt 0 ] || binaries=("$PWD"/target/acceptance/archive/*/sprawling)
    binary="${binaries[0]}"
    city="$PWD/target/pgo-train/city"
    rm -rf "$PWD/target/pgo-train"
    mkdir -p "$city/../adopted/notes"
    echo "a folder the city takes in" > "$city/../adopted/notes/readme.md"
    "$binary" init "$city"
    cp -r "$city/../adopted" "$city/adopted"
    "$binary" adopt "$city" adopted
    "$binary" check "$city"
    # The served city: queries, commands, eight clients at once, the
    # metrics beat, then the orderly close, so this process exits normally
    # and writes its profile. On Windows the city is started through the
    # walk's launcher `serve_grouped` (built by `acceptance` above) in a
    # process group of its own, and the line `close` on the launcher's stdin
    # has it send Ctrl-Break to that group; elsewhere SIGINT is the orderly
    # close (tools/adversary/Spec.lean D8).
    at="127.0.0.1:{{pgo_port}}"
    taken() { grep -q 'commands are taken' "$city/../serve.err" 2>/dev/null; }
    if command -v cygpath >/dev/null 2>&1; then
        targets="${CARGO_TARGET_DIR:-target}"
        case "$targets" in /* | ?:*) ;; *) targets="$PWD/$targets" ;; esac
        exec {closer}> >("$targets/debug/serve_grouped.exe" "$binary" serve "$(cygpath -m "$city")" "$at" --no-open > "$city/../serve.out" 2> "$city/../serve.err")
        server=$!
        stop() { echo close >&"$closer"; exec {closer}>&-; }
    else
        "$binary" serve "$city" "$at" --no-open > "$city/../serve.out" 2> "$city/../serve.err" &
        server=$!
        stop() { kill -INT "$server"; }
    fi
    alive() { kill -0 "$server" 2>/dev/null; }
    for _ in $(seq 1 100); do taken && break; sleep 0.1; done
    taken || { cat "$city/../serve.err" >&2; echo "pgo-train: the served city took no command within ten seconds" >&2; exit 1; }
    ask() { "$binary" call "$1" --at "$at" --quiet-ms 300 > /dev/null 2>&1; }
    for query in city_view endpoint_view preferences metrics known_hosts approval_queue cost_view registry_view governance harnesses toolkits; do
        ask "{\"ask\":{\"ask_id\":1,\"query\":\"$query\"}}"
    done
    ask '{"ask":{"ask_id":1,"query":{"building_view":{"addr":"hall"}}}}'
    clients=()
    for n in 1 2 3 4 5 6 7 8; do
        ask "{\"command\":{\"create_building\":{\"addr\":\"north$n\",\"template\":\"minimal\",\"idem\":\"idem1-0000000000000000000000000000000$n\"}}}" &
        clients+=($!)
    done
    for client in "${clients[@]}"; do wait "$client" || true; done
    "$binary" gauge --at "$at" --every 250 --samples 4 > /dev/null 2>&1 || true
    stop
    for _ in $(seq 1 100); do alive || break; sleep 0.1; done
    if alive; then echo "pgo-train: the served city did not close in order within ten seconds" >&2; exit 1; fi
    grep -q 'closed by the User' "$city"/.sprawling/ledger/*.jsonl || { echo "pgo-train: the served city ended without its handoff, so it wrote no profile" >&2; exit 1; }
    "$binary" resume "$city"
    "$binary" view "$city" > /dev/null
    "$binary" view "$city" --runs > /dev/null
    "$binary" view "$city" --tail 2 --kind building_created > /dev/null
    "$binary" playback export "$city" --out "$city/../whole.json"
    "$binary" playback export "$city" --from 1 --through 2 --out "$city/../narrow.json"
    "$binary" playback export "$city" --building hall --out "$city/../hall.json"
    "$binary" playback check "$city/../whole.json" --city "$city"
    "$binary" replay "$city/.sprawling/ledger"

# The PGO held-out acceptance load (Roadmap P2), which judges an arm and
# trains nothing: the four-action pressure reading and the first byte over
# the empty, l100k and l400k cities (`bench_startup`), then a narrow
# playback export and a tail of the ledger view on the l400k city, each
# timed by the arm's own `gauge` over thirty runs. `arm` names the
# directory under target/pgo-arms the arm is measured from; `binary` is the
# arm's executable, copied there beside a release `bench_startup`, which
# measures the binary beside itself. The fixture cities are kept in
# target/pgo-arms/bench-cities and shared by every arm, so both arms read
# the same bytes. Interleave the arms as END-READING.md orders them.
pgo-heldout arm binary:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build --release -p citysim --bin bench_startup --locked
    targets="${CARGO_TARGET_DIR:-target}"
    case "$targets" in /* | ?:*) ;; *) targets="$PWD/$targets" ;; esac
    suffix=""
    [ -f "$targets/release/bench_startup.exe" ] && suffix=".exe"
    dir="$targets/pgo-arms/{{arm}}"
    mkdir -p "$dir"
    cp "$targets/release/bench_startup$suffix" "$dir/bench_startup$suffix"
    cp '{{binary}}' "$dir/sprawling$suffix"
    sha256sum "$dir/sprawling$suffix"
    "$dir/bench_startup$suffix"
    city="$targets/pgo-arms/bench-cities/l400k"
    out="$dir/exports"
    rm -rf "$out"
    mkdir -p "$out"
    "$dir/sprawling$suffix" gauge --samples 30 -- "$dir/sprawling$suffix" view "$city" --tail 1000 | grep '^{"line":"spread"' | sed 's/^/view_tail_1000 /'
    for n in $(seq 1 30); do
        "$dir/sprawling$suffix" gauge --samples 1 -- "$dir/sprawling$suffix" playback export "$city" --from 1 --through 1000 --out "$out/narrow-$n.json" | grep '^{"line":"run"' | sed 's/^/playback_export_narrow /'
    done

# The acceptance gate for a real endpoint (never a gate in `just
# check`; without credentials it prints one line and succeeds).
#
# It is out of `just check` because it spends somebody's money over
# somebody's network: a key and a reachable endpoint are things a
# machine may legitimately not have, and a daily loop that needs them
# would be a loop people stop running. CI runs it nightly, where the
# credentials live.
#
# The test names the environment variables it reads, so run it once to
# be told them rather than reading a second list here. Which wire the
# endpoint speaks travels as one of those variables beside the base URL
# and the key it belongs to, instead of as a flag on this line that
# could disagree with them.
#
# Two cargo invocations, because the cap belongs to the test process and
# not to the compiler: a cold build of this target takes minutes, so
# `--no-run` pays for compilation first and the 180 seconds then bound
# only the calls to the endpoint.
# Serial, so three network tests share one cap and report in order.
#
# `args` reaches libtest, which is how one case is run on its own.
e2e *args:
    #!/usr/bin/env bash
    set -euo pipefail
    command -v timeout >/dev/null 2>&1 || {
        echo "e2e needs coreutils timeout: the 180-second cap is part of the gate"
        exit 1
    }
    cargo test -p sprawling --test e2e --locked --no-run
    timeout 180 cargo test -p sprawling --test e2e --locked -- \
        --nocapture --test-threads=1 {{args}}
