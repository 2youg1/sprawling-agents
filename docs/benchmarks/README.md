<!-- coderabbit-ai-generated:v1 -->

<!-- coderabbit-ai-generated-section:v1:a2f51dc511a6c8e6d3dcfe2ba965738f64e41e9e8ccdb35e531be0135d090c07 -->

# Benchmarks

The cross-agent benchmark measures sprawling against other agent harnesses on one machine class and one build. This directory holds its method and one directory per round.

| File | What it is |
| --- | --- |
| [`methodology.md`](methodology.md) | The binding method for future rounds: series registry, coverage contract, estimators and their parameters, sizing rules, schemas, and the commands that reproduce a round. Future round directories follow it and never redefine it; v0.0.11 retains its historical method. |
| [`v0.0.11/`](v0.0.11/README.md) | A published round: the fixed-version Windows observations, their exported samples, the manifest and the historical verifier. |

A future round directory holds its own `README.md`, `manifest.json`, `results.csv` and `exports.json`, and its prose states what that round measured, what it may claim and what it deferred. HTML is an optional later derivation, not a required artifact. The method owns the definitions once; a round owns only its observations and its claims.

v0.0.11 retains its published artifacts and does not require `exports.json`.

Published rounds stay as they are. A later change to the method is a new analysis version with its reason, not an edit to a round already read, so the numbers a reader finds here keep the analysis they were produced under. Each future round records the immutable full Git SHA `analysis_revision` in its manifest, along with its toolchain and input digests. Follow [the checkout and analyzer commands](methodology.md#reproduce-a-published-round) at that revision to reproduce it; retain that commit in repository history. For historical v0.0.11, check out the commit that published its directory and run the included `python3 docs/benchmarks/v0.0.11/verify.py`; this only provides the subset checks described below.

[`methodology.md`](methodology.md#reproduce-a-published-round) names the analyzer that validates, recomputes and renders a round. The v0.0.11 subset keeps its own `verify.py`, which checks the CSV digest, row counts, coverage and selected outcomes, recomputes available sample medians/maxima/P99 and shared coding totals, and prints a partial statistical summary. It does not check `known_input_tokens` in `manifest.json` or compare computed values with report numbers; it does not verify all exported values.


<!-- /coderabbit-ai-generated:v1 -->