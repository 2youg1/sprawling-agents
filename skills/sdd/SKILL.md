---
name: sdd
description: "Spec-first programming workflow: write the component's SPEC.md before its code, then implement exactly what the SPEC states and keep the two in step. Load before starting programming work that will be merged."
license: MPL-2.0
---

> **Provenance and license.** The original of this skill is 2youg1's own Chinese-language open-source skill, published under AGPL-3.0-or-later. This file is its English translation and adaptation by the same author, and it carries MPL-2.0 here, the license of the rest of this repository; the original remains AGPL-3.0-or-later.

# SDD Workflow

<principle>

This skill builds on `apostle-artifacts-loops`: it inherits that skill's "documentation first" principle and follows Spec-Driven Development. Before writing code for a component, think divergently first, then write a standalone SPEC.md for that component as specified below, so the user can understand it and later maintainers can keep it in step.

Zero references: this SKILL.md is the whole skill — it ships no reference files, scripts, or templates. A SPEC.md belongs to the project workspace; lay the documents out along the code's structure.

Before writing anything, put the project's authorities in your context: `AGENTS.md`, the authority documents (such as `CONTEXT.md`, `ARCHITECTURE.md`), ADRs, and the neighboring modules' documentation and code. Inherit their vocabulary and conventions.

Name each document `component-name + SPEC.md`, so one directory does not fill with indistinguishable SPEC.md files. Write a SPEC.md only for code that will be merged in this project — not for tests, prototypes, or demos — and when implementing code that has a SPEC.md, write only the comments that are strictly necessary.

When the code you are working on already has a SPEC.md, read it in full first. For improvements, additions, removals, and bug fixes alike, change the SPEC.md first, then write the code.

</principle>

<discipline>

## Execution discipline

- The SPEC.md is written before any code, and the implementation follows the SPEC.md exactly — no more, no less. On a deviation, either fix the code, or update the SPEC.md first and state the reason.
- When the code is complete, check it against the SPEC.md: every step's implementation should match the decision and its reason in the document. Finding a better way? Update the document with the user first.
- When the code is complete, verify against the document's testing section and record the results briefly.
- On completion, walk the "Documentation sync" section and remind the user to update every document it lists, writing the actual verification results down clearly.
- When you reach a point that must be reviewed or clarified by the user, deliver the SPEC.md or the code, then stop and align with the user in the form they prefer.

</discipline>

<sections>

## Section order of the SPEC.md

The SPEC.md uses a numbered list in the order below, written in a precise, clear, readable style with multi-level headings and other structure:

1. **Requirements breakdown** — pin the requirement down and split it into the smallest units that can each be completed and accepted independently.
2. **Acceptance criteria** — for each smallest unit, an observable, testable definition of done.
3. **Assumptions and ambiguity** — list the ambiguities in the requirement and the assumption chosen for each; where the "Authoritative sources" section can settle one, verify there first.
4. **Current-state analysis** — where existing code is involved, analyze it first: one hand on the code's logic, one on its measured behavior in time, memory, and the like.
5. **Authoritative sources** — cite official documentation or project documents wherever possible; confirm the design intent, follow the same concepts and style, and settle the exact variable names and what is to be quoted.
6. **Naming** — module, type, and variable names follow the project's vocabulary and domain documents; never coin a synonym.
7. **Module boundaries** — draw the module boundaries, dependencies, and data flow clearly.
8. **Interfaces first** — design the public interfaces and type signatures first, and make invalid states unrepresentable in the types rather than backing into runtime checks.
9. **Workflow** — analyze the module's complete workflow from entry to exit, top down.
10. **Implementation logic** — split the implementation into steps along the development workflow, and for each step state what to do and why: the key decisions and their trade-offs, complexity and performance considerations, and why it beats the alternatives — enough for a reader to judge the code's quality from the document.
11. **Boundary enumeration** — enumerate extreme inputs, exceptional paths, and concurrency conflicts.
12. **Error handling** — for each error class define the strategy: who catches it, how it propagates, and what form the caller or user sees.
13. **Dependency choices** — for every new external dependency, state the reason for the choice, its alternatives, and its maintenance cost.
14. **Hard-coding declarations** — where anything is hard-coded, explain the intent and the consequences.
15. **Impact surface** — list the callers, data, and configuration this change touches, and mark the paths that need regression verification.
16. **Tests and constraints** — list the tests the module needs to run correctly and the constraints that must hold.
17. **Documentation sync** — list the documents that must change on completion, so later maintainers can understand the change and nothing that needs changing is missed.

</sections>
