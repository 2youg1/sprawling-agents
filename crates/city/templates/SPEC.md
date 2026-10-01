# SPEC.md — <project name>

> What this project is and the decisions it holds, written before the code that follows them. It is committed with the project, so anyone who reads the code has its reasons; `Roadmap.md`, `Memo.md` and `Handoff.md` are not committed, so a decision that only lives there is lost to the next clone.
>
> The form is the condensed version of the SPEC every sprawling crate is built from. Fill the sections in order; a section you cannot fill yet says so rather than being deleted, so the gap stays visible to the next reader.
>
> **A change that contradicts a decision below changes the decision first, with its reason, in the same change.**

<what>
(One paragraph, then the units of work that can be accepted separately — each one a row in the acceptance table.)
</what>

<acceptance>

| Unit | Done means |
|---|---|
| | (A check somebody can run, and its expected result.) |

</acceptance>

<assumptions>
- **Assumed**: (what the design takes for granted; each one is a place the design can be wrong.)
- **Open**: (a question with the two answers it could take and what each would change.)
</assumptions>

<authorities>

| Fact | Where it is settled |
|---|---|
| | (The document, standard, or file that decides it. Prefer a primary source, and date the last time it was checked.) |

</authorities>

<names>
(One word per concept, used everywhere. Retired words go here with their replacement, so a reader of an old commit is not lost.)
</names>

<boundaries>
(What this project owns, what its neighbours own, and the seam between them — the interface, not the wish.)
</boundaries>

<interfaces>
(The types, commands, files or endpoints, written before they exist, with what each promises and what it refuses. Code that differs from this section is wrong until this section is changed.)
</interfaces>

<failure>

| What is refused | Code | What the caller can do instead |
|---|---|---|
| | | |

</failure>

<dependencies>

| Dependency | Version | Why this one |
|---|---|---|
| | | |

</dependencies>

<values>

| Value | Where | Why this number |
|---|---|---|
| | | |

</values>

<tests>
(Which checks run before a change lands, and the command that runs them. A rule a machine holds is listed with the machine; a rule a reviewer holds is listed as such.)
</tests>

<decisions>

<decision id="S-0001">

**(one-line title)**

- **Date**:
- **Decided**:
- **Instead of**: (the alternative that was rejected, and why it lost.)
- **Replaces**: (the decision this one supersedes; empty otherwise.)

</decision>

</decisions>
