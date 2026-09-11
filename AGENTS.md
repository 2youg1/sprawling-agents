# AGENTS.md — how work is done in this repository

sprawling is a locally deployed agent-city harness: one Rust binary, a browser client it serves, and a city that keeps its whole history in one append-only Ledger. Most of it was written by models. Every rule below holds for a person too, because the person who picks this up next also does not remember yesterday.

Read this file to the end before the first edit.

## The loop

```bash
cargo install just cargo-nextest --locked   # once; the toolchain installs itself from rust-toolchain.toml
just check                                  # fmt + clippy (-D warnings, --all-features) + nextest + client + every machine gate
```

**A change is finished when `just check` is green.** "I finished it" is a claim; a green run is the evidence.

| Command | What it does |
|---|---|
| `just check` | the closing condition for every change |
| `just gates` | the machine gates alone |
| `just check-client` | the client's lint, typecheck and tests |
| `just build-web` | build the client bundle into `target/web-dist` |
| `just dist` | the whole deliverable: client, binary, bill of materials, size badges |
| `just sim [seed]` | citysim scenarios; a failure reproduces from its seed |
| `just spec <crate>` | generate a SPEC skeleton |
| `just api-baseline` | recompute the public-surface baselines |
| `just replay <log>` | verify a ledger chain offline, read-only |
| `just mem [pid]` / `just bench` / `just budget` | the measurements, in this platform's own vocabulary |
| `just fuzz <target>` / `just mutants` | fuzz targets / mutation testing |
| `just adversary` | the out-of-tree property checker that attacks the binary through the wire; never a gate, and a no-op without GHC |

- Scope `cargo nextest` and `cargo build` to the crate you edited and the dependents `cargo tree --workspace -i` reports while iterating; run them whole once before the work is done. `cargo clippy --all-targets` and `cargo fmt --check` stay workspace-wide, because a warm cache answers both in seconds.
- Be patient with a Rust command and never kill it by PID. The lock makes it slow; that is expected.
- Time a build that feels slow before tuning it. The seconds sit in particular compilation units, not in the breadth of the command.

## Read before you write

- [`ARCHITECTURE.md`](ARCHITECTURE.md) — crate topology, seams, module map, the seven shapes, determinism. Its `depmap` block and module map are machine authorities.
- `crates/<crate>/<crate>-SPEC.md` — that crate's interfaces and decisions, written before its code.
- [`docs/glossary.md`](docs/glossary.md) — one name per concept, enforced by a gate.
- The tests next to the code you are about to change.
- **Read the official documentation of a tool before you use it** — a language feature, a crate, a CLI, a framework. Load the vendor's own agent guide or skill when one exists.

## One change, five steps

1. **Take one piece of work.** One session, one bounded change; read its context in full before starting.
2. **Write the SPEC first.** Interfaces and decisions land in the crate's SPEC before the code exists. A new module states which of the seven shapes it instantiates ([`ARCHITECTURE.md`](ARCHITECTURE.md) §9); when there is no answer, stop and ask rather than write.
3. **Red.** Write the failing test and **run it once to watch it fail**. That run is what proves the test can bite.
4. **Green.** Implement until it passes, no more. When the implementation wants to differ from the SPEC, change the SPEC first.
5. **Close.** All four: `just check` green | the red-to-green transition visible in the commit order | SPEC and code in step | the module map updated.

The client (`client/`) is exempt from steps 2 and 3 — see *The view layer* below.

- Before the code exists, the crate's SPEC carries the shape the module instantiates, its public signatures, the failures it returns with their codes, the values it fixes, and the decision that chose this over the alternative. A SPEC section that only names the module is not step 2.
- Unless the change is mechanical, keep the diff under **800 changed lines**, and under **500** when the logic is not obvious.
- When it is larger, find the smallest coherent stage that can land on its own and say what the remaining stages are. Base the split on the actual diff and its call sites, not on a guess about what looks separable.

## Rust

- Return failure through `Result`. No `unwrap`, `expect`, `panic!`, `todo!`, `unreachable!`, bare indexing or slicing in non-test code.
- Use checked arithmetic and `TryFrom`. No `as` casts.
- `unsafe_code` is `forbid` across the workspace. `desktop/` relaxes it to `deny`, and that relaxation is the only reason that package sits outside the workspace — do not import the exemption anywhere else.
- In `desktop/`, `unsafe` appears only under `platform/windows/`, and wraps the FFI call alone. Do not pull the reasoning that follows it inside the block; that only makes the next reader audit more lines.
- Every `unsafe` block carries one `SAFETY:` line giving **the precondition that makes the call sound**. Test the line by asking whether what it says could be false: *"we call `EnumWindows`"* cannot be false, so it is a restatement; *"the callback is an `extern "system" fn` in this module, and the `Vec` behind `lparam` outlives the call with no second alias"* can be false, so it is a precondition.
- Do not erase a failure: no `let _ =` on a `Result`, no `unwrap_or_default` standing in for a decision, no `.ok()` that drops the reason a caller needed.
- Suppress a lint with `#[expect(reason = "…")]` at the narrowest scope, never `#[allow]`. An `expect` that stops firing fails the build, which is how the suppression cleans itself up.
- Make `match` exhaustive. Avoid wildcard arms.
- Avoid `bool` and bare `Option` parameters that force a caller to write `foo(false)` or `bar(None)`. Prefer an enum, a newtype, or a named method that keeps the call site self-documenting.
- Prefer a typestate or a newtype that makes the invalid state unrepresentable over a runtime check that rejects it.
- Give an error the failed action, the subject, a stable code, and a recovery the caller can act on.
- Values that always travel together are one value. Give them a name.
- Take the time as a parameter. The single sampling point is `bin::assembly`.
- Use `BTreeMap` on kernel decision paths; keep floats out of ledger payloads; start tasks from the one spawn point.
- Inline `format!` arguments: `format!("{name}")`, not `format!("{}", name)`.

## Where code goes

- One module, one file, semantically named. Register the file in the module map, then create it. Keep `lib.rs` and index files free of logic.
- Keep a function inside 200 lines and 4 parameters, and a file inside 400 lines, tests included. `xtask/budgets.toml` holds the register of what predates each re-pricing, pinned at the length it had the day the line moved: a named file may not exceed its pin, and its entry is struck once it no longer needs one.
- **Every register in `budgets.toml` is a ratchet.** A number may improve freely and drift only within its slack, so a gradual regression cannot arrive one commit at a time. Record the new reading and the reason beside it; never widen the slack to admit one.
- These files sit within four lines of the cap. Add to a new module rather than to them: `crates/memory/src/worktree/trees.rs`, `crates/runtime/src/tools/exec.rs`, `crates/city/src/spine_files.rs`, `crates/sprawling/src/mcp_stdio.rs`, `xtask/src/modmap.rs`, `xtask/src/boundary.rs`.
- Prefer a new module over growing an existing one. When you extract from a large module, move its tests and type docs with it so the invariants stay next to what owns them.
- No `utils`, `helpers`, or `common` module. Name a module for what it owns.
- Do not create a helper referenced once, or a wrapper that only renames what it calls.
- Default to `pub(crate)`. Declare a `pub` trait only in a file on the seam list.
- Introduce a trait only at a seam that already has a second implementation — a test clock, a counting store, a second adapter. Similar text and hypothetical reuse are not seams.
- **Give every rule, state transition and decision exactly one authority.** Two places that decide the same thing is a defect while they still agree.
- Finish a migration inside one change-set: move every reader and writer, exercise the production path, then delete the old authority and its adapters.
- Trace callers, data flow, invariants and failure paths before you touch a shared interface.

## SPECs and Markdown

- **Write each revision as the document's first draft.** A SPEC states what is true now, not how it became true.
- Keep the reason a rule has its current shape; the next change depends on it. Drop the record of what the rule used to be; that is what git is for.
- The check is one sentence: *a reader opening this file for the first time meets no sentence that only someone who read the previous version can use.*
- A decision belongs in the SPEC's **§12 Decisions** as a numbered entry giving the decision, its reason, and the alternative it beat. It does not appear anywhere in §1–§11 as narration about when it changed.
- Do not write a changelog, a session log, or a card number into §1–§11. "This used to be X", "re-priced on <date>", "session 4 found" — none of these are what the interface is, and the last one carries working context off this machine (see *Privacy*).
- An acceptance record states what is verified and by which command, in the present tense. It does not state which sitting verified it, on what date, or what was still unverified when that sitting ended — that last one goes stale silently and reads as a defect long after it is fixed.
- **An open question in the tree is a claim about the code, never a message to a person.** §3 of a SPEC says what about the interface is undecided and what evidence would settle it. *"Awaiting a ruling"*, *"the call is yours"*, *"I am not doing this step"* are turns in a conversation: a reader outside cannot act on them, and they are the sentences that go stale first.
- **Delete an open question the moment it is answered, and put the answer where it is enforced.** A question that outlived its answer is worse than no entry — it tells every later reader that a settled matter is still open, and they have no way to tell from the document that it closed.
- Mark a ruling as a ruling when the authority ladder needs it: some rules hold because the person chose, not because anything derived them. Give the decision and its reason. Do not give the date, the sitting, or the words it arrived in.
- A rule that excludes an architecture states the parameter that made it right, beside the rule. That is not history; it is what the next reader needs in order to re-argue it.
- When a session ends with the work unfinished, write what is left into the SPEC section it belongs to — as the current state of that interface, not as a note about your session.
- Write for engineers across many countries and many levels of English. Prefer the concrete word to the abstract one, carry each point in one clause, and put what matters most at the end of the sentence.

## Tests

- Tests use the same doors as production code. To exercise something internal, put a seam on that face and give it a second adapter, or drive it from outside through citysim.
- Test modules may relax lints locally with `#[allow]` on the test module. Production code carries them as written.
- Prefer comparing whole objects to comparing fields one at a time.
- Do not add a test for a statically defined value, or a negative test for logic that was removed.
- A citysim failure must reproduce from its seed. When it does not, the defect is the determinism, not the scenario.
- A check that enters through the crates' public faces is Rust, beside the code it judges. A check that enters the way a stranger does — spawning the binary, opening a socket to a served city, speaking the wire from outside — is Haskell under `adversary/`. `xtask boundary` holds the line.
- `adversary/` shares `docs/glossary.md`. Do not coin a Haskell-side name for something the glossary already names.
- **When the adversary finds a defect, the knowledge migrates.** A person writes the failing case as a Rust test under `crates/sprawling/tests/`, and the Haskell side does not keep it. `adversary/` quantifies over traces; it is not a second home for a fact.
- `just check` on a machine without GHC behaves byte for byte as it does where `adversary/` is absent. Never make `just check` depend on it.

## The machine gates

Violating any of these turns CI red with a message naming the rule, the violation, and an alternative.

| Rule | Held by |
|---|---|
| The Rust rules above: no panics, checked arithmetic, no `unsafe`. | workspace lints, `-D warnings` |
| Module map registered; sizes inside 200/4/400. | `xtask modmap`, `xtask length` |
| `pub(crate)` by default; `pub` traits only on the seam list. | `xtask depmap` |
| The client's lockfile in step with its manifest, its runtime dependencies exactly `solid-js` and `effect`, every licence on the list `deny.toml` permits. | `xtask npm` |
| The MPL-2.0 notice then the copyright line, at the top of every `.rs` file. | `xtask header` |
| A white-box check written in Rust beside the code it judges; a check that enters the way a stranger does written in Haskell under `adversary/`. | `xtask boundary` |
| Nothing written for a test compiled into the binary a person downloads. | `xtask artifact` |
| One name per concept, taken from the glossary. | `xtask lexicon` |
| Each crate's committed baseline equal to its live public surface; the crate's SPEC moved in the same change-set as a baseline edit. | `xtask apisync` |
| Credentials as `secret:realm/name` references; plaintext reaches the vault and nowhere else. | `xtask secret` |
| Colour taken from the `@theme` block in `client/src/theme.css`, expressed as a ratio of the gamut limit. | `xtask color` |
| Every word a reader is given taken from `client/src/lang.json`. | `xtask wording` |
| Every role, accessible name and landmark a settled screen wrote down, offered by the shipped screen, with no box outside its container. | `xtask render` |
| Every kernel enum described in its SPEC table, variant for variant. | `xtask specalign` |
| Every verb the city can carry out reached by some control, or classified on the wire seam with the reason a person may not ask for it. | `xtask wiring` |
| `client/src/wire.ts` regenerated whenever `WIRE_V` moves. | `xtask wire-ts` |
| Sizes inside their budget, badges in step with the artifacts. | `xtask budget` |
| Nothing published that names one machine's home directory, its working notes, or a document this tree does not contain. | `xtask release` |
| **Fix the cause when a gate goes red.** | `xtask guard` |

- Loosening a gate **in the change that the gate is failing** requires an explicit ruling from the person, recorded as a `Verdict:` trailer. That is the one universal escape hatch, and this is what closes it.
- Re-pricing a rule **in a commit of its own** is ordinary work and needs no ruling. `xtask guard` looks for the pair — gate machinery and the source those gates judge, changed together without a trailer. One side alone never meets the rule.
- **Every rule that excludes an architecture carries the parameter that made it right.** When that parameter moves, re-argue the rule instead of obeying it. Rules that exclude a *defect* — the panic bans, the arithmetic bans, the determinism rules — carry no such condition, because nothing about them expires.

## The view layer

`client/` is **not** held to SPEC-first or to red-before-green. What makes an interface good is cheap iteration, and charging a SPEC and a failing test for every visual change buys correctness the view layer was not losing while taxing the only thing it is short of.

It is held to what survives: colour from `theme.css`, wording from `lang.json`, the size budget, and the eslint rules the client's own config sets (no `any`, no `as`, no `throw`, no `try`, no non-exhaustive switch).

**What replaces the ceremony is a method.** [`docs/frontend-method.md`](docs/frontend-method.md) is how a screen gets built here: settle it against the shipped stylesheet, add the fixture to `#/gallery`, and accept it with `cargo xtask render`, which opens the screen in a real engine and measures where its boxes landed. Read it before changing a screen.

## Language

| Where | Language |
|---|---|
| Identifiers, event names, error codes, rustdoc, commit subjects | English |
| `README.md`, `AGENTS.md`, `ARCHITECTURE.md`, `docs/` | English, except `README.zh-CN.md` and `docs/getting-started.zh-CN.md` |
| Crate SPECs and design discussion | Chinese, with concept names kept in their English form |
| Pull requests, issues, review comments | **your own language.** A parallel translation is welcome, not required: side by side a reader is faster, and a mistranslation is visible instead of silent |

- Comments are a failure signal by default. Four kinds earn their place: the MPL notice, public interface documentation, a warning about consequences, and a statement of intent the code cannot carry.
- In rustdoc, write what the signature cannot say — invariants, failure modes, call ordering, ownership.

## Privacy

**A working record is noise to a contributor and to a model, and signal to an attacker.** It is the same sentences either way: what was tried and abandoned, which machine could not verify what, which account ran out of quota, when somebody is away, what is still unfixed and how long it has been unfixed, and the words a decision arrived in. A reader skips them; someone looking for a way in reads them closely.

Two surfaces leak, and the gates reach only part of one.

`xtask secret` and `xtask release` scan files in the tree, and they judge **shapes** — a credential's prefix, a home directory's spelling. Working context written as ordinary prose has no shape, so it passes: *"only a third could be verified on this machine"* is a fact about somebody's hardware and every gate reads it as a sentence. **A pull request description, a commit body, an issue and a review comment are not files in the tree at all, and no gate reaches them.**

So the first group of rules below is about what you write into the product, and the second is about what you write around it. Both are yours to hold.

- **Ship the decision, not the occasion.** The decision, the reason it beat the alternative, and the parameter that would re-open it all belong in the tree. Who said it, when, in which session, on whose machine, and in what words do not.
- Never write your own working record into the product: no session diary, no round-by-round acceptance log, no "what I found this time", no dated ruling, no quotation of the person's words, no section numbered by which sitting produced it.
- **This is a privacy rule before it is a tidiness rule.** A working record is about a person — their machine, their toolchain versions, their account state, their words — and it ships to strangers inside whatever file carries it.
- Citing a log a reader cannot open leaks twice: it tells them the log exists, and it leaves the document unreadable without it.
- Working records live in `local/`, which is gitignored, and `xtask release` refuses a published document that sends a reader there. `.gitignore` also refuses the names these arrive under, so the common mistake cannot be committed rather than merely being forbidden.
- Report a measurement against the machine class it came from, never against yours: *"22 s on a warm cache, four cores"*, not *"22 s on this machine"*. A toolchain version belongs in `rust-toolchain.toml` or the doctor's output, not in prose.
- `the person` as a **domain role** is not covered by any of this. A city has residents and one person; naming that role in an interface, a payload or a rustdoc line is correct and stays.
- Never paste a credential, a token, a cookie or a signed URL anywhere, including into a description that explains how you tested. There is no redaction step afterwards, and a private repository still keeps a log.
- Name a file by its repo-relative path: `crates/kernel/src/secret.rs`, never `C:\Users\<someone>\...\crates\kernel\src\secret.rs`.
- Do not paste raw terminal output. Quote the lines that carry the finding. A shell dump brings the prompt, the host name, the account name and whatever the environment was showing.
- Do not paste your own conversation, reasoning trace, tool-call log or planning notes. The description says what you found; the diff says what you did.
- People's names, e-mail addresses, machine names, LAN addresses, and account, project or organisation identifiers do not appear. A commit's author identity is the only place a person's name belongs.
- A screenshot shows the application. Crop out the desktop, the task bar, the window title, the browser tabs, and everything else on the screen.
- When a finding genuinely needs private data to state, describe the shape instead of the value: *"a key-shaped value arrives in the header"*, not the key.
- Before you open a pull request, read your own description once with this section in hand. It is faster than asking for a force-push afterwards, and a force-push does not remove what was already fetched.

## Commits

- The first line is `card-<stage>.<index>: <what>`, for example `card-S4.02: the wire, and the two frames a socket cannot spell`.
- The body records **what you found**, since what you did is already in the diff: a gate that changed your design, a red-to-green transition that exposed a real defect, a choice between two approaches whose reason the result does not show.
- A commit touching anything the `guard` row covers carries a `Verdict: user-approved` trailer. The wording of the ruling stays with the person; the trailer records that there was one.

## Where the authorities are

1. The person's ruling.
2. [`ARCHITECTURE.md`](ARCHITECTURE.md) — structure.
3. `crates/<crate>/<crate>-SPEC.md` — that crate's interfaces and decisions.
4. The code and its tests.

Where a lower level contradicts a higher one, the higher wins and the lower is corrected in the same change. Where reality contradicts all of them, reality wins and the document is corrected first, with its reason.

One directory named in `.gitignore` holds one machine's working notes. Nothing in this repository may depend on it, and `xtask release` refuses a document that sends a reader there.
