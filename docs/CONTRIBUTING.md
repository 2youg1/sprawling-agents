# Contributing — how work is done in this repository

> **For someone about to change this code.** It gives the rules, the gate that holds each one, and how to get a green run locally.
>
> It does not explain what the code is made of or how it is verified in layers ([`../ARCHITECTURE.md`](../ARCHITECTURE.md)), nor what the words mean ([`glossary.md`](glossary.md)). The instructions an agent reads first are [`../AGENTS.md`](../AGENTS.md); this file is the longer version of the same rules, and where the two disagree, `AGENTS.md` is corrected or this file is.
>
> Most of this code was written by models, and the rules below hold for whoever comes next: a person typing by hand and a model that does not remember yesterday are held to exactly the same ones. Every rule exists so that a contributor without yesterday's context still produces work that holds.
>
> The rules are stated as the thing to do. Where a rule must never be broken, a machine gate enforces it, and the gate names the fix in its own output, so the first thing you write can be the right thing.

## 0 Thirty seconds

```bash
cargo install just --locked   # once; a recipe cannot check for the tool that runs it
just prereqs                  # every other tool the loop needs, with the install line for each one that is absent
just check                    # the whole check
```

`just check` runs, in this order: `prereqs`; `cargo fmt --check` on the workspace and `zig fmt --check` on the desktop server's Zig leaf; `build-web`, which bundles the client the binary embeds and the artifact gates judge; clippy on every target and feature with `-D warnings`; the two feature combinations nothing else compiles; the test suite under nextest; every machine gate followed by the supply-chain read; the client's lint, typecheck and tests; and, on Windows, the Zig leaf's own tests. "I finished it" is a claim; a green run is the evidence.

## 1 Read before you write

**Read the official documentation of a tool before you use it**, and load the vendor's own agent guide or skill when one exists. This applies to a language feature, a crate, a CLI, a front-end framework and a build system alike. Working from memory of an older version is how a change acquires a defect the compiler cannot see.

The client (`client/`) is **Svelte 5** with Effect, built by bun through Vite. Read <https://svelte.dev/docs/svelte/overview> before touching a component, and [`frontend-method.md`](frontend-method.md) for how a screen is accepted here.

Then read, in order: this file; `ARCHITECTURE.md`, for what the code is made of and why it has this shape; the SPEC of the crate you are touching — `crates/<dir>/Spec.lean` and its parts under `spec/`, or `<lib>-SPEC.md` until that crate migrates — which holds its interfaces and decisions and is written before its code; `docs/glossary.md`, the vocabulary the lexicon gate enforces; and the tests next to the code you are about to touch.

**Everything that explains this code ships with it.** The seam list is a section of `ARCHITECTURE.md`, the module map is `architecture.toml` beside it, and each crate's SPEC sits beside that crate. The only thing kept back is one machine's working notes, which `.gitignore` excludes and which nothing here may depend on.

## 2 The five steps of one change

1. **Take one bounded piece of work**, small enough to finish and verify. Read what it touches in full before you start: the crate's SPEC, the neighbouring modules, and their tests.
2. **Write the SPEC first.** Interfaces and decisions land in the crate's SPEC before the code exists: the shape the module instantiates (one of the seven in `ARCHITECTURE.md` §9), its public signatures, the failures it returns with their codes, the values it fixes, and the decision that chose this over the alternative. When there is no answer, stop and ask rather than write. A move that leaves every interface unchanged — a file split, a module moved — needs no SPEC section; the module map row is its record.
3. **Red.** Write the failing test and **run it once to watch it fail on an assertion**. That run is what tells you the test can bite; a compile error from a missing symbol proves nothing about the defect.
4. **Green.** Implement until it passes, no more. When the implementation wants to differ from the SPEC, change the SPEC first and then continue.
5. **Close.** Four things, all of them: the branch check green | the red-to-green transition visible in the commit order | SPEC and code in step | the module map in `architecture.toml` updated.

The client is exempt from steps 2 and 3; [`frontend-method.md`](frontend-method.md) says what replaces them.

Unless a change is mechanical, keep its diff under 800 changed lines, and under 500 when the logic is not obvious. When it is larger, land the smallest coherent stage that holds on its own and name the stages that remain, splitting along the actual diff and its call sites.

When work stops unfinished, write what is left into the SPEC section it belongs to, as the current state of that interface. A handoff that lives only in a person's head is a handoff that did not happen.

## 3 Three tiers of verification

Feedback in seconds keeps each step honest, and the whole check takes minutes, so verification runs in three tiers.

1. **While iterating, run the narrowest command that can fail for the change**: `cargo nextest run -p <crate> -E '<filter>'` naming the new test and the tests of the modules you changed; `cargo clippy -p <crate> --all-targets --all-features -- -D warnings`; `cargo check -p <dependent>` for each direct dependent (`cargo tree --workspace -i <crate> --depth 1`) after a `pub` surface changed; `cargo xtask gates <name>...` naming the gates that judge the files you touched; and `cargo fmt --check`, which compiles nothing.
2. **Before a branch merges, run the branch check once**: `just check-branch <base>`. It runs the whole test suites of the packages the branch changed, the tests in their dependents that name what changed, clippy, `cargo fmt --check`, every gate that reads sources, and `render`, `budget` and `npm` after `just build-web` when the branch touched `client/`. Every step runs even after another failed, and a green result is recorded against the tree, so a clean worktree on a recorded tree returns at once.
3. **`just check` runs once on the tree that merges a batch of branches into `main`, and `main` advances only when it is green.** The merge runs it as `just check-all`, which runs every phase even after another failed, writes one log per phase and a `phases.tsv` of phase, exit code and seconds, and reruns a named phase alone.

A green result belongs to the tree it ran on: record it with `git rev-parse HEAD^{tree}`, and rerun it on that tree only when something outside the tree, such as the toolchain, changed.

## 4 The rules, and what holds each one

A gate's violation turns the check red with a message naming the rule, the violation, and an alternative. The roster is the array in `tools/xtask/src/gates.rs`, and `cargo xtask gates --list` prints its <!-- xtask:begin gate_count -->24<!-- xtask:end --> names.

| Write it this way | Held by |
|---|---|
| Return failure through `Result`; propagate arithmetic, indexing and conversion errors instead of ending the process. Use `checked_*` arithmetic, `TryFrom` for narrowing conversions, and pattern matching with an explicit fallback for lookups. | workspace lints: `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`, `arithmetic_side_effects`, `as_conversions`, all `deny` |
| Write no `unsafe`: `unsafe_code` is `forbid` across the workspace. One crate relaxes it to `deny`, `crates/desktop/ffi`, the desktop server's FFI seam: its `unsafe` wraps one call into the Zig leaf, lifted at the narrowest scope with `#[expect(unsafe_code, reason = "…")]`, and carries one `SAFETY:` line giving the precondition that makes the call sound. | workspace lints; `crates/desktop/ffi/Cargo.toml` |
| One module, one file, semantically named. Register the file in the module map, then create it. Keep `lib.rs` and index files free of logic. | `xtask modmap` |
| Keep a function inside <!-- xtask:begin budget_figure:function_length.budget_lines -->200<!-- xtask:end --> lines and <!-- xtask:begin budget_figure:argument_count.budget_arguments -->4<!-- xtask:end --> parameters, and a source file inside <!-- xtask:begin budget_figure:file_length.budget_lines -->400<!-- xtask:end --> production lines; a top-level `#[cfg(test)]` item is not counted. The file rule covers the client too, while function length and parameter count are measured in Rust only, because measuring them means parsing the language. A file already over the line is pinned in `tools/xtask/budgets.toml` and may only get smaller. | `xtask length` |
| Default to `pub(crate)`. Declare a `pub` trait only in a file on the seam list. | `xtask depmap` |
| Keep `client/bun.lock` in step with `client/package.json`, keep the runtime dependencies exactly the list `RUNTIME` in `tools/xtask/src/npm.rs` names, and keep every licence in the installed tree on the list `deny.toml` permits. | `xtask npm` |
| Start every `.rs` file with the MPL-2.0 notice, then the copyright line, and carry that head exactly once: a file holding two was assembled from two files, and the documentation between them describes the other one. | `xtask header` |
| Write a white-box check in Rust beside the code it judges, and a check that enters the way a stranger does — spawning the binary, opening a socket, speaking the wire from outside — in Lean under `tools/adversary/`. | `xtask boundary` |
| Compile nothing written for a test into the binary a person downloads. | `xtask artifact` |
| Use one name per concept, taken from `docs/glossary.md`. | `xtask lexicon`, data face `tools/xtask/lexicon.toml` |
| Keep credentials as `secret:realm/name` references; let plaintext reach the vault only. | `xtask secret` |
| Take colour from the `@theme` block in `client/src/theme.css`, expressed as a ratio of the gamut limit. | `xtask color` |
| Take every word a reader is given from `client/src/lang.json`. | `xtask wording` |
| Keep a page's shape: every control named, every landmark named, one first heading, one left edge down the main column, nothing drawn outside the region holding it. The gallery is opened in a real engine and measured, and a missing bundle or engine is a violation that says which. | `xtask render` |
| Reach every verb the city can carry out from some control, or classify it on the wire seam with the reason a person may not ask for it. | `xtask wiring` |
| Regenerate `client/src/wire.ts` whenever `WIRE_V` moves. | `xtask wire-ts` |
| Quote a number in a document only inside a managed span, so the figure is recounted from the code that decides it. | `xtask docnum` |
| Describe every kernel enum in its SPEC table, variant for variant. | `xtask specalign` |
| Let the kani harness roster come from the `#[kani::proof]` attributes: no workflow names a harness, a stated total is the total, and a harness left unproved cites where that was decided. | `xtask proof` |
| Keep sizes inside their budget. | `xtask budget` |
| Publish nothing that names one machine's home directory, its working notes, or a document this tree does not contain. | `xtask release` |
| Keep the one lint table of its own, `crates/desktop/ffi`'s, equal to the workspace's except `unsafe_code`, and let every other crate inherit the workspace's. | `xtask guard` |
| Take the time as a parameter. The single sampling point is `bin::assembly`. | `clippy.toml` disallowed methods |
| Use `BTreeMap` on kernel decision paths; keep floats out of ledger payloads; start tasks from the one spawn point. | review, and the determinism tests in citysim |

**Fix the cause when a gate goes red.** Loosening a gate in the change that the gate is failing requires an explicit ruling from the person, recorded as the commit's `Verdict: user-approved` trailer. Review holds that rule rather than a gate, because a gate that read commit history would make every run depend on the range its caller passed. What review needs to see it is the split: put a change to gate machinery — `tools/xtask/`, `justfile`, `.github/`, the root `Cargo.toml`, `deny.toml`, `clippy.toml`, `rust-toolchain.toml`, `lakefile.toml`, `lean-toolchain`, `tools/xtask/budgets.toml`, `architecture.toml` — in a commit apart from the source it judges. A commit whose whole diff is gate machinery is a re-pricing, which needs no ruling: it says only that a rule now costs something different, and that is exactly what a reviewer has to read.

**A gate that has never failed is indistinguishable from a gate that does not exist.** When you add one, inject a violation and watch it go red once, for the same reason a test must fail before it passes.

**Every register in `budgets.toml` is a ratchet, and the ratchet holds only where a machine measures twice the same way.** A number may improve freely and drift only within its slack, so a regression cannot arrive one commit at a time; record a new reading and its reason beside it, and keep the slack where it is. Sizes are gated, because a byte count does not depend on how busy the machine was. Wall-clock figures live in the same register with what they would need in order to be measured, and are not gated: a slow runner is not a defect, and a gate that says it is teaches people to ignore gates.

**Every rule that excludes an architecture carries the parameter that made it right.** When that parameter moves, re-argue the rule instead of obeying it. Rules that exclude a defect — the panic bans, the arithmetic bans, the determinism rules — carry no such condition, because nothing about them expires.

## 4.1 Continuous integration

**`ci` runs on every push to `main` and on every pull request.** Its verdict jobs together are exactly `just check` plus the supply-chain read and the kernel proofs, so a green CI implies at least what a green `just check` implies. Every verdict job is a required check; the small `changes` job beside them is not, because its only work is to let the proof job skip on a change no proof is about. `platforms` and `nightly` answer questions nobody waits on — macOS, the Nix shell, byte-identical rebuilds, fuzzing, advisories — and run on a schedule; `upstream-watch` asks each watched path of [`third-party.md`](third-party.md) §1 whether it moved, daily.

**From a fork, get the run green before you open the pull request.** A contributor without push access to this repository works in a fork, and the pull request's own run then confirms a result instead of meeting the code for the first time:

1. Fork the repository, then open the fork's Actions tab and enable workflows, which GitHub disables in a new fork.
2. Keep the fork's `main` level with this repository's `main` (*Sync fork* on the fork's page, or `gh repo sync <fork>`). A run started by hand judges commit messages from where the branch left the fork's `main`, so a stale `main` makes the commit check read commits that are not yours.
3. Push the work to a branch of the fork and start the check on it: `gh workflow run ci.yml --repo <fork> --ref <branch>`.
4. Read the result with `gh run list --repo <fork> --workflow ci.yml --branch <branch>`, which shows each run's status and conclusion; `gh run watch <run-id> --repo <fork>` follows one run, and `gh run view <run-id> --repo <fork> --log-failed` prints the steps that failed. Fix a red run on the branch, push, and start it again.
5. Open the pull request once the run is green, and link the run in the description.

`ci.yml` reads no secret, so every one of its jobs runs in a fork; only `main` writes the build caches, so a fork's first run starts cold and takes longer. A check that needs what a fork lacks, such as a workflow that reads a secret, is left to the maintainer's run on the pull request; name that check in the description. The pull request's own run may wait for a maintainer to approve it, which GitHub asks for on a first contribution.

Three things run there and not at your desk: `cargo-deny` when it is not installed locally, the kani proofs (Linux only; `just proof` is a no-op elsewhere), and the nightly fuzz and mutation batches.

## 5 Comments and documentation

**Write the code so that it explains itself, and reach for a comment when it cannot.** A comment is one of four kinds: the MPL notice, public interface documentation, a warning about consequences, or a statement of intent the code cannot carry. Any other comment marks code that should say more itself.

In rustdoc, write what the signature cannot say — invariants, failure modes, call ordering, ownership. The parameter names are already visible.

A SPEC is written as its own first draft: it states what is true now, not how it became true. A decision sits beside what it decides, with its reason and the alternative it beat — a `D<n>` comment above the declaration in a `Spec.lean`, a numbered §12 entry in a Markdown SPEC that has not migrated — and an open question in §3 says what is undecided and what evidence would settle it. `AGENTS.md`, section *SPECs and Markdown*, gives the rest.

### Languages

| Where | Language |
|---|---|
| Identifiers, event names, error codes, rustdoc, commit subjects | English |
| `README.md`, `AGENTS.md`, `ARCHITECTURE.md`, `docs/` | English, except `README.zh-CN.md` and `docs/getting-started.zh-CN.md` |
| Crate SPECs — the Markdown ones, and the comments of `Spec.lean` and its parts — and design discussion | Chinese, with concept names kept in their English form; Lean declaration names are English, from the glossary |
| **Pull requests, issues, review comments** | **your own language** |

That last row is deliberate. Write the description in the language you think in; a precise sentence in your own language is worth more than an approximate one in someone else's. A parallel translation is welcome rather than required — English if you wrote in another language, Chinese if you wrote in English — and with both versions side by side, a mistranslation is visible instead of silent, whether it came from a person or from a model.

Before you open a pull request, read `AGENTS.md`, section *Privacy*: a description, a commit body and a review comment are not files in the tree, and no gate reads them.

## 6 Commit messages

The first line is `card-<stage>.<index>: <what>`, for example `card-S4.02: the wire, and the two frames a socket cannot spell`.

The body records **what you found**, since what you did is already in the diff. Three findings are always worth the space: a gate that changed your design, a red-to-green transition that exposed a real defect, and a choice between two approaches whose reason is not obvious from the result.

`deps:` is its own family and carries no card number: that prefix is what dependabot writes, and a bot does not plan work.

## 7 Tests

- Reach for properties before examples (`proptest`), `insta` for golden output, and `trybuild` when the point is that something cannot be expressed at all.
- **Tests use the same doors as production code.** To exercise something internal, put a seam on that face and give it a second adapter, or drive it from outside through citysim.
- Test modules may relax lints locally with `#[allow]` on the tests module. Production code carries the lints as written.
- Test a gate on fixtures its tests build; the gate run already judges this repository's tree.
- A citysim failure replays byte for byte from its scenario, a fixed script on a counted clock with no random source; when it does not, the defect is the determinism.

One question decides whether a test earns its lines: **would a real defect turn it red?** A test of a statically defined value, or one that only reacts to a rewritten implementation, costs more than it returns.

## 8 Environment

The repository pins the toolchain in `rust-toolchain.toml` and names every other tool in the develop tier of the doctor's table (`crates/sprawling/src/doctor/table.rs`), which is the only list: `sprawling doctor` and the settings page's dependency group install from it, and `just prereqs` reads it through the `prereqs.tsv` the table renders, printing what is absent, what needs it, and the line that installs it. Rows marked *required* are what `just check` cannot run without; rows marked *optional* are skipped by `just check` when absent, or belong to a recipe that says so itself. `nix develop` enters a shell holding those tools, and `nix flake check` refuses a shell that stops short of the list.

`.gitattributes` alone decides line endings: LF on every operating system. Give no editor, formatter or script a line-ending setting of its own; `git add --renormalize .` folds a whole-file diff caused by one back.

**The compile-failure counterexamples are byte comparisons against a compiler's output, so the toolchain installation is part of them.** Installing the `rust-src` component makes rustc render a source snippet inside a `note:` that the committed `.stderr` files do not carry, and every counterexample that meets one goes red without a line of this repository changing. A tool that pulls that component in can turn `just check` red on the next run; remove it (`rustup component remove rust-src`) rather than blessing the longer output, which would only move the failure to CI.

Let a Rust command finish, and never kill it by PID; waiting on the build lock is expected. Time a slow build with `cargo build --timings` before tuning it, because the seconds sit in particular compilation units.

## 9 Command surface

| Command | What it does |
|---|---|
| `just prereqs` | every tool the loop needs, and how to install each one that is absent |
| `just check` | the whole check |
| `just check-branch [base]` | the branch check on `<base>...HEAD` |
| `just check-all [phase...]` | every phase of `just check`, each run even after another failed |
| `just gates` | the machine gates alone, then the supply-chain read |
| `cargo xtask gates <name>...` | the named gates only; `--list` prints the roster |
| `just features` | the workspace on its default features, and `wire` without `server` |
| `just check-client` | the client's lint, typecheck and tests, after `build-web` |
| `just build-web` | build the client bundle into `crates/sprawling/web-dist` |
| `just dist` | the whole deliverable: client, binary, and bill of materials |
| `just budget` / `just bench` | every budget with what it costs today; the wall-clock readings, never gated |
| `just sim` | citysim scenarios: fixed scripts on a counted clock, so a failure replays from the scenario itself |
| `just spec <crate>` | create a crate's `Spec.lean` skeleton |
| `cargo xtask docnum [--write]` | check every managed span against the code that decides it; `--write` rewrites them |
| `just replay <log>` | verify a ledger chain offline, read-only |
| `just mem [pid]` | private, peak private and working set of that process; with no pid, of a fresh empty city served idle (`--city <dir>` serves that one) |
| `just fuzz <target>` / `just mutants` | fuzz targets / mutation testing |
| `just adversary` | the out-of-tree property checker that attacks the binary through the wire; never a gate, and a no-op without Lean |
| `just proof` | the kernel propositions kani holds against real MIR; never a gate, and a no-op without kani |
| `just --list` | every recipe, with the comment that says what it does |
