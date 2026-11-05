# Verification

Only tier 1 runs on the machine the change is written on, because the whole check takes minutes a workstation should not spend compiling. Tier 1 catches a defect in the change, tier 2 a change that breaks its neighbours, tier 3 two branches that are each green and wrong together; tiers 2 and 3 run on GitHub Actions, whose runners also cover the platforms a workstation is not.

## The loop

```bash
cargo install just --locked   # once
just prereqs                  # every other tool the loop needs, with the install line for each absent one
just check                    # the whole check: fmt, gates, Lean specifications, clippy, features, nextest, client
```

`just check` opens with `just prereqs`, which reads the develop tier of the doctor's table (`crates/sprawling/src/doctor/table/prereqs.tsv`), so a missing tool is named at once. `build-web` runs inside `check` because the `render` and `npm` gates judge the built client.

| Command | What it does |
|---|---|
| `just check` | the whole check; *Verification* says when it runs |
| `just check-branch [base]` | the local equivalent of `ci.yml` tier 2 for a change that cannot reach the runners, over `<base>...HEAD`; every step runs even after another fails, and a green result is recorded against the tree |
| `just check-all [phase...]` | every phase of `just check`, each run even after another fails; logs and `phases.tsv` under `$CARGO_TARGET_DIR/check-all`; named phases run alone |
| `just gates` | the machine gates, then the supply-chain read |
| `cargo xtask gates <name>...` | the named gates only; `--list` prints the roster |
| `just commits <base>..<tip>` | commit subjects and `Verdict:` trailers in a range, judged against *Commits* in [AGENTS.md](../AGENTS.md#commits); CI runs it on the change-set, while `just check` cannot, because a tree has no range |
| `just models` | every Lean specification, built with no `sorry`, `admit` or `axiom`; fails where Lean is absent |
| `just fmt` | rustfmt, `zig fmt` and `bun fmt` (width and file list in `bunfig.toml`); `just fmt-check` runs them in check mode |
| `just features` | the two feature combinations nothing else compiles: default features, and `wire` without `server` |
| `just check-client` | the client's lint, `bun check`, typecheck and tests on the `build-web` bundle |
| `just build-web` | the client bundle, into `crates/sprawling/web-dist` |
| `just dist` | client, binary, bill of materials |
| `just sim` | citysim scenarios: fixed scripts on a counted clock, so a failure replays |
| `just provider <script> <record>` | a stand-in provider on a loopback port playing a wire script and recording every exchange; prints `SPRAWLING_PROVIDER=<url>` first |
| `just shots` | every page at 1440 and 1920, dark and light, in `target/shots`; a person reads them, no gate does |
| `cargo xtask docnum [--write]` | every number and generated section a document carries, checked against the code; `--write` rewrites them |
| `just adversary` | the out-of-tree property checker attacking the binary through the wire; never a gate, a no-op without Lean |
| `just proof` | kani propositions on real MIR; never a gate, a no-op without kani (no Windows host); `cargo xtask proof --list` prints the roster |
| `just --list` | every other recipe — SPEC skeletons, ledger replay, measurements, fuzzing, mutation testing — each commented |

- Let a Rust command finish, and never kill it by PID; waiting on the build lock is expected. Time a slow build with `cargo build --timings` before tuning it, because the seconds sit in particular compilation units.

## Tier 1 — while iterating

Run the narrowest command that can fail for the change, as soon as the change exists: `cargo nextest run -p <crate> -E '<filter>'` naming the new test and the tests of the modules you changed; `cargo clippy -p <crate> --all-targets --all-features -- -D warnings`; `cargo check -p <dependent>` for each direct dependent (`cargo tree --workspace -i <crate> --depth 1`) after a `pub` surface changed; `cargo xtask gates <name>...` naming the gates that judge the files you touched; `lake build <module>` for a Lean file you touched; and `cargo fmt --check`, which compiles nothing. A measurement also runs locally, alone on the machine. Whole suites, mutation, fuzzing, kani, the adversary, the screenshots and release builds belong to the next tiers. Code behind `#[cfg(unix)]` or `#[cfg(not(windows))]` is linted only where it compiles, and CI lints Windows on every push but macOS once a night, so before such a change merges run `cargo clippy --workspace --all-targets --all-features --target x86_64-unknown-linux-gnu -- -D warnings` or wait for the macOS job.

## Tier 2 — the branch's CI

Before a branch merges, push it and let `ci.yml` judge it: a pull request starts it, and `gh workflow run ci.yml --ref <branch>` starts it on a branch with none. Nobody waits for the run: whoever picks up the next piece of work reads it first, and a red run is fixed on that branch before anything else lands on top of it. `on-demand.yml` gives the same branch whatever else it needs (*mutants*, *sim*, *shots*, *release*, *pgo*).

A contributor without push access runs tier 2 in a fork before opening the pull request, so its own run confirms a green result instead of being the first time the code meets the check; this is a ruling. **Fork the repository**, enable workflows on the fork's Actions tab (a new fork has them disabled), keep the fork's `main` level with this repository's, because a dispatched run judges commit messages from where the branch left the fork's `main`, and push to a branch of the fork. `gh workflow run ci.yml --repo <fork> --ref <branch>` **starts the run**, `gh run list --repo <fork> --workflow ci.yml --branch <branch>` shows its status, and `gh run view <run-id> --repo <fork> --log-failed` prints the failed steps. Open the pull request once the run is green and link it in the description. `ci.yml` reads no secret, so every job runs in a fork; a check that needs what a fork lacks, such as a workflow that reads a secret, is left to the maintainer's run, and the description names it.

## Tier 3 — the batch

A batch of green branches merges into one branch, `ci.yml` runs on that branch, and `main` fast-forwards only to a batch whose run is green. A red batch gets a fix commit on the batch branch, and the run is dispatched again. `just check` and `just check-all` run the same phases on one machine, for a change that cannot reach the runners; `just check-all` lists every failure in one run, and `just check-all <phase>` reruns a red phase alone.

A green result belongs to the tree it ran on: record it with `git rev-parse HEAD^{tree}` and rerun it only when something outside the tree, such as the toolchain, changed.

## Scheduled and on-demand workflows

Four scheduled workflows judge what a push cannot: `nightly.yml` (advisories, fuzzing), `platforms.yml` (the gates on macOS, the workspace on the oldest compiler `rust-version` names, the Nix flake, a byte-for-byte rebuild), `adversary.yml` (the Lean adversary on a fresh seed) and `upstream-watch.yml`. A red scheduled run is news about the tree as it stands: read it the next morning and fix the cause on `main`. `on-demand.yml`, dispatched with `-f job=<name>`, runs what nobody waits for on a push (mutation of a branch's diff, citysim, the screenshots, a release build of a ref, a profile-guided release build), and the kani autoharness lead runs when `nightly.yml` is dispatched by hand.
