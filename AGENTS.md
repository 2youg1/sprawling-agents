# AGENTS.md — how work is done in this repository

<context>

sprawling is a locally deployed harness that runs many agents on one machine as a city: one Rust binary, a browser client that the binary serves, and one append-only Ledger that holds the whole history of the city. Models wrote most of the code, and every rule below holds for a person as well, because whoever picks the work up next does not remember what happened yesterday.

The authorities, from highest to lowest, are the person's ruling, [`ARCHITECTURE.md`](ARCHITECTURE.md) for structure, a crate's SPEC — `crates/<dir>/Spec.lean` with its parts under `spec/`, or `<lib>-SPEC.md` until the crate migrates — for its interfaces and decisions, and then the code and its tests. When a lower level contradicts a higher one, the higher one wins and the lower one is corrected in the same change. When reality contradicts all of them, reality wins, and the document is corrected first, with the reason written beside the correction.

</context>

<reading>

## Read before you write

Read this file to the end before the first edit, and then read what the change touches:

- [`ARCHITECTURE.md`](ARCHITECTURE.md) — the crate topology, the seams, the module map, the seven shapes and the determinism rules. Its `depmap` block and its module map are machine authorities.
- A crate's SPEC — `crates/<dir>/Spec.lean` with its parts under `spec/` once the crate has migrated, `crates/<dir>/<lib>-SPEC.md` until then — holds the crate's interfaces and decisions, written before its code; [`ARCHITECTURE.md`](ARCHITECTURE.md) §11, *Specifications in Lean*, says where each part lives.
- [`docs/glossary.md`](docs/glossary.md) — one name for each concept, enforced by the `lexicon` gate through `tools/xtask/lexicon.toml`.
- The tests beside the code you are about to change, and the neighbouring modules.
- The official documentation of each tool you use, and the vendor's agent guide or skill when one exists.

</reading>

<loop>

## The loop

```bash
cargo install just --locked   # once; a recipe cannot check for the tool that runs it
just prereqs                  # every other tool the loop needs, with the install line for each one that is absent
just check                    # the whole check: fmt, source gates, Lean specifications, clippy, feature combinations, nextest, the client bundle, artifact gates, the client's own checks
```

`just prereqs` reads the one list of tools the loop needs, the develop tier of the doctor's table (`crates/sprawling/src/doctor/table/prereqs.tsv`), and `just check` opens with it, so a missing tool is named in milliseconds rather than twenty minutes into a compile. `build-web` runs inside `check` because the `render` and `npm` gates judge the built client and refuse when it is absent.

| Command | What it does |
|---|---|
| `just check` | the whole check; *Verification* says when it runs |
| `just check-branch [base]` | the branch check of *Verification* tier 2 on `<base>...HEAD`, running every step even after one fails; a green result is recorded against the tree, and a clean worktree on a recorded tree returns at once |
| `just check-all [phase...]` | every phase of `just check`, each run even after another failed, the chains that share no target at the same time; one log per phase and `phases.tsv` (phase, exit code, seconds) under `$CARGO_TARGET_DIR/check-all`; named phases run alone |
| `just gates` | the machine gates, then the supply-chain read |
| `cargo xtask gates <name>...` | the named gates only; `cargo xtask gates --list` prints the roster, one name per line |
| `just commits <base>..<tip>` | every commit subject and `Verdict:` trailer in the range, judged against *Commits*; CI runs it on the change-set, and `just check` cannot, because a tree has no range |
| `just models` | every Lean specification under `crates/`, built with no `sorry`, `admit` or `axiom`; fails where Lean is absent |
| `just features` | the two feature combinations nothing else compiles: the workspace on its default features, and `wire` without `server` |
| `just check-client` | the client's lint, typecheck and tests |
| `just build-web` | the client bundle, built into `crates/sprawling/web-dist` |
| `just dist` | the whole deliverable: client, binary, bill of materials |
| `just sim` | the citysim scenarios: fixed scripts on a counted clock, so a failure replays from the scenario itself |
| `just provider <script> <record>` | the stand-in provider: a wire script played on a loopback port, every exchange recorded, `SPRAWLING_PROVIDER=<url>` printed first |
| `just shots` | every page at 1440 and 1920, dark and light, as PNG files and an index in `target/shots`; a person reads them, no gate does |
| `cargo xtask docnum [--write]` | every number and generated section a document carries, checked against the code that decides it; `--write` rewrites them |
| `cargo xtask apisync [--write]` | the public API of `kernel` and `wire` against its baseline in `tools/xtask/api-baselines/`, rendered by the nightly rustdoc and the cargo-public-api release that `tools/xtask/public-api.txt` pins; a nightly job reads it, and a change to either surface rewrites the baseline in the same change-set |
| `just adversary` | the out-of-tree property checker that attacks the binary through the wire; never a gate, and a no-op without Lean |
| `just proof` | the kernel propositions kani holds against real MIR; never a gate, and a no-op without kani, which has no Windows host; `cargo xtask proof --list` prints the roster |
| `just --list` | every other recipe — SPEC skeletons, ledger replay, measurements, fuzzing, mutation testing — with the comment that says what it does |

- Let a Rust command finish, and never kill it by PID; waiting on the build lock is expected.
- Time a slow build with `cargo build --timings` before tuning it, because the seconds sit in particular compilation units.

</loop>

<rules>

## Rust

- Return failure through `Result`. Non-test code has no `unwrap`, `expect`, `panic!`, `todo!`, `unreachable!`, bare indexing or bare slicing.
- Use checked arithmetic and `TryFrom` in place of `as` casts.
- `unsafe_code` is `forbid` in the workspace lint table (`[workspace.lints.rust]` in the root `Cargo.toml`), which every workspace crate inherits and nothing inside a crate can lift. One crate writes a table of its own for exactly this reason: `crates/desktop/ffi`, the FFI seam of the desktop server, carries the workspace's table with `unsafe_code = "deny"`. There `unsafe` appears only around a call into the Zig leaf, lifted at the narrowest scope with `#[expect(unsafe_code, reason = "…")]`, and it wraps the FFI call alone, because reasoning pulled inside the block only makes the next reader audit more lines.
- A platform call takes the first of these that works, and the crate's SPEC records which one and why as a decision: a safe interface from the standard library or from a crate whose public surface is safe (for example `CommandExt::creation_flags` to set a child's priority class); then a Zig leaf behind a `(ptr, len)` boundary, checked for equivalence against a Rust reference, fuzzed on both sides, its boundary properties proved in Lean, and the one `unsafe` its `extern` call costs named in the SPEC. `unsafe` Rust that calls the platform directly is admitted only where a measurement shows it is the best choice overall.
- Every `unsafe` block carries one `SAFETY:` line that gives the precondition that makes the call sound. Test the line by asking whether what it says could be false: *"we call `EnumWindows`"* cannot be false, so it restates the code; *"the callback is an `extern "system" fn` in this module, and the `Vec` behind `lparam` outlives the call with no second alias"* can be false, so it is a precondition.
- Carry every failure to a decision: a `Result` is handled or returned. Binding it to `let _ =`, replacing it with `unwrap_or_default`, or turning it into an `Option` with `.ok()` drops the reason a caller needed.
- Suppress a lint with `#[expect(reason = "…")]` at the narrowest scope. An `expect` that stops firing fails the build, which is how the suppression cleans itself up; `#[allow]` belongs only on test modules.
- Write every `match` exhaustively, with no wildcard arm.
- Give a function an enum, a newtype or a named method where a `bool` or bare `Option` parameter would make a caller write `foo(false)` or `bar(None)`, and prefer a typestate or newtype that makes an invalid state unrepresentable over a runtime check that rejects it.
- Give an error the failed action, the subject, a stable code, and a recovery the caller can act on.
- Values that always travel together are one named value.
- Take the time as a parameter; the single sampling point is `bin::assembly`.
- Use `BTreeMap` on kernel decision paths, keep floats out of ledger payloads, and start tasks from the one spawn point.
- Inline `format!` arguments: `format!("{name}")`.
- Test code that compiles only on one platform is linted only where it compiles, and CI runs clippy on Windows for every push and on macOS once a night. Before a change to `#[cfg(unix)]` or `#[cfg(not(windows))]` code merges, run `cargo clippy --workspace --all-targets --all-features --target x86_64-unknown-linux-gnu -- -D warnings` or wait for the macOS job, because a lint there otherwise surfaces a day later.

## Where code goes

- One module is one file, named for what it owns. Register the file in the module map, then create it. Keep `lib.rs` and index files free of logic, and give this repository no `utils`, `helpers` or `common` module.
- Keep a function within 200 lines and 4 parameters, and a file within 400 production lines; the lines a top-level `#[cfg(test)]` item spans are not counted. `tools/xtask/budgets.toml` pins each older file that exceeds a cap at the length it had when the cap moved: a pinned file may not grow past its pin, and its entry is struck once it fits the cap.
- Every register in `budgets.toml` is a ratchet. A number may improve freely and drift only within its slack, so a regression cannot arrive one commit at a time. Record the new reading and its reason beside it, and keep the slack where it is.
- Prefer a new module over growing an existing one. When you extract from a large module, move its tests and type docs with it, so the invariants stay next to what owns them.
- Call an API directly rather than through a wrapper that only renames it, and inline a helper that has one caller.
- Default to `pub(crate)`. Declare a `pub` trait only in a file on the seam list.
- Introduce a trait only at a seam that already has a second implementation — a test clock, a counting store, a second adapter. Similar text and hypothetical reuse are not seams.
- Give every rule, state transition and decision exactly one authority. Two places that decide the same thing are a defect while they still agree, because nothing holds them together once one of them changes.
- Finish a migration inside one change-set: move every reader and writer, exercise the production path, then delete the old authority and its adapters.
- Trace callers, data flow, invariants and failure paths before you touch a shared interface.
- `.gitattributes` alone decides line endings (LF on every OS). Give no editor, formatter or script a line-ending setting of its own, because a second rule that disagrees turns a one-line edit into a whole-file diff; `git add --renormalize .` folds such a diff back.

## SPECs and Markdown

- Write each revision as the document's first draft. A SPEC states what is true now, and the check is one sentence: *a reader opening this file for the first time meets no sentence that only someone who read the previous version can use.*
- Keep the reason a rule has its current shape, which the next change depends on, and leave what the rule used to be to git.
- A decision is recorded where the crate's SPEC keeps decisions, with its reason and the alternative it beat: in a `Spec.lean` or one of its parts, a comment that opens with `D<n>` directly above the declaration it governs (`ARCHITECTURE.md` §11, *Specifications in Lean*); in a `<lib>-SPEC.md` that has not migrated, a numbered entry of **§12 Decisions**. The rest of a SPEC states the interface as it is, with no changelog, session log, card number or "this used to be X".
- An acceptance record states what is verified and by which command, in the present tense, without the sitting, the date, or what that sitting left unverified, which goes stale silently and later reads as a defect.
- An open question in the tree is a claim about the code. §3 of a SPEC says what about the interface is undecided and what evidence would settle it; a turn in a conversation such as *"awaiting a ruling"* gives a reader outside nothing to act on.
- Delete an open question the moment it is answered, and put the answer where it is enforced, because a question that outlived its answer tells every later reader that a settled matter is still open.
- Mark a ruling as a ruling where the authority ladder needs it, since some rules hold because the person chose them. Give the decision and its reason, without the date, the sitting, or the words it arrived in.
- When work stops unfinished, write what is left into the SPEC section it belongs to, as the current state of that interface.
- Write for engineers across many countries and many levels of English: prefer the concrete word to the abstract one, carry each point in one clause, and put what matters most at the end of the sentence.

## Tests

- Tests use the same doors as production code. To exercise something internal, put a seam on that face and give it a second adapter, or drive it from outside through citysim.
- Test modules may relax lints locally with `#[allow]` on the test module; production code carries them as written.
- Compare whole objects rather than fields one at a time.
- Test what types and the Lean models leave open, and a regression that has actually occurred. A test of a statically defined value, or a negative test for logic that was removed, restates the code and doubles its maintenance.
- Test a gate on fixtures its tests build. The gate run already judges this repository's tree, and a test that reruns the gate on it only repeats that walk.
- A citysim failure replays byte for byte from its scenario, a fixed script on a counted clock with no random source; when it does not, the defect is the determinism.
- A check that enters through the crates' public faces is Rust, beside the code it judges. A check that enters the way a stranger does — spawning the binary, opening a socket to a served city, speaking the wire from outside — is Lean under `tools/adversary/`, which takes its names from `docs/glossary.md`. `xtask boundary` holds the line.
- When the adversary finds a defect, a person writes the failing case as a Rust test under `crates/sprawling/tests/`, and `tools/adversary/`, which quantifies over traces, keeps no copy.
- Lean under a crate's `spec/` is neither kind of check: it is that crate's specification, it proves what a module must hold on every input its model admits, and it imports nothing from `tools/adversary/`, which imports no specification in turn.
- A change to the wire — a frame renamed, a field made nonzero, an exit code moved — updates `tools/adversary/src/Sprawling/Door.lean` and the renderer in `tools/adversary/src/Sprawling/Regression.lean` in the same change-set, and `just adversary` confirms it where Lean is installed. The nightly adversary run is otherwise the first place the drift shows.
- `just check` reads nothing under `tools/adversary/`: deleting the directory changes no step of it, and `just adversary`, which is never a gate, prints one line and succeeds where Lean is absent.

## The machine gates

A violation turns the check red with a message that names the rule, the violation, and an alternative. The roster is the array in `tools/xtask/src/gates.rs`, which `cargo xtask gates --list` prints; nothing compares that array with this table, so a commit that adds or removes a gate changes both.

| Rule | Held by |
|---|---|
| The Rust rules above: no panics, checked arithmetic, `unsafe` only in `crates/desktop/ffi`. | workspace lints, `-D warnings` |
| The MPL-2.0 notice, then the copyright line, at the top of every `.rs` file. | `xtask header` |
| One name per concept, taken from the glossary. | `xtask lexicon` |
| Module map registered; functions within 200 lines and 4 parameters, files within 400 production lines. | `xtask modmap`, `xtask length` |
| A white-box check written in Rust beside the code it judges; a check that enters the way a stranger does written in Lean under `tools/adversary/`. | `xtask boundary` |
| Nothing written for a test compiled into the binary a person downloads. | `xtask artifact` |
| Every file a publishable package compiles in production, through `include!`, `include_str!` or `include_bytes!`, inside that package's own directory or its build script's `OUT_DIR`, because crates.io carries the package directory and nothing else. | `xtask packaged` |
| `pub(crate)` by default; `pub` traits only on the seam list. | `xtask depmap` |
| Every dependency a manifest declares named by the code of its package, and every workspace dependency inherited by some package. | `xtask unused` |
| The client's lockfile in step with its manifest, its runtime dependencies exactly the list `RUNTIME` in `tools/xtask/src/npm.rs` names (`svelte`, `effect`, and the `@lezer` highlighter), every licence on the list `deny.toml` permits. | `xtask npm` |
| Credentials as `secret:realm/name` references; plaintext reaches the vault and nowhere else. | `xtask secret` |
| Colour taken from the `@theme` block in `client/src/theme.css`, expressed as a ratio of the gamut limit, and text on glass legible over the brightest surface behind it. | `xtask color` |
| A transition's curve and duration taken from the tokens in `client/src/theme.css`, never spelled in a view. | `xtask motion` |
| Every word a reader is given taken from `client/src/lang.json`. | `xtask wording` |
| Every role, accessible name and landmark a settled screen wrote down, offered by the shipped screen, with no box outside its container; no fixed bar over the conversation, and its standing controls within `talk_controls`. | `xtask render` |
| Every verb the city can carry out reached by some control, or classified on the wire seam with the reason a person may not ask for it. | `xtask wiring` |
| `client/src/wire.ts` regenerated whenever `WIRE_V` moves. | `xtask wire-ts` |
| Every number a document quotes, and every kernel enum table a SPEC carries, equal to the code that decides it. | `xtask docnum` |
| The kani harness roster read out of the `#[kani::proof]` attributes: no workflow names a harness, a stated total is the total, and a harness left unproved cites where that was decided. | `xtask proof` |
| Sizes inside their budget. | `xtask budget` |
| Kernel enums and the tables and rosters of `crates/kernel/Spec.lean` agree variant by variant, and every module's SPEC anchor resolves. | `xtask specalign` |
| One effective specification per crate, no prose naming a SPEC the tree lacks, the checker and the specifications importing along the crate graph and never each other, no `sorry`, `admit` or `axiom` in any `.lean`, and every path a specification cites on disk. `cargo xtask spec <lib>` is the command that writes a skeleton; the gate of the same name only judges. | `xtask spec` |
| Nothing published that names one machine's home directory, its working notes, or a document this tree does not contain. | `xtask release` |
| The one lint table of its own, `crates/desktop/ffi`'s, equal to the workspace's except `unsafe_code`, and every other crate inheriting the workspace's; every workspace package pinned in `[workspace.dependencies]` to `=` the `[workspace.package] version`, and named by no member through a path of its own. | `xtask guard` |

- Fix the cause when a gate goes red. Loosening a gate in the change the gate is failing requires an explicit ruling from the person, recorded as the commit's `Verdict: user-approved` trailer; the wording of the ruling stays with the person, and the trailer records that there was one. Review holds this rule rather than a gate, because a gate that read commit history made every run depend on the range its caller passed.
- Put a change to gate machinery — `tools/xtask/`, `justfile`, `.github/`, `flake.nix`, the root `Cargo.toml`, `deny.toml`, `clippy.toml`, `rust-toolchain.toml`, `lakefile.toml`, `lean-toolchain`, `tools/xtask/budgets.toml`, `architecture.toml` — in a commit apart from the source it judges, so review sees whether the gate moved to admit it. Re-pricing a rule in a commit of its own is ordinary work and needs no ruling.
- Every rule that excludes an architecture carries the parameter that made it right, and when that parameter moves, re-argue the rule instead of obeying it. Rules that exclude a defect — the panic bans, the arithmetic bans, the determinism rules — carry no such condition, because nothing about them expires.

## The view layer

`client/` is held to colour from `theme.css`, wording from `lang.json`, the size budget, and the eslint rules the client's own config sets (no `any`, no `as`, no `throw`, no `try`, no non-exhaustive switch). It is exempt from SPEC-first and from red-before-green, because cheap iteration is what makes an interface good, and a SPEC and a failing test for every visual change would tax that iteration to buy correctness the view layer was not losing. The interaction contract of each part — its ARIA pattern, its keys, where focus returns — is the half that is not exempt, and `client/Spec.lean` holds it: section 9 names the parts under `client/spec/` where each contract is a state machine with its proofs.

Read [`docs/frontend-method.md`](docs/frontend-method.md) before you change a screen. It says how a screen is built here: settled against the shipped stylesheet, given a fixture on `#/gallery`, and accepted by `cargo xtask render`, which measures where the boxes landed in a real engine. Its last part holds the design as approved — the grid, the tokens, the surfaces and the shell's parts — under the labels the code cites, so how a screen looks is described there and nowhere else.

## Language

| Where | Language |
|---|---|
| Identifiers, event names, error codes, rustdoc, commit subjects | English |
| `README.md`, `AGENTS.md`, `ARCHITECTURE.md`, `docs/` | English, except `README.zh-CN.md` and `docs/getting-started.zh-CN.md`, which change in the same commit as their English pair |
| Crate SPECs — the Markdown ones, and the comments of `Spec.lean` and its parts — and design discussion | Chinese, with concept names kept in their English form; Lean declaration names are English, from the glossary |
| Pull requests, issues, review comments | your own language; a parallel translation is welcome, because side by side a reader is faster and a mistranslation is visible instead of silent |

- A comment is one of four kinds: the MPL notice, public interface documentation, a warning about consequences, or a statement of intent the code cannot carry. Any other comment marks code that should say more itself.
- In rustdoc, write what the signature cannot say: invariants, failure modes, call ordering, ownership.

## Privacy

A working record is noise to a contributor and to a model, and signal to an attacker: what was tried and abandoned, which machine could not verify what, which account ran out of quota, when somebody is away, what is still unfixed and for how long. A reader skips those sentences, and someone looking for a way in reads them closely.

`xtask secret` and `xtask release` scan the files in the tree for shapes — a credential's prefix, the spelling of a home directory. Working context written as prose has no shape, so it passes: *"only a third could be verified on this machine"* is a fact about somebody's hardware that every gate reads as a sentence. A pull request description, a commit body, an issue and a review comment are not files in the tree, and no gate reaches them, so they are yours to hold.

- Ship the decision, not the occasion: the decision, the reason it beat the alternative, and the parameter that would re-open it, without who said it, when, on whose machine, or in what words.
- Keep your own working record — a session diary, a round-by-round acceptance log, a dated ruling, a quotation of the person's words — in the gitignored `local/`, because it is about a person and ships to strangers inside whatever file carries it. `xtask release` refuses a published document that sends a reader there.
- Cite only what a reader can open; a cited log they cannot open tells them it exists and leaves the document unreadable.
- Report a measurement against the machine class it came from — *"22 s on a warm cache, four cores"* — and keep a toolchain version in `rust-toolchain.toml` or the doctor's output.
- *The person* as a domain role — a city has residents and one person — is outside this section; naming it in an interface, a payload or a rustdoc line is correct.
- Keep credentials, tokens, cookies and signed URLs out of everything you write, including a description of how you tested; nothing redacts it afterwards.
- Name a file by its repo-relative path, such as `crates/kernel/src/secret.rs`. An absolute path spells a home directory and an account name.
- Quote the lines of terminal output that carry the finding; a raw dump brings the prompt, the host name and the account name with it.
- Describe what you found, since the diff says what you did. Your conversation, reasoning trace, tool-call log and planning notes stay out, and so do people's names, e-mail addresses, machine names, LAN addresses, and account, project or organisation identifiers; a commit's author identity is the only place a person's name belongs.
- Crop a screenshot to the application, with no desktop, task bar, window title or browser tabs.
- When a finding genuinely needs private data to state, describe the shape instead of the value: *"a key-shaped value arrives in the header"*.
- Read your own description once with this section in hand before you open a pull request; a force-push afterwards does not remove what was already fetched.

</rules>

<procedure>

## One change, five steps

1. **Take one piece of work.** Read its context in full before starting: the crate's SPEC, the neighbouring modules, and their tests.
2. **Write the SPEC first.** Interfaces and decisions land in the crate's SPEC before the code exists: the shape the module instantiates ([`ARCHITECTURE.md`](ARCHITECTURE.md) §9), its public signatures, the failures it returns with their codes, the values it fixes, and the decision that chose this over the alternative. A SPEC section that only names the module is not this step. When no shape fits, stop and ask rather than write. A move that leaves every interface unchanged — a file split, a module moved — needs no SPEC section, because the module map row is its record.
3. **Red.** Write the failing test and run it once to watch it fail on an assertion. That run is what proves the test can bite; a compile error from a missing symbol proves nothing about the defect.
4. **Green.** Implement until it passes, and no further. When the implementation wants to differ from the SPEC, change the SPEC first.
5. **Close.** The branch check is green, the red-to-green transition is visible in the commit order, the SPEC and the code agree, and the module map is updated.

The client is exempt from steps 2 and 3, as *The view layer* says.

Unless the change is mechanical, keep the diff under 800 changed lines, and under 500 when the logic is not obvious. When it is larger, land the smallest coherent stage that holds on its own and name the remaining stages, splitting along the actual diff and its call sites.

## Verification

Feedback in seconds keeps each step honest and the whole check takes minutes, so verification runs in three tiers: the first catches a defect in the change, the second a change that breaks its neighbours, the third two branches that are each green and wrong together.

1. **While iterating, run the narrowest command that can fail for the change**, as soon as the change exists: `cargo nextest run -p <crate> -E '<filter>'` naming the new test and the tests of the modules you changed; `cargo clippy -p <crate> --all-targets --all-features -- -D warnings`; `cargo check -p <dependent>` for each direct dependent (`cargo tree --workspace -i <crate> --depth 1`) after a `pub` surface changed; `cargo xtask gates <name>...` naming the gates that judge the files you touched; and `cargo fmt --check`, which compiles nothing. Leave whole suites, the artifact gates (`render`, `budget`, `npm`) and release builds to the next tiers unless the change is about them.
2. **Before a branch merges, run the branch check once**, `just check-branch <base>`: the whole test suites of the crates the branch changed, the tests in their dependents that name what changed, clippy on those crates, `cargo fmt --check`, and every gate that reads sources, adding `render`, `budget` and `npm` after `just build-web` when the branch touched `client/`.
3. **`just check` runs once on the tree that merges a batch of branches into `main`, and `main` advances only when it is green.** The merge runs it as `just check-all`, so one run lists every failure; rerun a red phase alone with `just check-all <phase>`, confirm each fix with the narrowest command that shows it, and then run `just check-all` again.

A green result belongs to the tree it ran on: record it with `git rev-parse HEAD^{tree}`, and rerun it on that tree only when something outside the tree, such as the toolchain, changed.

Four workflows run on a schedule rather than on a push, and each judges something a push cannot: `nightly.yml` (advisories, fuzzing, the API baselines, the kani autoharness lead, and the real-endpoint e2e job where the repository has an endpoint configured), `platforms.yml` (macOS, the Nix flake, a byte-for-byte rebuild), `adversary.yml` (the Lean adversary on a fresh seed) and `upstream-watch.yml`. A red scheduled run is news about the tree as it stands; read it the next morning and fix the cause on `main`.

## Commits

- The first line is `card-<stage>.<index>: <what>`, for example `card-S4.02: the wire, and the two frames a socket cannot spell`.
- The body records what you found, since what you did is already in the diff: a gate that changed your design, a red-to-green transition that exposed a real defect, a choice between two approaches whose reason the result does not show.
- A commit that loosens a gate under the ruling *The machine gates* requires carries the `Verdict: user-approved` trailer.

</procedure>
