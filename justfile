# sprawling daily loop. Contract mirrored in AGENTS.md; keep both in sync.
set shell := ["bash", "-uc"]

default: check

# Every card closes on this being green.
#
# `build-web` sits before `gates` because two of the gates - render and
# npm - judge artifacts rather than sources: they need `target/web-dist`
# and `client/node_modules` to exist, and they skip when those are
# absent. Without this dependency `just check` reported green on a
# machine where neither gate had ever run.
check: prereqs fmt-check clippy features test build-web gates check-client check-desktop

# The one authority on what this repository's loop needs installed.
#
# AGENTS.md, docs/CONTRIBUTING.md and flake.nix all point here instead of
# listing tools themselves: four lists of one fact is how a person
# installed everything named and still met a red `apisync` on the first
# run. Each row is one `command -v`, so the whole recipe costs
# milliseconds and `check` opens with it - a missing tool is named before
# a compile rather than twenty minutes into one.
#
# A required row missing fails this recipe; an optional one is printed
# and passes, because the recipe that needs it says so itself.
# `just prereqs list` prints one `class<TAB>name` line per row, which is
# what flake.nix's `devshell-covers-just-check` compares its devshell
# against, so a tool added here and not there turns that check red.
prereqs mode="check":
    #!/usr/bin/env bash
    set -uo pipefail
    mode='{{mode}}'
    case "$mode" in
        check|list) ;;
        *) echo "prereqs takes 'check' or 'list', not '$mode'" >&2; exit 2 ;;
    esac
    missing=0
    need() {
        class=$1; name=$2; probe=$3; recipe=$4; purpose=$5
        if [ "$mode" = list ]; then
            printf '%s\t%s\n' "$class" "$name"
            return 0
        fi
        if eval "$probe" >/dev/null 2>&1; then
            return 0
        fi
        printf '%-8s %-17s %s\n         install: %s\n' "$class" "$name" "$purpose" "$recipe"
        if [ "$class" = required ]; then
            missing=$((missing + 1))
        fi
        return 0
    }
    need required cargo 'command -v cargo' \
        'rustup from https://rustup.rs; rust-toolchain.toml then pins the version' \
        'every recipe in this file'
    need required rustfmt 'command -v rustfmt' \
        'rustup component add rustfmt' \
        'just fmt-check'
    need required cargo-clippy 'command -v cargo-clippy' \
        'rustup component add clippy' \
        'just clippy, the zero-warning build'
    need required cargo-nextest 'command -v cargo-nextest' \
        'cargo install cargo-nextest --locked' \
        'just test'
    need required bun 'command -v bun' \
        'https://bun.sh' \
        'the client bundle, and the gates that judge artifacts'
    need required cargo-public-api 'command -v cargo-public-api' \
        'cargo install cargo-public-api --locked' \
        'the apisync gate, which fails closed without it'
    need required nightly-rustdoc 'rustup run nightly rustdoc --version' \
        'rustup toolchain install nightly --profile minimal' \
        'the rustdoc JSON cargo-public-api reads'
    need optional cargo-deny 'command -v cargo-deny' \
        'cargo install cargo-deny --locked' \
        'the supply-chain read; CI runs it on every push either way'
    need optional cargo-kani 'command -v cargo-kani' \
        'cargo install --locked kani-verifier, then cargo kani setup' \
        'just proof; kani has no Windows host, where CI proves instead'
    need optional elan 'command -v elan' \
        'https://github.com/leanprover/elan' \
        'just adversary alone, which is never a gate'
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

# --all-features is load-bearing: code behind a feature (runtime/wasm, */conformance)
# escapes the zero-warning gate without it.
clippy:
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

# The feature combinations nothing else compiles. `clippy` above runs
# --all-features, `dist` builds the default set of the release binary
# alone, so two combinations reach no compiler at all: the workspace on
# its default features, and channels with `server` off - which is the
# reason that feature exists, since it keeps the TCP stack out of a
# wasm32 build. Code behind a feature is compiled the day somebody turns
# that feature on, and a combination that does not build is what the
# first person to turn it on meets. Two check-mode passes, seconds each
# on a warm cache; the zero-warning rule stays with `clippy`, which sees
# every feature at once.
features:
    cargo check --workspace --locked
    cargo check -p channels --no-default-features --locked

# `just prereqs` names cargo-nextest; `just test-std` is the fallback.
test:
    cargo nextest run --workspace --locked --all-features

test-std:
    cargo test --workspace --locked

# All machine gates (xtask). cargo-deny runs in CI and here when installed.
gates:
    cargo xtask gates
    @command -v cargo-deny >/dev/null 2>&1 && cargo deny check || echo "cargo-deny not installed locally; CI runs it"

# L-02: `desktop/` is not compiled by any workspace command - the root
# Cargo.toml excludes it so the Win32 boundary can relax `unsafe_code`
# in one place (desktop-SPEC.md section 8.5). Without this recipe its
# source never meets a compiler, a test runner or a licence check that
# this repository runs.
#
# Two things this recipe does not judge are judged by `gates`: the lint
# table and the package metadata are compared back to the root manifest
# by `cargo xtask guard`, because a drifted copy is not something a
# compiler can see.
check-desktop:
    cd desktop && cargo fmt --all --check
    cd desktop && cargo clippy --all-targets --locked -- -D warnings
    cd desktop && cargo nextest run --locked
    @cd desktop && { command -v cargo-deny >/dev/null 2>&1 && cargo deny check || echo "cargo-deny not installed locally; CI runs it"; }

# The browser client (client/client-SPEC.md): Solid + Effect, driven by
# bun, bundled into target/web-dist where crates/sprawling/build.rs reads
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
check-client: build-web
    cd client && bun run lint && bun run typecheck && bun run test

# The gate that opens the gallery in a real engine, on its own: roles,
# accessible names, landmarks, one left edge, nothing outside its box.
# Needs `just build-web` first, and says so when the bundle is absent.
render:
    cargo xtask render

# V5: the kernel propositions kani holds against real MIR. Deliberately
# not in `just check`, and for the same honesty as `adversary`: kani has
# no Windows host, this project is developed on Windows, and a gate that
# always skips on the machine people actually use is the defect this wave
# exists to remove rather than a gate. Without kani installed it prints
# one line and succeeds; CI's linux `proof` job is where it must pass.
# `cargo xtask proof --list` prints the harness names it will run.
proof:
    cargo xtask proof

# citysim scenarios land from S2; the crate's test suite is the entry point.
# No seed argument: the scenarios are fixed scripts driven by a counting
# clock on one thread, so a failure replays from the script rather than
# from a number. A seed returns when a random scenario batch does.
sim:
    cargo test --package citysim --locked

# Generate or refresh a crate SPEC skeleton (apostle-sdd 17 sections + B.5 amendments).
spec crate:
    cargo xtask spec {{crate}}

# Regenerate public-api baselines (`just prereqs` names cargo-public-api and nightly).
api-baseline:
    cargo xtask apisync --write

# Requires cargo-fuzz + nightly; V4 runs the smoke batch nightly in CI.
fuzz target:
    cargo fuzz run {{target}} --fuzz-dir fuzz

# Requires cargo-mutants. The threshold lives in
# xtask/budgets.toml; it is enforced here rather than in `just check` because a
# full mutation run is minutes, and a gate nobody waits for is a gate nobody runs.
mutants:
    cargo mutants --package kernel --minimum-test-timeout 60 --error-value 'kernel::AxError::failure(kernel::AxCode::InvalidArgs, "mutant", "mutant")'

# The performance register: every budget, what it costs today, and what is gated.
budget:
    cargo xtask budget

# The three wall-clock budgets, measured on this machine (never gated).
bench:
    cargo run --release -p citysim --bin bench

# CycloneDX bill of materials -> target/sbom.cdx.json (release item two).
sbom:
    cargo xtask sbom

# Two builds of one tree must be byte-identical (release item three).
repro:
    cargo xtask repro

# The whole deliverable: client bundle first, then the binary that embeds
# it, then the size badges README shows. The badges are rendered from the
# artifacts this recipe just produced, so a release cannot ship a size
# somebody typed.
# An optional target triple builds for a platform other than this
# machine's default; the badges are then left alone, because README's
# sizes describe the artifact a person downloads first and two builds of
# one tag must not disagree about one number.
dist target="": build-web
    cargo build --release -p sprawling --locked {{ if target == "" { "" } else { "--target " + target } }}
    cargo xtask sbom
    {{ if target == "" { "cargo xtask badge --write" } else { "echo the badges belong to the host build" } }}

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

# A11: one session's resident memory, in this platform's own vocabulary.
# Optional argument: a pid to measure instead of the tool itself.
mem pid="":
    cargo xtask mem {{pid}}

# V10: the adversarial property checker in `adversary/`, which lives outside the
# workspace, outside the release, and outside `just check`
# (adversary/adversary-SPEC.md section 2). It is never a gate: on a machine with
# no Lean toolchain this prints one line and succeeds, so `just check` behaves
# exactly as it does where the directory is absent.
#
# This recipe is the only place that knows where the binary is. The checker is
# told through SPRAWLING_BIN and never searches for one, so an adversary run can
# never be driven by a stale binary somebody left in target/.
#
# V10: attack the built binary through the wire (never a gate; skipped without Lean)
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
    cd adversary && SPRAWLING_BIN="$binary" lake exe adversary {{args}}

# N-14.6: the acceptance gate for a real endpoint (never a gate in `just
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
# not to the compiler: a cold build of this target measured 2m18s on the
# development machine, so `--no-run` pays for compilation first and the
# 180 seconds then bound the calls to the endpoint (Roadmap section 0.0).
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
