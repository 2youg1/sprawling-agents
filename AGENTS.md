# AGENTS.md — how work is done in this repository

sprawling is a locally deployed agent-city harness: one Rust binary, a browser client it serves, and a city that keeps its whole history in one append-only Ledger. Most of it was written by models. Every rule below holds for a person too, because the person who picks this up next does not remember yesterday either.

Read this file to the end before the first edit.

## The loop

```bash
cargo install just --locked   # once; a recipe cannot check for the tool that runs it
just prereqs                  # every other tool the loop needs, with the install line for each one that is absent
just check                    # fmt + clippy (-D warnings, --all-features) + two feature-combination checks + the Lean design models + nextest + the client bundle + every machine gate + the client's own checks
```

The `prereqs` recipe reads the only list of the tools the loop needs, the develop tier of the doctor's table, and `just check` opens with it so that a missing tool is named in milliseconds. `build-web` runs inside `check` because `render` and `npm` judge the built client, and refuse when it is absent.

| Command | What it does |
|---|---|
| `just check` | the whole check; *Verification* says when it runs |
| `just check-branch [base]` | the branch check of *Verification* tier 2 on `<base>...HEAD`, running every step even after one fails; a green result is recorded against the tree, and a clean worktree on a recorded tree returns at once |
| `just check-all [phase...]` | every phase of `just check`, each run even after another failed, the chains that share no target at the same time; one log per phase and `phases.tsv` (phase, exit code, seconds) under `$CARGO_TARGET_DIR/check-all`; named phases run alone |
| `just gates` | the machine gates alone, then the supply-chain read |
| `cargo xtask gates <name>...` | the named gates only; `cargo xtask gates --list` prints the roster, one name per line |
| `just commits <base>..<tip>` | every commit subject and `Verdict:` trailer in that range, judged against *Commits* below; CI runs it on the change-set, and `just check` cannot, because a tree has no range |
| `just models` | the Lean design models under `adversary/design/`, built with no `sorry`, `admit` or `axiom`; silent where Lean is absent |
| `just features` | the two feature combinations nothing else compiles: the workspace on its default features, and `channels` without `server` |
| `just check-client` | the client's lint, typecheck and tests |
| `just build-web` | build the client bundle into `target/web-dist` |
| `just dist` | the whole deliverable: client, binary, bill of materials, size badges |
| `just sim` | citysim scenarios: fixed scripts on a counted clock, so a failure replays from the scenario itself |
| `cargo xtask docnum [--write]` | check every number and generated section a document carries against the code that decides it; `--write` rewrites them |
| `just --list` | every other recipe — SPEC skeletons, ledger replay, measurements, fuzzing, mutation testing — with the comment that says what it does |
| `just adversary` | the out-of-tree property checker that attacks the binary through the wire; never a gate, and a no-op without Lean |
| `just proof` | the kernel propositions kani holds against real MIR; never a gate, and a no-op without kani, which has no Windows host. `cargo xtask proof --list` prints the roster it proves |

- Let a Rust command finish, and never kill it by PID; waiting on the build lock is expected.
- Time a slow build with `cargo build --timings` before tuning it, because the seconds sit in particular compilation units.

## Verification

Feedback in seconds keeps each step honest and the whole check takes minutes, so verification runs in three tiers: the first catches a defect in the change, the second a change that breaks its neighbours, the third two branches that are each green and wrong together.

1. **While iterating, run the narrowest command that can fail for the change**, as soon as the change exists: `cargo nextest run -p <crate> -E '<filter>'` naming the new test and the tests of the modules you changed; `cargo clippy -p <crate> --all-targets --all-features -- -D warnings`; `cargo check -p <dependent>` for each direct dependent (`cargo tree --workspace -i <crate> --depth 1`) after a `pub` surface changed; `cargo xtask gates <name>...` naming the gates that judge the files you touched; and `cargo fmt --check`, which compiles nothing. Leave whole suites, the artifact gates (`render`, `budget`, `npm`) and release builds to the next tiers unless the change is about them.
2. **Before a branch merges, run the branch check once**, `just check-branch <base>`: the whole test suites of the crates the branch changed, the tests in their dependents that name what changed, clippy on those crates, `cargo fmt --check`, and every gate that reads sources, adding `render`, `budget` and `npm` after `just build-web` when the branch touched `client/`.
3. **`just check` runs once on the tree that merges a batch of branches into `main`, and `main` advances only when it is green.** The merge runs it as `just check-all`, so one run lists every failure; rerun a red phase alone with `just check-all <phase>`, confirm each fix with the narrowest command that shows it, and then run `just check-all` again.

A green result belongs to the tree it ran on: record it with `git rev-parse HEAD^{tree}`, and rerun it on that tree only when something outside the tree, such as the toolchain, changed.

## Read before you write

- [`ARCHITECTURE.md`](ARCHITECTURE.md) — crate topology, seams, module map, the seven shapes, determinism. Its `depmap` block and module map are machine authorities.
- `crates/<crate>/<crate>-SPEC.md` — that crate's interfaces and decisions, written before its code.
- [`docs/glossary.md`](docs/glossary.md) — one name per concept, enforced by a gate.
- The tests next to the code you are about to change.
- The official documentation of each tool you use, and the vendor's agent guide or skill when one exists.

## One change, five steps

1. **Take one piece of work.** Read its context in full before starting: the crate's SPEC, the neighbouring modules, and their tests.
2. **Write the SPEC first.** Interfaces and decisions land in the crate's SPEC before the code exists. A new module states which of the seven shapes it instantiates ([`ARCHITECTURE.md`](ARCHITECTURE.md) §9); when there is no answer, stop and ask rather than write. A move that leaves every interface unchanged — a file split, a module moved — needs no SPEC section; the module map row is its record.
3. **Red.** Write the failing test and **run it once to watch it fail** on an assertion. That run is what proves the test can bite; a compile error from a missing symbol proves nothing about the defect.
4. **Green.** Implement until it passes, no more. When the implementation wants to differ from the SPEC, change the SPEC first.
5. **Close.** All four: the branch check green | the red-to-green transition visible in the commit order | SPEC and code in step | the module map updated.

The client (`client/`) is exempt from steps 2 and 3 — see *The view layer*.

- Before the code exists, the crate's SPEC carries the shape the module instantiates, its public signatures, the failures it returns with their codes, the values it fixes, and the decision that chose this over the alternative. A SPEC section that only names the module is not step 2.
- Unless the change is mechanical, keep the diff under **800 changed lines**, and under **500** when the logic is not obvious. When it is larger, land the smallest coherent stage that holds on its own and name the remaining stages, splitting along the actual diff and its call sites.

## Rust

- Return failure through `Result`. No `unwrap`, `expect`, `panic!`, `todo!`, `unreachable!`, bare indexing or slicing in non-test code.
- Use checked arithmetic and `TryFrom`. No `as` casts.
- `unsafe_code` is `deny` across the workspace, and the root of every crate without a foreign-function call carries `#![forbid(unsafe_code)]`, which nothing inside the crate can lift. `desktop` is the only crate without that line: there `unsafe` appears only under `platform/windows/`, lifted at the narrowest scope with `#[expect(unsafe_code, reason = "…")]`, and it wraps the FFI call alone, because reasoning pulled inside the block only makes the next reader audit more lines.
- A platform call takes the first of these that works, and the crate's SPEC §12 records which one and why: a safe interface from the standard library or from a crate whose public surface is safe (for example `CommandExt::creation_flags` to set a child's priority class); then a Zig leaf behind a `(ptr, len)` boundary, checked for equivalence against a Rust reference, fuzzed on both sides, its boundary properties proved in Lean, and the one `unsafe` its `extern` call costs named in the SPEC. `unsafe` Rust that calls the platform directly is admitted only where a measurement shows it is the best choice overall.
- Every `unsafe` block carries one `SAFETY:` line giving **the precondition that makes the call sound**. Test the line by asking whether what it says could be false: *"we call `EnumWindows`"* cannot be false, so it is a restatement; *"the callback is an `extern "system" fn` in this module, and the `Vec` behind `lparam` outlives the call with no second alias"* can be false, so it is a precondition.
- Carry every failure to a decision: a `Result` is handled or returned, never bound to `let _ =`, replaced by `unwrap_or_default`, or turned by `.ok()` into an `Option` that drops the reason a caller needed.
- Suppress a lint with `#[expect(reason = "…")]` at the narrowest scope, never `#[allow]`. An `expect` that stops firing fails the build, which is how the suppression cleans itself up.
- Make `match` exhaustive, with no wildcard arms.
- Give a function an enum, a newtype, or a named method where a `bool` or bare `Option` parameter would make a caller write `foo(false)` or `bar(None)`, and prefer a typestate or newtype that makes an invalid state unrepresentable over a runtime check that rejects it.
- Give an error the failed action, the subject, a stable code, and a recovery the caller can act on.
- Values that always travel together are one named value.
- Take the time as a parameter. The single sampling point is `bin::assembly`.
- Use `BTreeMap` on kernel decision paths; keep floats out of ledger payloads; start tasks from the one spawn point.
- Inline `format!` arguments: `format!("{name}")`, not `format!("{}", name)`.

## Where code goes

- One module, one file, semantically named. Register the file in the module map, then create it. Keep `lib.rs` and index files free of logic.
- Keep a function inside 200 lines and 4 parameters, and a file inside 400 production lines; the lines a top-level `#[cfg(test)]` item spans are not counted. `xtask/budgets.toml` pins each older file that exceeds a cap at the length it had when the cap moved: a pinned file may not grow past its pin, and its entry is struck once it fits the cap.
- **Every register in `budgets.toml` is a ratchet.** A number may improve freely and drift only within its slack, so a regression cannot arrive one commit at a time. Record the new reading and its reason beside it, and keep the slack where it is.
- Prefer a new module over growing an existing one. When you extract from a large module, move its tests and type docs with it so the invariants stay next to what owns them.
- Name a module for what it owns; this repository has no `utils`, `helpers` or `common` module.
- Call an API directly rather than through a wrapper that only renames it, and inline a helper that has one caller.
- Default to `pub(crate)`. Declare a `pub` trait only in a file on the seam list.
- Introduce a trait only at a seam that already has a second implementation — a test clock, a counting store, a second adapter. Similar text and hypothetical reuse are not seams.
- **Give every rule, state transition and decision exactly one authority.** Two places that decide the same thing is a defect while they still agree, because nothing holds them together once one changes.
- Finish a migration inside one change-set: move every reader and writer, exercise the production path, then delete the old authority and its adapters.
- Trace callers, data flow, invariants and failure paths before you touch a shared interface.
- `.gitattributes` alone decides line endings (LF on every OS). Give no editor, formatter or script a line-ending setting of its own, because a second rule that disagrees turns a one-line edit into a whole-file diff; `git add --renormalize .` folds such a diff back.

## SPECs and Markdown

- **Write each revision as the document's first draft.** A SPEC states what is true now, not how it became true. The check is one sentence: *a reader opening this file for the first time meets no sentence that only someone who read the previous version can use.*
- Keep the reason a rule has its current shape, which the next change depends on, and leave what the rule used to be to git.
- A decision belongs in the SPEC's **§12 Decisions** as a numbered entry giving the decision, its reason, and the alternative it beat. §1–§11 state the interface as it is: no changelog, no session log, no card number, no "this used to be X".
- An acceptance record states what is verified and by which command, in the present tense, without the sitting, the date, or what that sitting left unverified, which goes stale silently and later reads as a defect.
- **An open question in the tree is a claim about the code.** §3 of a SPEC says what about the interface is undecided and what evidence would settle it; a turn in a conversation — *"awaiting a ruling"* — gives a reader outside nothing to act on.
- **Delete an open question the moment it is answered, and put the answer where it is enforced**, because a question that outlived its answer tells every later reader a settled matter is still open.
- Mark a ruling as a ruling where the authority ladder needs it, since some rules hold because the person chose them. Give the decision and its reason, without the date, the sitting, or the words it arrived in.
- When work stops unfinished, write what is left into the SPEC section it belongs to, as the current state of that interface.
- Write for engineers across many countries and many levels of English. Prefer the concrete word to the abstract one, carry each point in one clause, and put what matters most at the end of the sentence.

## Tests

- Tests use the same doors as production code. To exercise something internal, put a seam on that face and give it a second adapter, or drive it from outside through citysim.
- Test modules may relax lints locally with `#[allow]` on the test module. Production code carries them as written.
- Compare whole objects rather than fields one at a time.
- Test what types and the Lean models leave open, and a regression that has actually occurred; a test of a statically defined value, or a negative test for logic that was removed, restates the code and doubles its maintenance.
- Test a gate on fixtures its tests build; the gate run already judges this repository's tree, and a test that reruns the gate on it only repeats that walk.
- A citysim failure replays byte for byte from its scenario, a fixed script on a counted clock with no random source; when it does not, the defect is the determinism.
- A check that enters through the crates' public faces is Rust, beside the code it judges. A check that enters the way a stranger does — spawning the binary, opening a socket to a served city, speaking the wire from outside — is Lean under `adversary/`, which takes its names from `docs/glossary.md`. `xtask boundary` holds the line.
- **When the adversary finds a defect, the knowledge migrates**: a person writes the failing case as a Rust test under `crates/sprawling/tests/`, and `adversary/`, which quantifies over traces, keeps no copy.
- `just check` on a machine without Lean behaves byte for byte as it does where `adversary/` is absent, so nothing in `just check` depends on it.

## The machine gates

A violation turns the check red with a message naming the rule, the violation, and an alternative. The roster is the array in `xtask/src/gates.rs`, which `cargo xtask gates --list` prints; nothing compares it with this table, so a commit that adds or removes a gate changes both.

| Rule | Held by |
|---|---|
| The Rust rules above: no panics, checked arithmetic, `unsafe` only where a crate root does not forbid it. | workspace lints, `-D warnings`, `#![forbid(unsafe_code)]` |
| Module map registered; functions inside 200 lines and 4 parameters, files inside 400 production lines. | `xtask modmap`, `xtask length` |
| Every dependency a manifest declares named by the code of its package, and every workspace dependency inherited by some package. | `xtask unused` |
| `pub(crate)` by default; `pub` traits only on the seam list. | `xtask depmap` |
| The client's lockfile in step with its manifest, its runtime dependencies exactly the list `RUNTIME` in `xtask/src/npm.rs` names (`svelte`, `effect`, and the `@lezer` highlighter), every licence on the list `deny.toml` permits. | `xtask npm` |
| The MPL-2.0 notice then the copyright line, at the top of every `.rs` file. | `xtask header` |
| A white-box check written in Rust beside the code it judges; a check that enters the way a stranger does written in Lean under `adversary/`. | `xtask boundary` |
| Nothing written for a test compiled into the binary a person downloads. | `xtask artifact` |
| One name per concept, taken from the glossary. | `xtask lexicon` |
| Credentials as `secret:realm/name` references; plaintext reaches the vault and nowhere else. | `xtask secret` |
| Colour taken from the `@theme` block in `client/src/theme.css`, expressed as a ratio of the gamut limit. | `xtask color` |
| Every word a reader is given taken from `client/src/lang.json`. | `xtask wording` |
| Every role, accessible name and landmark a settled screen wrote down, offered by the shipped screen, with no box outside its container. | `xtask render` |
| Every verb the city can carry out reached by some control, or classified on the wire seam with the reason a person may not ask for it. | `xtask wiring` |
| `client/src/wire.ts` regenerated whenever `WIRE_V` moves. | `xtask wire-ts` |
| Every number a document quotes, and every kernel enum table a SPEC carries, equal to the code that decides it. | `xtask docnum` |
| The kani harness roster read out of the `#[kani::proof]` attributes: no workflow names a harness, a stated total is the total, and a harness left unproved cites where that was decided. | `xtask proof` |
| Sizes inside their budget, badges in step with the artifacts. | `xtask budget` |
| Nothing published that names one machine's home directory, its working notes, or a document this tree does not contain. | `xtask release` |

- **Fix the cause when a gate goes red.** Loosening a gate in the change that the gate is failing requires an explicit ruling from the person, recorded as the commit's `Verdict: user-approved` trailer; the wording of the ruling stays with the person, and the trailer records that there was one. Review holds this rule rather than a gate, because a gate that read commit history made every run depend on the range its caller passed.
- Put a change to gate machinery — `xtask/`, `justfile`, `.github/`, the root `Cargo.toml`, `deny.toml`, `clippy.toml`, `rust-toolchain.toml`, `xtask/budgets.toml`, `architecture.toml` — in a commit apart from the source it judges, so review sees whether the gate moved to admit it. Re-pricing a rule in a commit of its own is ordinary work and needs no ruling.
- **Every rule that excludes an architecture carries the parameter that made it right.** When that parameter moves, re-argue the rule instead of obeying it. Rules that exclude a *defect* — the panic bans, the arithmetic bans, the determinism rules — carry no such condition, because nothing about them expires.

## The view layer

`client/` is **not** held to SPEC-first or to red-before-green: what makes an interface good is cheap iteration, and a SPEC and a failing test for every visual change would tax it to buy correctness the view layer was not losing. It is held to colour from `theme.css`, wording from `lang.json`, the size budget, and the eslint rules the client's own config sets (no `any`, no `as`, no `throw`, no `try`, no non-exhaustive switch).

Read [`docs/frontend-method.md`](docs/frontend-method.md) before changing a screen. It is how a screen gets built here: settle it against the shipped stylesheet, add the fixture to `#/gallery`, and accept it with `cargo xtask render`, which measures where its boxes landed in a real engine.

## Language

| Where | Language |
|---|---|
| Identifiers, event names, error codes, rustdoc, commit subjects | English |
| `README.md`, `AGENTS.md`, `ARCHITECTURE.md`, `docs/` | English, except `README.zh-CN.md` and `docs/getting-started.zh-CN.md` |
| Crate SPECs and design discussion | Chinese, with concept names kept in their English form |
| Pull requests, issues, review comments | **your own language.** A parallel translation is welcome, not required: side by side a reader is faster, and a mistranslation is visible instead of silent |

- A comment is one of four kinds: the MPL notice, public interface documentation, a warning about consequences, or a statement of intent the code cannot carry. Any other comment marks code that should say more itself.
- In rustdoc, write what the signature cannot say — invariants, failure modes, call ordering, ownership.

## Privacy

**A working record is noise to a contributor and to a model, and signal to an attacker**: what was tried and abandoned, which machine could not verify what, which account ran out of quota, when somebody is away, what is still unfixed and for how long. A reader skips those sentences; someone looking for a way in reads them closely.

`xtask secret` and `xtask release` scan files in the tree for **shapes** — a credential's prefix, a home directory's spelling. Working context written as prose has no shape, so it passes: *"only a third could be verified on this machine"* is a fact about somebody's hardware that every gate reads as a sentence. **A pull request description, a commit body, an issue and a review comment are not files in the tree, and no gate reaches them.** Both are yours to hold.

- **Ship the decision, not the occasion**: the decision, the reason it beat the alternative, and the parameter that would re-open it, without who said it, when, on whose machine, or in what words.
- Keep your own working record — a session diary, a round-by-round acceptance log, a dated ruling, a quotation of the person's words — in the gitignored `local/`, because it is about a person and ships to strangers inside whatever file carries it. `xtask release` refuses a published document that sends a reader there.
- Cite only what a reader can open; a cited log they cannot open tells them it exists and leaves the document unreadable.
- Report a measurement against the machine class it came from: *"22 s on a warm cache, four cores"*, not *"22 s on this machine"*. A toolchain version belongs in `rust-toolchain.toml` or the doctor's output.
- `the person` as a **domain role** — a city has residents and one person — is outside this section; naming it in an interface, a payload or a rustdoc line is correct.
- Keep credentials, tokens, cookies and signed URLs out of everything you write, including a description of how you tested; nothing redacts it afterwards.
- Name a file by its repo-relative path, such as `crates/kernel/src/secret.rs`; an absolute path spells a home directory and an account name.
- Quote the lines of terminal output that carry the finding; a raw dump brings the prompt, the host name and the account name with it.
- Describe what you found; the diff says what you did. Your conversation, reasoning trace, tool-call log and planning notes stay out, and so do people's names, e-mail addresses, machine names, LAN addresses, and account, project or organisation identifiers; a commit's author identity is the only place a person's name belongs.
- Crop a screenshot to the application: no desktop, task bar, window title or browser tabs.
- When a finding genuinely needs private data to state, describe the shape instead of the value: *"a key-shaped value arrives in the header"*, not the key.
- Read your own description once with this section in hand before you open a pull request; a force-push afterwards does not remove what was already fetched.

## Commits

- The first line is `card-<stage>.<index>: <what>`, for example `card-S4.02: the wire, and the two frames a socket cannot spell`.
- The body records **what you found**, since what you did is already in the diff: a gate that changed your design, a red-to-green transition that exposed a real defect, a choice between two approaches whose reason the result does not show.
- A commit that loosens a gate under the ruling *The machine gates* requires carries the `Verdict: user-approved` trailer.

## Where the authorities are

1. The person's ruling.
2. [`ARCHITECTURE.md`](ARCHITECTURE.md) — structure.
3. `crates/<crate>/<crate>-SPEC.md` — that crate's interfaces and decisions.
4. The code and its tests.

Where a lower level contradicts a higher one, the higher wins and the lower is corrected in the same change. Where reality contradicts all of them, reality wins and the document is corrected first, with its reason.
