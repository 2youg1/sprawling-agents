---
name: sdd
description: "Specification-first development with formally verified Lean contracts. Use before designing, implementing, changing, or delegating production code, and when migrating an existing Markdown specification to Spec.lean."
license: MPL-2.0
---

> **Provenance and license.** The original of this skill is 2youg1's own Chinese-language open-source skill, published under AGPL-3.0-or-later. This file is its English translation and adaptation by the same author, and it carries MPL-2.0 here, the license of the rest of this repository; the original remains AGPL-3.0-or-later.

# SDD Workflow

<principle>

Translate the requirements and decisions already agreed upon by the user and the primary agent into a module specification whose substantive content is formally verified Lean code, so that subsequent implementers can distinguish binding decisions from implementation details left to their discretion.

The specification precedes the production implementation. Constructing, executing, and proving Lean models are activities within specification design; a model that serves as a specification is a maintained project artifact.

Within a specification, natural language occurs only in Lean comments, where it records sources, rationale, assumptions, open questions, verification boundaries, and relationships to other documents; express behavioral requirements that admit formalization as definitions, types, contracts, and proofs, rather than substituting comments for these declarations.

Follow the project's Lean module organization and maintain a `Spec.lean` entry point for the module; where decomposition is necessary, organize definitions and proofs through imports, retaining one authoritative definition of each rule.

Preserve the seventeen responsibilities below in their stated order, organizing them through numbered section comments in `Spec.lean`, with references to declarations in imported modules where the material is substantial. If a responsibility does not apply, explain why in its section comment rather than introducing vacuous declarations merely to complete the structure.

</principle>

<context>

Before designing or modifying a specification, read `AGENTS.md`, the architecture documents, the glossary, relevant decision records, existing specifications, Lean models, and neighboring modules, and adopt their established concepts and conventions.

Distinguish user decisions, established project constraints, and agent inferences, recording their sources in the comments accompanying the relevant declarations; retain inferences as explicit assumptions rather than presenting them as matters already agreed upon.

Reuse existing definitions and verification mechanisms. When a design conflicts with an established decision, revise that decision and its rationale before changing the specifications and implementations that depend on it.

</context>

<alignment>

Before delegation, examine every decision that affects acceptance, including normal results, rejection conditions, the state after failure, operation ordering, observable effects, and architectural constraints already selected.

Identify reasonable alternative interpretations that still satisfy the current specification but produce different results, and classify each as permitted, excluded, or unresolved by reference to the existing record of agreement.

If two approaches both satisfy the specification but the existing record of agreement explicitly excludes one, first add the constraints that distinguish them, and only then delegate implementation work that depends on that decision.

Record permitted differences as implementation discretion; express excluded differences as formal constraints or explicit engineering checks, preserving the reasons for their exclusion.

An unresolved question blocks the relevant work only if it affects the contract or the task's outcome; present concrete alternatives and their consequences, and continue work that does not depend on that decision.

</alignment>

<sections>

1. **Requirements breakdown**
   Decompose requirements into independently acceptable units and associate them with domain objects, operations, and properties; record each requirement's source and scope in comments.

2. **Acceptance criteria**
   Express completion conditions through observable behavior, preconditions, postconditions, and theorems, covering both required normal behavior and behavior that must be rejected; state how the production implementation will be checked for conformance.

3. **Assumptions and ambiguity**
   Make assumptions relevant to proofs explicit as model parameters or theorem hypotheses, explaining their justification in comments; record unresolved questions and the consequences of alternative answers, without replacing an unresolved requirements decision with an axiom.

4. **Current-state analysis**
   For an existing implementation, identify how its actual behavior corresponds to the target contract, constructing a model of the current behavior where necessary; record source locations, performance measurements, known discrepancies, and supporting evidence in comments.

5. **Authoritative sources**
   Cite precise locations in requirements, standards, project documents, and upstream definitions in comments, identifying the declarations they constrain; obtain established facts from their authoritative locations rather than transcribing a second definition.

6. **Naming**
   Use the project's vocabulary in Lean declarations; where names differ across languages, record an explicit correspondence and preserve the meaning of each concept.

7. **Module boundaries**
   Express the module's responsibilities through types, interfaces, and dependency relationships; separate external effects from the environment model explicitly, and verify production-layer restrictions through the corresponding structural checks.

8. **Interfaces first**
   Define inputs, outputs, errors, and invocation conditions before defining behavior; exclude invalid states through types wherever possible, and express the remaining requirements as explicit contracts.

9. **Workflow**
   Describe the flow from entry to exit through transition functions or behavioral relations; preserve permitted choices in concurrent or otherwise nondeterministic behavior, rather than imposing an ordering chosen incidentally by the reference model.

10. **Implementation logic**
    Provide reference definitions and proofs sufficient to explain and verify the behavior; record substantive trade-offs, alternatives, and implementation discretion in comments. State the computational model underlying complexity claims, and cite measurement evidence for claims about actual performance.

11. **Boundary enumeration**
    Represent extreme inputs, invalid operations, faults, and concurrent interleavings, and examine these boundaries through properties and scenarios that distinguish relevant cases; passing a finite set of scenarios does not establish a universal property.

12. **Error handling**
    Define failure, propagation, recovery, and caller-observable results through error types and state transitions; which state is preserved or changed after failure is part of the contract.

13. **Dependency choices**
    Record each dependency's role, the rationale for its selection, alternatives, and maintenance costs in comments; refer to the project manifests for concrete versions, and incorporate behavioral assumptions into the model.

14. **Hard-coding declarations**
    Explain the basis, scope, and consequences of changing each fixed value; retain one authoritative definition of each value, and maintain correspondence between model and implementation through established generation or consistency checks.

15. **Impact surface**
    Record which callers, data, configuration, and properties are affected by changes to the public contract, referring to the relevant declarations and verification entry points; determine impact from actual dependencies rather than maintaining a task list for a single implementation effort.

16. **Tests and constraints**
    Discharge formal obligations through proofs, list implementation-conformance checks, integration checks, and other acceptance commands, and distinguish proved properties, test coverage, and environmental assumptions.

17. **Documentation relationships**
    Maintain relationships to architecture documents, the glossary, decision records, neighboring specifications, and operational documentation in comments, identifying the referenced locations, the content connected by each relationship, its authoritative source, and the changes that require the relationship to be re-examined.
    These records describe persistent dependencies, rather than a checklist of documents to update after completing the current task.

</sections>

<migration>

Migrate by module or by a specification scope that can be switched independently, allowing different modules to occupy different migration stages; each scope has exactly one effective behavioral specification at any given time.

1. **Inventory the existing Markdown specification and its consumers.**
   Cover all seventeen responsibilities, identifying behavioral requirements, design decisions, assumptions, examples, acceptance checks, and the documents, scripts, and agent instructions that consume the existing specification.

2. **Establish a migration correspondence.**
   Map each existing requirement to a Lean declaration, an accompanying comment, or a referenced existing authority.
   Resolve omissions, contradictions, and matters that cannot be confirmed explicitly; record semantic changes as separate decisions rather than incorporating them into a format conversion without acknowledgment.

3. **Construct and verify the candidate specification.**
   Reuse existing models where possible, translate behavioral requirements into checkable definitions and contracts, move rationale and documentation relationships into comments, and complete the required proofs and implementation-conformance checks.

4. **Identify the authority during the transition.**
   Before the switch, the still-applicable content of the existing specification and its existing authority relationships remain in force, and the candidate Lean specification is marked as undergoing migration.
   Incorporate relevant changes made during the transition into the migration work; implementers must not be left to choose between two specifications.

5. **Complete the switch within one change-set.**
   Verify the correspondence, update project conventions and all active references, designate the Lean specification as the authority for that scope, and remove the previous Markdown specification.
   Preserve historical versions through version control; before removal, place rationale that remains relevant in Lean comments or existing documents explicitly referenced by those comments.

6. **Check the migration completion criteria.**
   Every previous requirement has been carried forward or has an explicit decision authorizing its revision; the Lean specification and its dependencies are included in the build and pass verification; implementation-conformance checks pass; all active references have been switched; and the repository retains neither the previous Markdown specification for that scope nor a second definition of its behavior.

If these completion criteria are not met, report the specific outstanding items and retain the migration status rather than declaring the transition to `Spec.lean` complete.

</migration>

<delegation>

When delegating, provide the specification's path and version, the task scope, required related material, permitted modification locations, and acceptance commands, so that the subagent can obtain the complete context independently.

The subagent selects an implementation within the contract's permitted scope; when an unresolved question affects the contract, it submits concrete alternatives and their consequences.

Unless the task explicitly includes specification design, the subagent implements the existing contract; when it finds a contradiction or an unsatisfiable requirement, it submits evidence and proposed revisions for the primary agent, which retains the context of the agreement, to resolve.

The primary agent handles revisions under the authorization already given, consulting the user only on matters that remain undecided or exceed that authorization.

</delegation>

<verification>

Check separately whether the specification faithfully captures the agreed requirements, whether the model satisfies the contract, and whether the production implementation conforms to the model, recording the evidence for each.

Acceptance requires the core behavior to be formalized and the corresponding proofs completed; a file containing only type declarations, natural-language comments, string descriptions, or executable examples does not constitute a verified specification.

Check that initial states and required normal behavior are realizable, preventing vacuous guarantees obtained through unsatisfiable preconditions, unreachable states, or rejection of every operation.

Include all effective specifications and their dependencies in the project build, with proofs that do not depend on `sorry`, `admit`, or unreviewed axioms; report the remaining environmental assumptions and behavior outside the model's coverage explicitly.

Perform conformance checks appropriate to the production implementation's language and risks, distinguishing behavioral comparison tests from implementation-refinement proofs; a proof of a Lean model must not be represented as a proof of the Rust implementation.

Review contract changes at acceptance, verifying that changes to guarantees, preconditions, and scope are justified and that requirements have not been altered to conceal implementation deviations.

</verification>

<maintenance>

For each modification, first determine whether it changes the contract; for a behavioral change, update the specification and its rationale before modifying the production implementation, updating affected proofs and conformance checks in the same change-set.

Treat proof reorganization and internal refactoring that preserve the contract as ordinary maintenance, without manufacturing specification changes that serve no purpose.

Use the relationships in section 17 to identify affected documents and maintain the content actually affected within the same change-set; the specification describes the currently effective contract rather than accumulating records of individual implementation efforts.

</maintenance>
