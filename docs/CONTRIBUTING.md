# Contributing — preparing and submitting a change

This guide explains contribution preparation, feature admission and submission. [AGENTS.md](../AGENTS.md) holds the repository rules, [ARCHITECTURE.md](../ARCHITECTURE.md) holds structure, and [the glossary](glossary.md) defines the vocabulary. Follow the authority order in AGENTS.md when documents disagree, and correct the lower-priority document in the same change.

## 0 Development tools

Install `just` once with `cargo install just --locked`, then run `just prereqs` to find the other tools and their install commands. The [verification tiers](../AGENTS.md#verification) decide which checks to run while editing, before submission and on the integrated tree; use that policy before starting a full check.

## 1 Read before you write

Before editing, follow [AGENTS.md's reading requirements](../AGENTS.md#read-before-you-write): read that file completely, ARCHITECTURE.md, the glossary, the affected crate's specification and relevant Lean parts, and the implementing source, callers, neighbouring modules and tests. Trace callers, data flow, invariants and failure paths before changing a shared interface, and identify the Lean properties the implementation must preserve. State the problem, the required result and the constraints before implementation, choose the smallest coherent change that meets the result, and include only the source and documents it needs; consider how the next maintainer will understand and extend the design.

Read the official documentation of the tools you use and load the vendor's agent guide or skill when one exists, as the same reading requirements specify. Before changing a client screen, read [the frontend method](frontend-method.md); the client's SPEC-first and red-stage exemptions are defined in [The view layer](../AGENTS.md#the-view-layer).

For a new feature, apply the [core-feature criteria](../AGENTS.md#read-before-you-write) before choosing an implementation and follow that section's commit-body requirement. Use the applicable submission template when opening an issue or pull request.

## 2 The five steps of one change

Follow [One change, five steps](../AGENTS.md#one-change-five-steps) for specification, regression evidence, implementation and closure. The same section defines the diff limits and the client exemptions; [SPECs and Markdown](../AGENTS.md#specs-and-markdown) defines where an unfinished interface is recorded so the next contributor can continue it.

## 3 Three tiers of verification

Use [Verification](../AGENTS.md#verification) for the narrow local checks, branch CI, batch CI and the local equivalent when runners cannot be reached. A result applies to the tree checked, and the same section defines how to record and reuse that evidence.

## 4 Rules and checks

[The machine gates](../AGENTS.md#the-machine-gates) holds the rules and their enforcement. `cargo xtask gates --list` prints the current roster; `cargo xtask gates <name>...` runs selected gates. A failure names the rule, the violation and an alternative.

Fix the reported cause, and follow that section's requirements for separate machinery commits and approval when loosening a gate in the change it rejects. Passing gates supplements review; it does not establish that a feature belongs in the core or that a design is sound.

## 4.1 Continuous integration

The [verification policy](../AGENTS.md#verification) defines how branch and batch checks run, how scheduled and on-demand workflows are used, and how contributors without push access obtain a green fork run before opening a pull request. Follow that procedure and link the exact run in the submission; identify any checks left to the maintainer because the fork lacks their required resources.

## 5 Comments and documentation

Use [SPECs and Markdown](../AGENTS.md#specs-and-markdown) for current-state specifications and decision placement, and [Language](../AGENTS.md#language) for comments, rustdoc and document languages.

### Languages

For issues, use the [Bug report](../.github/ISSUE_TEMPLATE/bug_report.yml) or [Feature request](../.github/ISSUE_TEMPLATE/feature_request.yml) form. For pull requests and review comments, follow [Language](../AGENTS.md#language).

Before committing or submitting text or attachments, apply [Privacy](../AGENTS.md#privacy) to establish their public necessity and inspect what readers will receive. Use [the private security channel](../SECURITY.md) for vulnerabilities.

## 6 Commit messages

Follow [Commits](../AGENTS.md#commits) for the subject, the findings the body records and any required ruling trailer; the feature justification belongs to the [reading requirements](../AGENTS.md#read-before-you-write).

## 7 Tests

Use [Tests](../AGENTS.md#tests) to choose meaningful checks and distinguish negative fixtures, a regression's red stage, and deliberate failure or environment experiments. Keep rejection assertions in passing outer tests in the default suite; use the opt-in and return conditions defined there for experiments, and fix regressions before merging rather than hiding a defect through test selection.

When adding a gate, build a fixture that violates the rule and assert that the gate rejects it; keep that assertion in the default suite so a gate that stops rejecting the bad input turns the test red.

## 8 Environment

The repository pins the toolchain in `rust-toolchain.toml` and names every other tool in the develop tier of the doctor's table (`crates/sprawling/src/doctor/table.rs`), which is the only list: `sprawling doctor` and the settings page's dependency group install from it, and `just prereqs` reads it through the `prereqs.tsv` the table renders, printing what is absent, what needs it, and the line that installs it. Rows marked *required* are what `just check` cannot run without; rows marked *optional* are skipped by `just check` when absent, or belong to a recipe that says so itself. `nix develop` enters a shell holding those tools, and `nix flake check` refuses a shell that stops short of the list.

Use [Where code goes](../AGENTS.md#where-code-goes) for the line-ending authority and recovery from a whole-file diff.

**The compile-failure counterexamples are byte comparisons against a compiler's output, so the toolchain installation is part of them.** Installing the `rust-src` component makes rustc render a source snippet inside a `note:` that the committed `.stderr` files do not carry, and every counterexample that meets one goes red without a line of this repository changing. A tool that pulls that component in can turn `just check` red on the next run; remove it (`rustup component remove rust-src`) rather than blessing the longer output, which would only move the failure to CI. The passing outer tests remain required; this environment correction does not authorize skipping them.

Use [The loop](../AGENTS.md#the-loop) for build-lock and slow-build handling.

## 9 Command surface

[The loop](../AGENTS.md#the-loop) lists the repository commands and their purposes; `just --list` prints every recipe available in the checkout.
