---
name: authority-review
description: "Two-pass audit of a branch or PR diff before it merges, reported side by side and never reranked. Pass one hunts bugs, broken existing behaviour, security holes, developer-experience regressions, feature-gate leaks, and divergence from the recorded decision. Pass two applies one lens to the whole diff: a fact with more than one definition - a value, name, path, port, limit, default, grammar, or enum spelling written twice, so the copies can drift apart - ranked by how close they are to disagreeing. Use for 'review this branch', 'audit this PR', 'deep review', 'single source of truth', or before shipping a diff you do not fully trust."
license: MIT
---

# Authority review

Audit a checked-out branch twice, with different eyes each time, then merge the findings into one verdict.

> Adapted from the Thermos plugin in [`cursor/plugins`](https://github.com/cursor/plugins) (MIT) — the two rubrics, the write-findings-down rule, the prioritisation, and the approval bar — deepened with this configuration's own authority lens. Modified by 2youg1, 2026. The original runs the two rubrics as parallel sub-agents and synthesizes their reports; an agent without sub-agents runs them sequentially here. Sequential is not merely a downgrade: finish pass one and write its findings down **before** reading pass two's rubric, because a correctness verdict carried forward suppresses exactly the findings pass two exists to produce.

## Scope

Report only on code this branch **adds or modifies**. Do not report vulnerabilities or smells in untouched existing code, however tempting. The diff is the subject; the rest of the repository is context you read in order to judge the diff.

Gather before starting: the diff against the merge base, and the full current contents of every changed file. Judging a hunk from its diff context alone produces confident wrong findings.

## Pass one — correctness, security, and the record

Audit for bugs, changes that break existing behaviour, and security vulnerabilities. Be thorough, rigorous, and careful; nothing should slip through.

**Breaking existing functionality.** Codebases have cross-module dependencies where a simple local change interacts subtly with something far away. Trace the side effects of each change through its callers and its data, rather than reading the hunk in isolation.

**Breaking developer experience.** It is easy to break other people's ability to build and run the code locally. Watch for changes to how secrets are read or where they are read from, renamed or newly required environment variables, remapped ports and networking, and new scripts that must be run for existing functionality to keep working. Adding a dependency through the package manager does not count. Adding a *new alternative* way to build does not count. What counts is changing the way developers already build and run.

**Feature-gate leaks.** When the codebase gates features behind flags or internal-only checks, verify that nothing this branch adds escapes its gate. These leaks are subtle; check the gate on every path, not just the obvious one.

**Divergence from the record.** The change claims to carry out a decision, and this configuration keeps decisions in `AGENTS.md`, `ARCHITECTURE.md`, `glossary.md`, the nearest `SPEC.md`, the ADRs, and the Lean models. Report a requirement the record asked for that the change leaves missing or partial, behaviour the change adds that no record asked for, and a requirement that looks implemented and is not. Quote the record's line for each finding; when no record exists, that is itself the finding.

**Intended breakage.** If a high-risk finding is the declared point of the branch — removing a flag, retiring a safeguard, deliberately dropping a feature — and the change is well scoped, do not spend the author's time reporting it. Still report it when you believe the author has not seen the full implications, when they appear to be under-weighting the damage, or when the change looks malicious.

**Do not over-report.** Reporting a medium issue as high destroys the trust that makes the next report useful. Trace each issue end to end and reach real confidence before assigning it a priority.

Write pass one's findings down now, then read the next section.

## Pass two — authority

One lens over the whole diff: every value, name, path, port, limit, default, grammar, and enum spelling needs exactly one authoritative definition that every reader uses. Two definitions of one fact are a defect from the moment they exist, not from the moment they disagree, so an audit that waits for the copies to drift has already lost what it was protecting.

Rank each finding by how close the copies are to disagreeing, worst first.

1. **Already diverged.** The copies disagree today in value or meaning: the UI calls a field `idle_timeout` while the backend enforces it as a total deadline; an error message lists the allowed enum values with one missing.
2. **Can diverge silently.** Several values must agree and nothing enforces it: one value under two constant names in two crates; "unset" written as `0` in one field and `None` in another.
3. **Respelled.** A literal repeats an existing constant; a payload key is written by hand at the writer and at each reader; a well-known identifier is typed out in many files.
4. **Erased.** A failure or decision is replaced by a default: `unwrap_or_default` where absence needed a decision; `let _ =` on a `Result`; an accepted parameter nobody reads.

Six shapes carry most of the findings. Each is a labelled heuristic — "possible Data Clumps" — never a hard violation, and a decision recorded in this repository overrides the shape.

| Shape | What it is | The fix |
|---|---|---|
| Duplicated Code | the same logic shape appears in more than one hunk or file | extract the shared shape; call it from both |
| Shotgun Surgery | one logical change forces edits scattered across many files | gather what changes together into one module |
| Divergent Change | one file is edited for several unrelated reasons, or ad-hoc conditionals land in flows that are not about this feature | split it, or put the logic behind a dedicated abstraction, state machine, or module |
| Data Clumps | the same few fields keep travelling together — a type waiting to be born | bundle them into one type and pass that |
| Primitive Obsession | a primitive or string stands in for a domain concept, or a cast and an optional make the contract indirect | give the concept its own small type; make the boundary explicit |
| Repeated Switches | the same switch on the same type recurs across the change | one map, or one polymorphism, shared by every site |

A script finds classes 2 to 4; only reading finds class 1. Script the literals repeated across files, the constants grouped by value and by name, the `Option` clusters, the wildcard arms, the `let _ =`. Then trace every fact a script flagged, and every setting a person can enter, from where it is set to where it takes effect, comparing each stop on the way. The copy that disagrees is the one no script could see.

**Ambitious about structure.** Do not stop at "this could be a bit cleaner." Look for the restructuring that makes whole branches, helpers, modes, or layers disappear — the move that makes the change feel inevitable in hindsight. When there is a path to deleting complexity rather than rearranging it, push for that path.

**Findings the shapes do not carry.** Each needs an explicit justification from the author, not a shrug:

- The diff preserves substantial incidental complexity when a visible restructuring would delete it.
- The diff pushes a file from under 1000 lines to over 1000 lines. Treat this as a strong smell by default and ask whether the code should be decomposed first; waive it only for a compelling structural reason, and only when the result is still clearly organized.
- A silent fallback papers over an unclear invariant instead of stating it.
- Independent work is serialized for no reason, or related updates can leave state half-applied when an atomic structure is available.

**Preferred remedies**, roughly in descending order of value: delete a layer of indirection rather than polish it; reframe the state model so the conditionals disappear instead of being centralized; move the ownership boundary so the feature becomes a natural extension of something that already exists; turn a special case into a simpler default with fewer exceptions; split a large file into focused modules; replace a condition chain with a typed model or explicit dispatch; separate orchestration from business logic; reuse the canonical helper.

## Verdict

Merge both passes into one report, deduplicated. Findings first, prioritized in this order:

1. Correctness, security, and broken existing behaviour
2. Divergence from the record
3. A fact whose copies have already diverged
4. A fact that can diverge silently, or one that is respelled or erased
5. Structural regressions and missed dramatic simplifications
6. Boundary, abstraction, and file-size problems
7. Legibility

Prefer a small number of high-conviction findings to a long list of cosmetic notes. Do not flood a review with nits while a structural problem sits unaddressed.

Every finding names the failure, cites `file:line`, and states the evidence. An authority finding additionally names the definition that survives, every reader and writer to move onto it, and the copies to delete in the same change; a list of copies that does not say which one wins has not finished the job. A finding you could not trace to confidence is reported as unverified or not at all.

**Approval bar.** Behaviour being correct is not sufficient. Approve only when there is no structural regression, no visible missed simplification, no unjustified file-size explosion, no new spaghetti branching, no fact left with two authorities, no architecture-boundary leak, and no avoidable duplication of a canonical helper.

**Tone.** Direct, serious, demanding about quality; never rude. Do not soften a major finding into a mild suggestion. If the branch makes the codebase messier, say so plainly.

## Last step

Only after both passes are complete, check the PR or MR discussion with `gh` or `glab` for comments from bots or reviewers. Doing this last preserves fresh eyes for the audit itself. Evaluate what others found, incorporate the valid findings you missed, and mark which findings came from the discussion rather than from you.

Never present a finding with unfinished research. "The client has issue X, but this may be handled in the backend" is not a finding when the backend is in this repository and you could have checked.
