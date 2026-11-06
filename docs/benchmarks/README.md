# Benchmarks

The cross-agent benchmark measures sprawling against other agent harnesses on one machine class and one build. This directory holds its method and one directory per round.

| File | What it is |
| --- | --- |
| [`methodology.md`](methodology.md) | The binding method: series registry, coverage contract, estimators and their parameters, sizing rules, schemas, and the commands that reproduce a round. A round directory follows it and never redefines it. |
| [`v0.0.11/`](v0.0.11/README.md) | A published round: the fixed-version Windows observations, their exported samples, the manifest and the historical verifier. |

A round directory holds its own `README.md`, `manifest.json`, `results.csv`, `exports.json` and `report.html`, and its prose states what that round measured, what it may claim and what it deferred. The method owns the definitions once; a round owns only its observations and its claims.

Published rounds stay as they are. A later change to the method is a new analysis version with its reason, not an edit to a round already read, so the numbers a reader finds here keep the analysis they were produced under.

[`methodology.md`](methodology.md#reproduce-a-published-round) names the analyzer that validates, recomputes and renders a round; it is not written yet, so no new round is publishable until that stage lands. The v0.0.11 subset keeps its own `verify.py`, which recomputes that round's exported numbers and nothing else.
