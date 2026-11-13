---
name: ub-mode
description: "Use before designing, fixing, refactoring, or reviewing code. Align the scenario, choose a surgical or structural change from evidence, establish the contract and failing check, then review and verify the complete diff through the project's own checks."
license: MIT
---

# ub-mode

> Adapted from [unclebob/ub-mode](https://github.com/unclebob/ub-mode) and pstack's principles and playbooks in [cursor/plugins](https://github.com/cursor/plugins), both MIT. Modified for this repository by 2youg1, 2026. The upstream's action log and push-to-main instruction are not adopted; repository authorities own evidence, permissions, and delivery.
>
> The upstream's companion tools — [crapper](https://github.com/unclebob/crapper), [dryer](https://github.com/unclebob/dryer), and [mutator](https://github.com/unclebob/mutator) — are optional aids, not vendored or required here; §4 names where each fits.

Use this loop for a code change. The repository's `AGENTS.md`, specifications, and contribution guides supply the project contract and the commands this skill deliberately does not repeat.

## 1. Align and choose the scope

Start from the real user or agent scenario. A bug starts with its reproduction. State the observable result, the constraints, and the checks that can settle them; restate these before executing a nontrivial task so a wrong reading costs no implementation. Aim at the end state those checks can verify; **a break in the middle is allowed when you planned it and can undo it.**

Read the current diff and preserve work already present. Trace the affected callers, data flow, invariants, and failure paths before choosing a design. Judge maintainability, clarity, runtime cost, and protection of the person's data and machine together; protection is never a trade-off.

Let the mechanism decide how much to change:

- **Surgical:** the design is sound and one causal place explains the defect. Ship only what the reproduction and contract justify; adjacent cleanup and speculative guards stay out.
- **Structural:** scattered conditionals decide one rule, callers compensate for the same invalid state, related fixes keep failing, or the existing seam cannot pin the behaviour. Name the target shape, pin the behaviour, then remove or rewrite the faulty structure. A refactoring request calls for this clarity, not a symptom patch restrained by file count.
- **Evidence changes the request:** when a requested surgical fix meets a structural cause, show the evidence and the smallest coherent structural unit before widening the diff. Keep the rest of the codebase out of it.

Minimal means the **smallest complete change that removes the cause**, not the fewest lines or files. Finish a migration by moving every reader and writer, exercising the production path, and deleting the old authority and its adapters.

Explicit wording beats inference. When two fixes that share one premise fail the same gate, write that premise in one sentence and count which actors hold the imbalance. **Remove the assignment. Do not start another fix until you have done that.** When a premise is wrong and the goal is still reachable, correct the premise and continue; record the corrected premise and every inferred choice for the report.

## 2. Design the change

**Redesign from first principles:** do not bolt a new requirement onto the old design. Ask what you would have built if it had been there from the start, think that through, and deliver it in pieces.

**One decision lives in one place:** give each rule, value, state transition, and decision exactly one authority. Encode invariants in types where possible, make invalid states unrepresentable, and parse outside input once, at the boundary; **do not silence the compiler.** Keep domain decisions independent of I/O and UI.

- **Data shape first:** get the data shape right before you write the logic, and do first whatever helps every later phase.
- **No shared writers:** when actors would write concurrently, do not share state — if they do not need the same mutable thing, give each its own; if they do, allow one writer, and enforce that in the structure.
- **Idempotent operations:** running it twice, or running it after a crash halfway through, ends in the same state.
- **Leave clear code alone:** if the code is already clear and local, leave it.

Prefer a deep module with a narrow interface over a chain of pass-through wrappers. Introduce a trait only where a second implementation already exists, call the underlying API directly, and place behaviour beside the concept that owns it. Take time as a parameter and keep decision order explicit.

Before writing a check, probe, transform, or script, inspect the repository's existing development tools and the module that owns the operation. Reuse or extend one of them; a tool useful across tasks belongs in that project home, while a one-off experiment stays out of the shipped tree. **If the work is not obvious at a glance, build the tool that does it.** Do one unit by hand, then make the tool match that unit. DRY removes repeated facts and work; it is no substitute for a concrete design.

Choose the simplest design that satisfies the whole contract. Remove dead code and accidental complexity before adding machinery. Optimize hot paths from measurements made with the repository's harness rather than intuition.

## 3. Establish the contract and red evidence

Follow the repository's specification workflow before changing production interfaces; load its specification skill where one is named. A structural change first pins the behaviour that must survive through a characterization test, replay, or equivalence check at a real call seam.

For a defect, write the regression check at the seam that exhibits it and watch it fail on the intended assertion. The check calls the code the way its user does and checks the result that user would see. **If it would still pass when every function returned nothing, throw it out. A missing-symbol compile error is not that evidence either. Fix the cause — a guard that only hides the crash is not a fix.** If an honest test is impractical, keep a runnable reproduction and state the remaining verification gap instead of writing a test that cannot fail meaningfully.

## 4. Implement, then challenge the result

Implement the chosen unit until its checks pass; finish one small unit and check it before starting the next. When the implementation exposes a better contract, revise the specification and its reason before continuing.

Review the complete diff against the scenario, invariants, and failure paths. **Tests do not settle design quality.** Repair each finding or record a concrete reason for keeping it:

- a layer, flag, branch, or abstraction that the contract does not need;
- a loop, retry, recursion, or wait without a bound or a credible termination argument;
- duplicated decisions, repeated work, or reuse whose input can have changed;
- unchecked arithmetic, avoidable algorithmic growth, unbounded retention, or hot-path copying;
- an exemption, special case, or widened budget without a checkable reason.

Where a measurement would settle a review finding, clone and run whichever upstream tool fits: crapper scores each function's complexity and coverage — split or test until each is under CRAP 10, and consider whether case statements are worth reducing; dryer ranks structural duplication — eliminate what scores 0.8 or above; mutator mutation-tests the suite — cover the uncovered, kill the survivors, delete the dead code. These tools inform the review; the repository's gates remain the authority.

When the same defect class recurs, **eliminate it through structure, types, or a check whose message names the repair**, rather than adding another instruction for agents to follow; verify a new check actually fails on the real past failure before trusting it.

For refactoring, compare behaviour against the pin and judge whether a reader now traces fewer layers and less hidden state. When a file exceeds 500 lines, evaluate its architecture and file boundaries together; split along responsibilities when that reduces reader load and simplifies maintenance, without treating the threshold as an automatic split rule. Revert a reshape that only moves complexity or adds indirection.

## 5. Verify and deliver

Use the repository's verification guide to run every check affected by the final diff and no unrelated suite by habit. Exercise the real behaviour or artifact when the claim needs more than compilation. Apply the repository's blast-radius and authority reviews before its final check, then rerun the affected checks after every repair.

Before opening a pull request, follow the repository's contribution guide and submission template. Complete its required local checks and, when required, a green fork CI run rather than treating the pull request as the first test run. Commit, push, or publish only within the user's request and project rules; no workflow here pushes directly to the mainline by default.

Keep the main thread small: send the bulk elsewhere, and keep the summary. Find what is safe to remove, confirm nothing still in use needs it, then reclaim the disk. Uncommitted work waits for a decision.

**Stop only for a blocker**, as `AGENTS.md` defines one, and **a single `continue` is never a reason to stop.** Report the commands and evidence first, then the outcome they establish, then the blockers, corrected premises and inferred choices that qualify it, and at most two matters needing attention. **Report decisions and results, not a log of every action.**
