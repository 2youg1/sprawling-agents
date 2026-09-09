# Runtime document templates

Once a city is running, agents and the person share a set of markdown files. Their formats live in this directory.

**A template teaches by being present, and a template is a form to fill in rather than a question to answer.** "How to write this well" is carried by the **structure** of the file rather than by instruction in the prefix: reconciliation can only judge a line after it is written, while a form makes a small local model write it correctly the first time. That matters most for confidential work, which only local models are allowed to do.

Templates land in the building root with `CreateBuilding`; the two hall files land in the city's reserved directory when the city is raised. A resident opens the file and sees the skeleton.

| File | Location | Written by | In git | What it is |
|---|---|---|---|---|
| `SPEC.md` | building root | agent, person | yes | What the project is and the decisions it holds, before the code. The project's guide: it travels with every clone. |
| `BUILDING.md` | `<building>/.sprawling/` | person | yes | The rules of the building. Evaluated into a BuildingPolicy. Outside every write domain, so the agents it governs can only read it. |
| `Roadmap.md` | building root | agent, through `plan` | no | The single denominator for plan and progress. A city has no second roadmap and no todo tool. |
| `Memo.md` | building root | agent | no | Decisions and corrections. The outline is rewritten in place; the body is append-only. |
| `Handoff.md` | building root | agent | no | The recovery package, five sections. **Not a new authority.** |
| `JOB.md` | room | person or dispatcher | no | The task for this session. The agent **reads it and leaves it unchanged**. |
| `URBANITE.md` | with the resident | person | no | Who this resident is and how they work. |
| `MAYOR.md` | `<city>/.sprawling/` | person | — | Who the Mayor is: the city's planner, writing Markdown only. |
| `CLERK.md` | `<city>/.sprawling/` | person | — | Who answers approvals when the person does not: what it allows, refuses, and leaves to the person. |
| `BUILDING-hall.md` | `<city>/hall/.sprawling/BUILDING.md` | the city | — | The rules City Hall is raised with. The one `BUILDING.md` a person does not write, because what the two residents serving every building may do is a property of the city. |

The three files in the building root and the rules beside them are together called the **Spine**. Long-running work stays continuous through these files rather than through session memory — this is what "no continuous self" looks like at the file layer.

**Two of them are the project's, the rest are the city's.** `SPEC.md` and `BUILDING.md` are committed with the project, because a decision or a rule that only the city remembers is lost to the next clone. The roadmap, the memo, the handoff and the rooms are the guide for whoever works here next, and the `.gitignore` the city writes when it raises or adopts a building keeps them out of the project's history. A building's `.sprawling/` holds its rules and its configuration and is committed with it; the city's own `.sprawling/` — the Ledger, the store, the hall's two identities — is inside no project and in no project's git.

**The Roadmap table is parsed** (`kernel::spine`): six columns, five status words, and an evidence column that holds a Locator. A row that does not parse is displayed and excluded from the completion figure, so a table can never quietly inflate its own progress. The hall's `Roadmap.md` is the same form one level up: a row there is a building's share of the city's plan, and its evidence is the building's own roadmap.
