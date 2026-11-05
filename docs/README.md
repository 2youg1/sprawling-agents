# docs

The documents for people who use, run or change sprawling, grouped by who reads them first.

| Reader | Document | What it answers |
|---|---|---|
| Someone starting | [`getting-started.md`](getting-started.md), in Chinese [`getting-started.zh-CN.md`](getting-started.zh-CN.md) | From nothing installed to a city that does work, with every idea met on the way |
| Someone using a city every day | [`operating.md`](operating.md) | Steering and stopping work, answering residents, driving a city from a script, and the ways out of what goes wrong |
| An agent or script author | [`wire.md`](wire.md) | CLI, frames, answers, exit codes and generated protocol rosters |
| Someone integrating tools | [`integrations.md`](integrations.md) | Any ACP agent as a resident, MCP servers and CLI control from an existing agent |
| Someone deciding what a building's residents may do | [`catalog.md`](catalog.md) | Every built-in tool and namespace a resident can be given, what admits each one, and the sentence a model reads about it first |
| Someone measuring work | [`performance.md`](performance.md) | Monitor counters, reproducible workloads and measurement provenance |
| Someone reviewing measured workloads | [`benchmarks/v0.0.11/README.md`](benchmarks/v0.0.11/README.md) | Preliminary fixed-version Windows observations, exported samples and comparison limits |
| Someone debugging | [`logging.md`](logging.md) | What goes in the diagnostic log and what goes in the Ledger, and why the two stay apart |
| Anyone | [`glossary.md`](glossary.md) | What each word means; the `lexicon` gate holds the code and the documents to these words |
| Someone changing the code | [`CONTRIBUTING.md`](CONTRIBUTING.md) | Contribution preparation, feature admission and submission, with links to the repository rules |
| Someone changing structure | [`ARCHITECTURE.md`](ARCHITECTURE.md) | The crates and the dependency rules between them, the seams, and where each kind of change goes |
| Someone running or reading a check | [`verification.md`](verification.md) | The command table, the CI tiers, forks and scheduled workflows |
| Someone loosening or touching a gate | [`gates.md`](gates.md) | What each machine gate holds, and the rules for changing one |
| Someone writing or judging tests | [`testing.md`](testing.md) | Scenarios first, the three fixture kinds, and the Rust, Lean and adversary split |
| Someone designing a screen | [`ui-design.md`](ui-design.md) | The design judgement; the method and the approved design stay in `frontend-method.md` |
| Someone changing a screen | [`frontend-method.md`](frontend-method.md) | How a screen under `client/` is built, settled and accepted |
| Anyone checking what is owed | [`third-party.md`](third-party.md) | Whose work this stands on, and the licence obligations |

One file in this directory is part of the product rather than a document about it, so a change to it is a change to what a person runs: [`dist/QUICKSTART.md`](dist/QUICKSTART.md) is copied into every release archive. The documents a city writes for its residents live beside the code that writes them, in [`crates/city/templates/`](../crates/city/templates/): they are compiled into the binary, the city writes `City.md` when it is raised and the templates into its buildings, and [`README.md`](../crates/city/templates/README.md) there lists them.
