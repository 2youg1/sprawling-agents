# BUILDING.md — hall

> The rules of City Hall. Unlike every other `BUILDING.md`, this one is written by the city itself when the city is raised, and it is fixed: the two residents who live here serve every other building, so what they may do is a property of the city rather than a local convention.
>
> A person may still edit it. What the person may not do is get a City Hall that builds, because nothing here has an `exec` tool to build with.

## What this building does

City Hall holds the city's own plan and the two residents that serve every other building. The Mayor turns an idea into work the buildings can pursue; the clerk answers approvals the person delegated. Neither of them holds a project: work that belongs to a subject belongs to a building raised for that subject.

## confidential

`confidential: false`

> City Hall reads every building's documents and writes the city's plan. Making it confidential would stop it reading the evidence it exists to read.

## Write domains

`write: documents`

- `hall`

> Documents reach means Markdown files only, and never a plan file. The plan has one editing entrance, the `plan` tool, which keeps the six-column table readable; an `edit` that reached `Roadmap.md` would be a second authority over the same rows.

## Reading-room admission

(Which SKILLs City Hall can read. Name them; a list of names is checkable and "as needed" is not.)

## How work is done here

The Mayor reads before it writes, records what it decided in `Memo.md`, and reports through `signal`. The clerk answers in the same three parts a Gate uses and writes its reason into the record. Evidence comes from buildings: nothing here runs, builds, tests or commits.
