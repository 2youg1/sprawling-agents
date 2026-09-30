# sprawling's daily loop. AGENTS.md says when each recipe runs; this file
# is what it runs.
set shell := ["bash", "-uc"]

default: check

# The whole check, run on the tree that merges a batch of branches into
# `main` (AGENTS.md, Verification tier 3).
#
# `build-web` sits before `gates` because two of the gates - render and
# npm - judge artifacts rather than sources: they need `target/web-dist`
# and `client/node_modules` to exist, and they refuse rather than skip
# when those are absent, so a check that never built them would be red,
# not silently green. CI says the same thing through
# `.github/actions/client-artifacts`, which runs `build-web` for the jobs
# that run those gates. It sits before `clippy` as well, because
# `crates/sprawling/build.rs` embeds the bundle: built after the compile,
# a stale bundle is what clippy and the tests would see.
#
# Formatting of both trees runs right after `prereqs`, because it answers
# in seconds and every later step compiles for minutes: without it an
# unformatted line in `desktop/` would surface only in `check-desktop`,
# the last step, after the whole workspace was built and tested. `just` runs
# a dependency once per invocation, so `check-desktop` finds
# `fmt-check-desktop` already done and does not repeat it. The source
# gates and the Lean models follow for the same reason: each answers in
# seconds, before clippy's minutes.
#
# `ci.yml` runs the same recipes, one job per slice of this line, so a
# green pull request implies exactly this and no less.
check: prereqs fmt-check fmt-check-desktop gates-sources models build-web clippy features test gates check-client check-desktop

# The merge's whole check: the phases of `check`, but every phase runs
# even after another has failed, so one run names every red rather than
# the first. build-web runs first and once, because the compile embeds
# the bundle and the gates read it. The phases that share the workspace
# target run as one chain, and the client and `desktop/` (its own
# target) run beside it, so the run takes as long as its longest chain.
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
    phase fmt-desktop just fmt-check-desktop
    phase build-web just build-web
    { phase clippy just clippy; phase features just features
      phase test just test --no-fail-fast; phase gates just gates; } &
    { phase models just models; } &
    { phase client just client-checks; } &
    { phase desktop just --no-deps check-desktop; } &
    wait
    touch "$out/phases.tsv"
    column -t -s $'\t' "$out/phases.tsv"
    awk -F'\t' '$2 != 0 { red = 1 } END { exit red }' "$out/phases.tsv"

# The branch check (AGENTS.md, Verification tier 2) on <base>...HEAD:
# the guard over the branch's own commits, formatting, clippy on the
# workspace when a package changed, every test of a changed package and
# the tests of its dependents that name an item or module the diff
# touched, every gate that reads sources, and the client, its artifact
# gates and `desktop/` when the branch touched them. Every step runs even
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
    # Which package holds a path is cargo metadata's answer (xtask-SPEC.md
    # section 8-39); the paths go through stdin, since a rename branch
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
    touched desktop && step desktop just check-desktop
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
# (sprawling-SPEC.md section 8-58). This recipe reads that file rather
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

fmt-check:
    cargo fmt --all --check

# `desktop/` is outside the workspace, so `cargo fmt --all` at the root
# does not reach it.
fmt-check-desktop:
    cd desktop && cargo fmt --all --check

# --all-features is load-bearing: code behind a feature (runtime/wasm, */conformance)
# escapes the zero-warning gate without it.
clippy:
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

# The feature combinations this repository checks: the workspace on its
# default features, test targets included, and `wire` with `server`
# off - which is the reason that feature exists, since it keeps the TCP
# stack out of a wasm32 build. Code behind a feature is compiled the day
# somebody turns that feature on, and a combination that does not build
# is what the first person to turn it on meets.
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
# Two check-mode passes, seconds each on a warm cache; the zero-warning
# rule stays with `clippy`, which sees every feature at once.
features:
    cargo check --workspace --locked --all-targets
    cargo check -p sprawling-wire --no-default-features --locked

# `just prereqs` names cargo-nextest; `just test-std` is the fallback.
[positional-arguments]
test *args:
    cargo nextest run --workspace --locked --all-features "$@"

test-std:
    cargo test --workspace --locked

# The three gates that judge a built artifact rather than a source:
# `render` opens `target/web-dist`, `npm` reads `client/node_modules`,
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
# Lean"), and the design models under `tools/adversary/design/` until they
# reach their crates. `lakefile.toml` sets `warningAsError`, so a `sorry` or
# an `admit`, which Lean reports as a warning, fails the build; an `axiom`
# raises no warning at all, so it is refused here by its shape, since no
# specification in this tree has an axiom anybody reviewed. The checker
# under `tools/adversary/` is not built here: it attacks the binary and
# stays out of every required check.
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
    if grep -rnE --include='*.lean' '^[[:space:]]*(private[[:space:]]+)?axiom[[:space:]]' crates tools/adversary/design; then
        echo "models: an axiom above is a proof obligation nobody discharged; prove it as a theorem"
        exit 1
    fi
    lake build Spec Design

# Every commit subject and ruling trailer in a range of history
# (xtask-SPEC.md section 8-35). Not in `check`, because a tree has no
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
#
# The optional directory is for a tree outside the workspace: `desktop/`
# carries its own manifest and its own deny.toml, and it is read with the
# same list of checks rather than with a second spelling of it.
deny dir=".":
    #!/usr/bin/env bash
    set -euo pipefail
    if ! command -v cargo-deny >/dev/null 2>&1; then
        echo "cargo-deny not installed locally; CI runs it on every push"
        exit 0
    fi
    cd '{{dir}}'
    cargo deny check bans licenses sources

# `desktop/` is not compiled by any workspace command - the root
# Cargo.toml excludes it so the Win32 boundary can relax `unsafe_code`
# in one place (desktop-SPEC.md section 8.5). Without this recipe its
# source never meets a compiler, a test runner or a licence check that
# this repository runs.
#
# Two things this recipe does not judge are judged by `gates`: the lint
# table and the package metadata are compared back to the root manifest
# by `cargo xtask guard`, because a drifted copy is not something a
# compiler can see.
check-desktop: fmt-check-desktop
    cd desktop && cargo clippy --all-targets --locked -- -D warnings
    cd desktop && cargo nextest run --locked
    just deny desktop

# The browser client (client/client-SPEC.md): Svelte + Effect, built by
# Vite under bun, bundled into target/web-dist where crates/sprawling/build.rs reads
# it. `just prereqs` names bun. `--frozen-lockfile` makes bun.lock
# the authority, so a build cannot resolve a version nobody committed.
build-web:
    cd client && bun install --frozen-lockfile && bun run build

# The client's own gates: lint (no `any`, no `as`, no throw, no try, no
# non-exhaustive switch), typecheck, and its tests.
#
# `build-web` owns the dependency install, so this recipe depends on it
# instead of holding a second `bun install` line that could drift from
# it. `just` runs a dependency once per invocation, so `just check`
# installs and bundles exactly once even though both recipes are in it.
check-client: build-web client-checks

# The client's own gates on the dependencies build-web installed.
client-checks:
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

# Generate or refresh a crate SPEC skeleton.
spec crate:
    cargo xtask spec {{crate}}

# Regenerate the public-api baselines with the renderer
# `tools/xtask/public-api.txt` pins; `cargo xtask apisync` names the
# install line for whichever half is missing.
api-baseline:
    cargo xtask apisync --write

# Requires cargo-fuzz + nightly; `nightly.yml` runs the smoke batch.
fuzz target:
    cargo fuzz run {{target}} --fuzz-dir tools/fuzz

# Requires cargo-mutants. The threshold lives in
# tools/xtask/budgets.toml; it is enforced here rather than in `just check` because a
# full mutation run is minutes, and a gate nobody waits for is a gate nobody runs.
mutants:
    cargo mutants --package sprawling-kernel --minimum-test-timeout 60 --error-value 'kernel::AxError::failure(kernel::AxCode::InvalidArgs, "mutant", "mutant")'

# The performance register: every budget, what it costs today, and what is gated.
budget:
    cargo xtask budget

# The wall-clock readings, never gated: citysim's load scenarios, then
# the two instruments that drive the city's own accounting loop - a relay
# round trip and the gap a second dispatch leaves in a running one
# (sprawling-SPEC.md 8-84).
bench:
    cargo run --release -p citysim --bin bench
    cargo nextest run -p sprawling --release --run-ignored only -E 'test(/::instrument_/)' --no-capture

# The four-action pressure reading (citysim-SPEC.md 8-5) - install,
# startup, raise a city, open a session - measured, never gated.
#
# `build-web` first, the same dependency `dist` carries: without the
# bundle the binary embeds a placeholder page, and a placeholder is not
# the artifact a person installs - its weight is what the install action
# unpacks and the startup action loads.
#
# The shipped binary is built next and the bench measures the one that
# lands beside its own executable, so no run can be driven by a stale
# artifact. This recipe is where that ordering lives.
bench-startup: build-web
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
# **The engine is part of what ships.** `runtime/wasm` is off by default,
# so a plain release build carries no execution engine and every `python`
# call an archive's city makes would be refused; `cargo xtask package`
# refuses exactly that binary, which is how a release built without this
# flag fails at the last step instead of publishing a crippled one. The
# size this produces is the size a person downloads.
dist target="": build-web
    cargo build --release -p sprawling --features sandbox --locked {{ if target == "" { "" } else { "--target " + target } }}
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
# (xtask-SPEC.md section 8-30). A pid reads that process. Anything else
# serves a city - a fresh empty one, or `--city <dir>` - so the release
# binary is built first: a reading of a stale binary describes a tree
# nobody has.
mem *args:
    {{ if args =~ '^[0-9]+$' { "true" } else { "cargo build --release -p sprawling --locked" } }}
    cargo xtask mem {{args}}

# The adversarial property checker in `tools/adversary/`, which lives outside the
# workspace, outside the release, and outside `just check`
# (tools/adversary/adversary-SPEC.md section 2). It is never a gate: on a machine with
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
    binary="$PWD/target/debug/sprawling"
    [ -f "$binary" ] || binary="$binary.exe"
    # The path is handed to a program that is not a shell, so it has to be one
    # the operating system can open. Under Git Bash `$PWD` is `/c/...`, which
    # nothing outside that shell resolves; `cygpath -m` turns it back into
    # `C:/...` and is absent everywhere it is not needed.
    ! command -v cygpath >/dev/null 2>&1 || binary="$(cygpath -m "$binary")"
    SPRAWLING_BIN="$binary" lake exe adversary {{args}}

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
