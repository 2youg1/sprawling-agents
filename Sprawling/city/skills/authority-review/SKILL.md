---
name: authority-review
description: "Resolve changed or conflicting authorities through the repository's authority order and executable evidence. Use when a diff changes a contract, two records disagree, or a gate and its documentation prescribe different actions. Not a general code review."
license: MIT
---

# Authority review

Resolve which rule governs and prove the result. Do not perform a general architecture, style or code-smell review.

> Adapted from the Thermos plugin in [`cursor/plugins`](https://github.com/cursor/plugins) (MIT), narrowed from general review to authority conflicts. Modified by 2youg1, 2026.

## Scope

Use this when the diff changes an authority, two records disagree, or a check and its documentation prescribe different actions. `blast-radius` owns behavioral reach; `ub-mode` owns design scope; machine gates own mechanical rules.

Read the complete diff and the authorities for each changed contract. The codebase is the next agent's memory: resolve the conflict at its source rather than adding a local interpretation that later readers must reconcile again.

## Steps

1. **Name the conflict.** State the observable behaviour change, or the records that already disagree. Stop only when the diff changes neither an authority nor any pair of records that conflict.
2. **Locate one authority.** Apply the order in `AGENTS.md`: the person's ruling, architecture for structure, the touched crate's SPEC for interfaces and decisions, then code and tests. Remove or rewrite contradictory lower-authority text in the same change. Two records at the same level that still disagree need the person's ruling; do not choose silently.
3. **Bind each finding to evidence.** A behavioral claim needs a runnable reproduction, test, replay, property or formal check. A contract claim needs the exact authority line and the check that enforces it. Mark anything else unverified; do not turn a plausible concern into a repair task.
4. **Leave a constraint.** When the same conflict can recur, encode the resolved rule in the owning type, API, generator, test or gate. Prose is only the part no mechanism can express. Do not add a waiver or a second local rule.

## Risk map

These are investigation branches, not a finding quota. Enter only the branches the diff touches, and close each one with the named evidence. A branch that does not apply needs one concrete reason, not a generic `n/a`.

| Problem the old review addressed | How it is closed now |
|---|---|
| Broken behavior or a security hole | Reproduce through the production door in `blast-radius`; retain a regression test, property, fuzz case or Lean proof. A reviewer saying it looks safe is not evidence. |
| Developer experience, configuration, environment variables, ports and build paths | Run the affected setup and platform commands, including fork or artifact CI where applicable. A source-only reading cannot establish that a fresh checkout works. |
| Feature-gate or test-scaffolding leaks | Inspect the real release bundle and exercise `artifact`, `packaged`, `render` and `wiring` for the changed surface. A flag existing in source is not proof that the shipped boundary holds. |
| Divergence from the recorded decision | Quote the surviving authority and name the test or gate that compares implementation with it. This remains the core of this skill. |
| Deliberate breakage and migration | Record the decision and the rejected alternative in the owning SPEC, then prove the migration at its real callers. Do not rediscover an accepted breakage as a finding. |
| One fact with several definitions | Name the surviving definition and every reader and writer to move onto it. Prefer a type, generator or machine comparison; a note that two copies should agree leaves the defect alive. |
| Shotgun surgery, divergent change and repeated switches | Enter through `ub-mode` when call sites compensate for the same decision or conditionals encode one state model. Show the invalid state or repeated failure, then remove it through the owning module, table or typestate. Do not flag a shape merely because it appears twice. |
| Data clumps and primitive obsession | Show which values always travel together or which invariant a primitive cannot express, then introduce the named value. `length` checks excessive parameters; it does not replace this judgment. |
| Duplicated logic | Prove the copies implement the same decision, not merely similar text. Extract at the authority that owns it; do not create a shared helper for accidental similarity. |
| Erased failure and silent fallback | Trace the decision the caller needs and make it explicit in the result type. Workspace lints cover mechanical forms; inspect any remaining default that changes meaning. |
| Shared writers, partial updates and needless serialization | Name the writer, invariant and atomic boundary, then exercise concurrent or interrupted execution. Keep one writer where the invariant requires it; otherwise separate the state. |
| Structural regression or missed simplification | Show the branches, invalid states or hidden state the reshape removes and verify the behavior pin. A cleaner-looking diff without those deltas is not a finding. |

File length belongs to `length`, non-MPL comments to `comments`, and ordinary runtime reach to `blast-radius`. Do not manufacture findings from file-size thresholds, textual similarity, speculative YAGNI or a second generic review pass.

## Completion

Every changed or conflicting authority has one surviving definition. Every applicable risk-map branch is either closed by its named evidence or explicitly unverified. Report the resolved authority, the evidence commands and any remaining unverified assumption. Read reviewer and bot discussion after this independent pass and verify each still-relevant finding rather than adopting it.
