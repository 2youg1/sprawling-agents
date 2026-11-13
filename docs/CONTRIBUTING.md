# Contributing — preparing and submitting a change

This guide explains contribution preparation, feature admission and submission. [AGENTS.md](../AGENTS.md) holds the repository rules, [ARCHITECTURE.md](ARCHITECTURE.md) holds structure, and [the glossary](glossary.md) defines the vocabulary. Follow the authority order in AGENTS.md when documents disagree, and correct the lower-priority document in the same change.

## 0 Development tools

Install `just` once with `cargo install just --locked`, then run `just prereqs` to find the other tools and their install commands. The [verification tiers](verification.md) decide which checks to run while editing, before submission and on the integrated tree; use that policy before starting a full check.

Start a fresh checkout by reading [AGENTS.md](../AGENTS.md). Its required instructions and skills are tracked here; no global agent configuration or skill installation is needed. Contributors may separately choose [actionbook/rust-skills](https://github.com/actionbook/rust-skills) as auxiliary Rust reading. Its suggestions are not project rules and do not replace the built-in workflow.

## 1 Read before you write

Before editing, read [AGENTS.md](../AGENTS.md#read-before-you-write) completely and follow its task routing table. Read the affected crate's specification and relevant Lean parts, and the implementing source, callers, neighbouring modules and tests. Trace callers, data flow, invariants and failure paths before changing a shared interface, and identify the Lean properties the implementation must preserve. State the problem, the required result and the constraints before implementation, choose the smallest coherent change that meets the result, and include only the source and documents it needs; consider how the next maintainer will understand and extend the design.

Read the official documentation of the tools you use and load the vendor's agent guide or skill when one exists, as the same reading requirements specify. Before changing a client screen, read [the frontend method](frontend-method.md); the client's SPEC-first and red-stage exemptions are defined in [The view layer](../AGENTS.md#the-view-layer).

A new core feature must clear all five tests before any implementation: the need exceeds what a skill can meet; no hook plugin should own the behaviour; MCP, CLI or ACP integration cannot reasonably meet the requirement and its guarantees; it serves most users; it is neither a theme nor a workflow for one deployment. Otherwise keep it out of core; if it is still worth building, prefer an extension or a separate repository and bring the result back in an issue. If the seams are too narrow, an issue may propose the smallest interface needed before any implementation. The commit body carries one prose paragraph — the need, the alternatives, why the core should own it, with concrete scenarios and evidence for "most users". Bug fixes are exempt, except a new feature bundled with one. Use the applicable submission template when opening an issue or pull request.

## 2 One change

Before designing or changing code, load [ub-mode](../crates/city/skills/ub-mode/SKILL.md) and follow its loop for scope, specification, regression evidence, implementation and closure. [The view layer](../AGENTS.md#the-view-layer) defines the client exemptions; [SPECs and Markdown](../AGENTS.md#specs-and-markdown) defines where an unfinished interface is recorded.

Keep a pull request's change surface as small as the coherent result permits, ideally within 99 files. Do not split a deletion that removes more than it adds merely for its size. A larger migration lands in stages that each hold on their own, split along actual call sites rather than file counts.

## 3 Three tiers of verification

Use [Verification](verification.md) for the narrow local checks, branch CI, batch CI and the local equivalent when runners cannot be reached. A result applies to the tree checked, and the same document defines how to record and reuse that evidence.

## 4 Rules and checks

[The machine gates](gates.md) holds the rules and their enforcement. `cargo xtask gates --list` prints the current roster; `cargo xtask gates <name>...` runs selected gates. A failure names the rule, the violation and an alternative.

Fix the reported cause, and follow that section's requirements for separate machinery commits and approval when loosening a gate in the change it rejects. Passing gates supplements review; it does not establish that a feature belongs in the core or that a design is sound.

## 4.1 Continuous integration

Before submission, apply [tier 1](verification.md#tier-1--while-iterating) to the complete final diff and record each command and result in the pull request. Then follow [tier 2](verification.md#tier-2--the-branchs-ci) for branch CI. A contributor without push access opens the pull request only after the fork run is green; link that exact run and identify any check left to the maintainer because the fork lacks its required resources.

## 5 Comments and documentation

Use [SPECs and Markdown](../AGENTS.md#specs-and-markdown) for current-state specifications and decision placement, and [Language](../AGENTS.md#language) for comments, rustdoc and document languages.

### Languages

For issues, use the [Bug report](../.github/ISSUE_TEMPLATE/bug_report.yml), [Enhancement](../.github/ISSUE_TEMPLATE/enhancement_request.yml) or [Feature request](../.github/ISSUE_TEMPLATE/feature_request.yml) form; [§10](#10-issue-labels-and-triage) covers the labels they apply. For pull requests and review comments, follow [Language](../AGENTS.md#language).

Before committing or submitting text or attachments, apply [Public content and privacy](#public-content-and-privacy) to establish what readers need and inspect what they will receive. Use [the private security channel](SECURITY.md) for vulnerabilities.

### Public content and privacy

`xtask secret` and `xtask release` scan tracked files for shapes such as credential prefixes and home-directory spellings. Prose about a working environment has no such shape, and no gate scans a pull request description, commit body, issue, review comment, or attachment. The author owns that inspection.

- Keep information unpublished until a public reader needs it to use, understand, reproduce, review, or maintain the project. Publish only the checked minimum. Required source, specifications, and tests stay tracked; working notes and incidental context stay outside public artifacts.
- Ship the decision, its reason, and the parameter that would reopen it. Leave out the occasion, the participant, and the machine on which it arose.
- Cite only material a reader can open. Replace a citation to a private plan with its reason or the SPEC section that holds the decision.
- Describe what was found; the diff shows what changed. Omit conversations, reasoning traces, tool-call logs, personal names, addresses, machine names, and account or organisation identifiers.
- Keep credentials, tokens, cookies, and signed URLs out of every artifact. Nothing redacts them after publication; an exposed credential needs rotation outside the repository.
- Use repository-relative paths. Quote only terminal lines that carry the finding, and crop screenshots to the application without desktop or browser chrome.
- When a finding needs private data, describe its shape rather than its value.
- Report a measurement with its machine class. The manifest or doctor's output carries the toolchain version.

*The person* as a domain role remains correct in an interface, payload, or rustdoc line. This section governs working context, not that vocabulary.

### Other projects

Name another project only when the issue or the pull request cannot be understood or checked without it, and keep the rule tightest for experimental or unpublished ones: they offer no stable interface to follow and no licence this repository can rely on, and a public reference outlives whatever they become. A reference is read as an endorsement, an affiliation or a comparison, so an unneeded one buys a licence question the change does not answer.

A change that borrows from another project states which of the two cases holds:

- **A borrowed fact** — an interface shape, a protocol, a value read from upstream — is cited, and [`third-party.md`](third-party.md) is updated in the same change: its [§1 table](third-party.md#1-where-the-intelligence-comes-from), with the path followed and the commit read, because `upstream-watch.yml` reads that table and the tracked commit is written nowhere else. Keep a quotation to the short excerpt the citation needs.
- **Borrowed code** is not pasted in. This repository vendors no upstream code, because every `.rs` file carries the MPL-2.0 notice and a file from another project cannot wear it ([`third-party.md`](third-party.md) §7). Re-implement from the documented facts, or bring the work in as a dependency through the gates: `cargo deny` against the licence allowlist in `deny.toml`, or the client's `RUNTIME` list in `crates/xtask/src/npm.rs`. Copyleft work, GPL-family code among it, is not copied in at all.

If neither case holds, the project is not mentioned. If which case holds is unclear, ask in the issue before opening the pull request.

## 6 Commit messages

Follow [Commits](../AGENTS.md#commits) for the subject, the findings the body records and any required ruling trailer; the feature justification belongs to [feature admission](#1-read-before-you-write).

## 7 Tests

Use [Testing](testing.md) to choose meaningful checks and distinguish negative fixtures, a regression's red stage, and deliberate failure or environment experiments. Keep rejection assertions in passing outer tests in the default suite; use the opt-in and return conditions defined there for experiments, and fix regressions before merging rather than hiding a defect through test selection.

When adding a gate, build a fixture that violates the rule and assert that the gate rejects it; keep that assertion in the default suite so a gate that stops rejecting the bad input turns the test red.

## 8 Environment

The repository pins the Rust toolchain in `rust-toolchain.toml`, Lean in `lean-toolchain` and Zig in `crates/desktop/ffi/zig-version`, and names every other tool in the develop tier of the doctor's table (`crates/sprawling/src/doctor/table.rs`), which is the only list: `sprawling doctor` and the settings page's dependency group install from it, and `just prereqs` reads it through the `prereqs.tsv` the table renders, printing what is absent, what needs it, and the line that installs it. Rows marked *required* are what `just check` cannot run without; rows marked *optional* are skipped by `just check` when absent, or belong to a recipe that says so itself. `nix develop` enters a shell holding those tools, and `nix flake check` refuses a shell that stops short of the list.

**Zig is checked to the exact version.** The desktop leaf's build script runs `zig version` and refuses any version other than the pin, so a machine that builds the workspace on Windows needs that Zig on PATH, and so does anyone who installs `sprawling` from crates.io there; `sprawling doctor` lists it with its install line. The client's `.ts` type check is `bun check`, the TypeScript 7 checker built into the runtime; `.svelte` files are checked by `svelte-check`, whose classic lane reads the `typescript` 6 package - kept also for typescript-eslint, whose peer range does not admit 7 yet (`client/README.md`).

Use [Where code goes](../AGENTS.md#where-code-goes) for the line-ending authority and recovery from a whole-file diff.

**The compile-failure counterexamples are byte comparisons against a compiler's output, so the toolchain installation is part of them.** Installing the `rust-src` component makes rustc render a source snippet inside a `note:` that the committed `.stderr` files do not carry, and every counterexample that meets one goes red without a line of this repository changing. A tool that pulls that component in can turn `just check` red on the next run; remove it (`rustup component remove rust-src`) rather than blessing the longer output, which would only move the failure to CI. The passing outer tests remain required; this environment correction does not authorize skipping them.

Use [The loop](verification.md#the-loop) for build-lock and slow-build handling.

## 9 Command surface

[The loop](verification.md#the-loop) lists the repository commands and their purposes; `just --list` prints every recipe available in the checkout.

## 10 Issue labels and triage

Before acting on or opening an issue, search related open and closed issues and pull requests. Read every comment, review, and inline review comment to the end; search snippets do not replace the discussion. Use `gh issue list --state all --search <terms>`, `gh pr list --state all --search <terms>`, then `gh issue view <n> --comments` or `gh pr view <n> --comments`. Read recent history with `git log -p -n <count> -- <path>` so a duplicate, settled decision, or fix in flight does not get rebuilt. An issue links what this search found.

Every label — its name, colour and description — has one authority, [`.github/labels.yml`](../.github/labels.yml), and the live set is synchronized from that file; the command is at the end of this section. The labels divide along four axes:

- `type:` — what the issue is, exactly one: `bug`, `enhancement` or `feature`. The forms apply it: [Bug report](../.github/ISSUE_TEMPLATE/bug_report.yml), [Enhancement](../.github/ISSUE_TEMPLATE/enhancement_request.yml), [Feature request](../.github/ISSUE_TEMPLATE/feature_request.yml). An issue about documentation is still one of the three, with `area: docs` naming where it lands.
- `area:` — where the work lands, one or more: `frontend` (`client/`), `backend` (the Rust binary and `crates/`), `docs`.
- `aspect:` — a cross-cutting concern, zero or more: `performance`, `security`, `accessibility`. A security bug is `type: bug` plus `aspect: security`, reported through [SECURITY.md](SECURITY.md) rather than a public issue.
- `status:` — the triage state, below.
- `help wanted` and `good first issue` — the contribution labels, below.
- `duplicate`, `invalid`, `wontfix` and `question` — close out or redirect an issue.

An issue opened by someone who cannot triage it, through a form or not, is marked `status: needs-triage` — 待审核, the mark that the maintainer has not read it yet — by [issue-triage.yml](../.github/workflows/issue-triage.yml), which is the only source of the mark; an issue the maintainer opens is already read, and an issue triaged before the run reaches it keeps the status it was given. Triage then ends in one of three ways: `status: accepted` — 待解决, read and wanted, waiting for a fix or an implementation; `status: needs-info`, waiting on the reporter for more; or a close as `duplicate`, `invalid` or `wontfix`, or an answer and close as `question`. `area:` and `aspect:` labels are added during triage. When the work lands the issue is closed, and the same workflow takes its status label off, so an open `status: accepted` issue is unresolved by definition.

`help wanted` is applied by the maintainer to an accepted issue whose need and acceptance criteria are already written down; the label is the endorsement that a pull request meeting those criteria is welcome, and the issue itself is the specification for the work. Comment on the issue to claim it, and link the pull request back to it. `good first issue` marks the subset of those that suit a first-time contributor. A contributor without triage or write access cannot set a label; name the one you believe fits in a comment.

Dependabot applies `dependencies`, `rust` and `github_actions` to its pull requests, and `upstream-watch.yml` applies `upstream-watch` to the issues it opens; leave those labels in place.

To change the set, edit `.github/labels.yml` and synchronize it: `bunx github-label-sync --access-token "$(gh auth token)" --labels .github/labels.yml 2youg1/sprawling-agents` (`--dry-run` previews). A label the file does not carry is removed by the sync, which is the intent: one file decides the set.
