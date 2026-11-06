# docs

The documents for people who use, run or change sprawling. They are grouped by the question a reader arrives with; the first group is the way in, and the last one is where to look a word up.

This file is the index of the documents below. The Chinese project landing page is [`README.zh-CN.md`](README.zh-CN.md); it is the translation of the repository's [`README.md`](../README.md), not of this index.

## Start

| Document | What it answers |
|---|---|
| [`getting-started.md`](getting-started.md) | From nothing installed to a city that does work, with every idea met on the way. |
| [`getting-started.zh-CN.md`](getting-started.zh-CN.md) | The same guide in Chinese. |
| [`dist/QUICKSTART.md`](dist/QUICKSTART.md) | The release archive's first page: raise a city, open it, send the first task. |

## Use a city

| Document | What it answers |
|---|---|
| [`operating.md`](operating.md) | Steering and stopping work, answering residents, driving a city from a script, and the ways out of what goes wrong. |
| [`logging.md`](logging.md) | What goes in the diagnostic log and what goes in the Ledger, and why the two stay apart. |

## Build on a city

| Document | What it answers |
|---|---|
| [`wire.md`](wire.md) | CLI, frames, answers, exit codes and the generated protocol rosters. |
| [`integrations.md`](integrations.md) | Any ACP agent as a resident, MCP servers, and CLI control from an existing agent. |
| [`catalog.md`](catalog.md) | Every built-in tool and namespace a resident can be given, what admits each one, and the sentence a model reads about it first. |

## Measure

| Document | What it answers |
|---|---|
| [`performance.md`](performance.md) | Monitor counters, reproducible workloads, and the method every published measurement follows. |
| [`benchmarks/README.md`](benchmarks/README.md) | The cross-agent benchmark: its method, its rounds, and the rule that a published round stays as it was. |
| [`benchmarks/methodology.md`](benchmarks/methodology.md) | The binding method of the next round: series coverage, estimators, sizing, schemas and reproduction. |
| [`benchmarks/v0.0.11/README.md`](benchmarks/v0.0.11/README.md) | One published round: preliminary fixed-version Windows observations, their exported samples and their limits. |

## Change the code

| Document | What it answers |
|---|---|
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | Contribution preparation, feature admission and submission, with links to the repository rules. |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | The crates and the dependency rules between them, the seams, and where each kind of change goes. |
| [`verification.md`](verification.md) | The command table, the CI tiers, forks and scheduled workflows. |
| [`gates.md`](gates.md) | What each machine gate holds, and the rules for changing one. |
| [`testing.md`](testing.md) | Scenarios first, the three fixture kinds, and the Rust, Lean and adversary split. |
| [`ui-design.md`](ui-design.md) | The design judgement a new screen starts from. |
| [`frontend-method.md`](frontend-method.md) | How a screen under `client/` is built, settled and accepted. |

## Reference and policy

| Document | What it answers |
|---|---|
| [`glossary.md`](glossary.md) | What each word means; the `lexicon` gate holds the code and the documents to these words. |
| [`third-party.md`](third-party.md) | Whose work this stands on, and the licence obligations. |
| [`SECURITY.md`](SECURITY.md) | The private route for reporting a vulnerability, and what is in scope. |
| [`CHANGELOG.md`](CHANGELOG.md) | What each release changed for someone who runs or builds the city. |

One file here is part of the product rather than a document about it: [`dist/QUICKSTART.md`](dist/QUICKSTART.md) is copied into every release archive, so a change to it is a change to what a person runs.

The documents a city writes for its residents live beside the code that writes them, in [`crates/city/templates/`](../crates/city/templates/): they are compiled into the binary, the city writes `City.md` when it is raised and the templates into its buildings, and [`README.md`](../crates/city/templates/README.md) there lists them.
