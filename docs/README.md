# Sprawling documentation

[Project home](../README.md) · [简体中文项目主页](README.zh-CN.md)

## Start

| Document | What it answers |
|---|---|
| [`getting-started.md`](start/getting-started.md) | From nothing installed to a city that does work, with every idea met on the way. |
| [`getting-started.zh-CN.md`](start/getting-started.zh-CN.md) | The same guide in Chinese. |
| [`dist/QUICKSTART.md`](dist/QUICKSTART.md) | The release archive's first page: raise a city, open it, send the first task. |

## Use a city

| Document | What it answers |
|---|---|
| [`operating.md`](operate/operating.md) | Steering and stopping work, answering residents, driving a city from a script, and the ways out of what goes wrong. |
| [`logging.md`](operate/logging.md) | What goes in the diagnostic log and what goes in the Ledger, and why the two stay apart. |

<a name="build-on-a-city"></a>

## Integrate agents, tools and scripts

| Document | What it answers |
|---|---|
| [`wire.md`](wire.md) | CLI, frames, answers, exit codes and the generated protocol rosters. |
| [`integrations.md`](integrate/integrations.md) | Any ACP agent as a resident, MCP servers, and CLI control from an existing agent. |

## Measure

| Document | What it answers |
|---|---|
| [`performance.md`](maintain/performance.md) | Monitor counters, reproducible workloads, and the method every published measurement follows. |
| [`benchmarks/README.md`](benchmarks/README.md) | The cross-agent benchmark: its method, its rounds, and the rule that a published round stays as it was. |
| [`benchmarks/methodology.md`](benchmarks/methodology.md) | The binding method of the next round: series coverage, estimators, sizing, schemas and reproduction. |
| [`benchmarks/v0.0.11/README.md`](benchmarks/v0.0.11/README.md) | One published round: preliminary fixed-version Windows observations, their exported samples and their limits. |

## Change the code

| Document | What it answers |
|---|---|
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | Contribution preparation, feature admission and submission, with links to the repository rules. |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | The crates and the dependency rules between them, the seams, and where each kind of change goes. |
| [`verification.md`](maintain/verification.md) | The command table, the CI tiers, forks and scheduled workflows. |
| [`gates.md`](gates.md) | What each machine gate holds, and the rules for changing one. |
| [`testing.md`](maintain/testing.md) | Scenarios first, the three fixture kinds, and the Rust, Lean and adversary split. |
| [`ui-design.md`](maintain/ui-design.md) | The design judgement a new screen starts from. |
| [`frontend-method.md`](maintain/frontend-method.md) | How a screen under `RefRain/` is built, settled and accepted. |

## Reference and policy

| Document | What it answers |
|---|---|
| [`catalog.md`](reference/catalog.md) | Every built-in tool and namespace a resident can be given, what admits each one, and the sentence a model reads about it first. |
| [Project background](reference/background.md) ([简体中文](reference/background.zh-CN.md)) | The goals and design rationale behind the project. |
| [`glossary.md`](glossary.md) | What each word means; the `lexicon` gate holds the code and the documents to these words. |
| [`third-party.md`](third-party.md) | Whose work this stands on, and the licence obligations. |
| [`SECURITY.md`](SECURITY.md) | The private route for reporting a vulnerability, and what is in scope. |
| [`CHANGELOG.md`](CHANGELOG.md) | What each release changed for someone who runs or builds the city. |

One file here is part of the product rather than a document about it: [`dist/QUICKSTART.md`](dist/QUICKSTART.md) is copied into every release archive, so a change to it is a change to what a person runs.

The documents a city writes for its residents live beside the code that writes them, in [`Sprawling/city/templates/`](../Sprawling/city/templates/): they are compiled into the binary, the city writes `City.md` when it is raised and the templates into its buildings, and [`README.md`](../Sprawling/city/templates/README.md) there lists them.
