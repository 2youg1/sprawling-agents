# sprawling-benchstat

The analyzer for the benchmark rounds under [`docs/benchmarks/`](../../docs/benchmarks/README.md): it validates one round's manifest and raw observations, computes every statistic the method binds, publishes the canonical `exports.json`, and renders the round report's generated section from those exports. Generation and verification are one implementation, so a published round is checked against the analyzer that produced it rather than against a second statistics implementation. The Cargo package is published as `sprawling-benchstat`, while its library and command are both named `benchstat`. It depends on no product crate, and its manifest writes `role = "tool"`, so it enters neither the `sprawling` binary nor a release archive.

## The parts

`manifest` reads the method, the analysis revision, the fixed parameters, the coverage rows, the claims and the cell plans, and refuses a missing or contradictory field. `observation` reads the 28-column header and gives every row its cell identity. `estimators` is pure: nearest-rank quantiles, tails, OLS, Wilson and Newcombe, Pettitt, Holm, dip and KDE. `resampling` holds the SHA-256 streams, Politis–White block lengths, the moving-block bootstrap, BCa and the jackknife. `analysis` groups observations by the explicit cell key and writes each statistic with its status, estimator, parameters and seed. `report` renders canonical JSON and the Markdown section from those values. `round` reads a round directory, checks the digest and publishes through `.part` files. `main` is the three commands and their exit codes, with no analysis in it. In [`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 9's vocabulary, `manifest`, `observation` and `error` are values, `estimators`, `resampling` and `shape` are decisions, `analysis` and `report` are projections, and `main` and `round` are adapters. It declares no seam and implements none: it is one command, not a port.

## The round it reads

A round directory holds `manifest.json`, `results.csv`, `README.md`, and, once exported, `exports.json`. `manifest.json` pins `schema_version = 1`, the 40-hex `analysis_revision`, the method and estimator versions, the seed string, the fixed parameters (`resamples = 10000`, `confidence = 0.95`, `alpha = 0.05`, `power = 0.8`, `bootstrap_unit = "cohort"`), the SHA-256 of `results.csv` in `input_hashes`, the coverage rows, the claims and the cell plans. `results.csv` is the 28-column long-form table [`methodology.md`](../../docs/benchmarks/methodology.md) defines, one row per observation. `README.md` holds the round's prose and exactly one `<!-- benchstat:begin -->` … `<!-- benchstat:end -->` pair around the section the analyzer owns; everything outside the pair is preserved.

The analyzer reads no clock, environment variable or network. A missing manifest field, a series outside the design registry, a repeated identifier, an orphan request span, a non-finite value or a CSV digest that no longer matches its pin is refused before any statistic is computed. A cell with no pre-registered plan is still described: its `sizing` and `shape` are exported as unavailable with the reason, and its `ranking` is `unavailable`, because no plan registered an independent unpaired contrast.

The one round already in the tree, [`docs/benchmarks/v0.0.11/`](../../docs/benchmarks/v0.0.11/), predates this schema — its CSV has 25 columns where 28 are required, and its manifest lacks the analysis revision, the fixed parameters and the input digest — so no command here loads it; the two examples below are how its numbers are rechecked.

## The three commands

Install the published package with `cargo install sprawling-benchstat --locked`, or build this checkout with the first command below:

```bash
cargo build --locked -p sprawling-benchstat

target/debug/benchstat export docs/benchmarks/<round> [--write]
target/debug/benchstat verify docs/benchmarks/<round>
target/debug/benchstat report docs/benchmarks/<round> [--write]
```

`export` validates the round, computes every cell and prints the canonical exports JSON to stdout. With `--write` it publishes `exports.json` and the README's generated section, each through a `.part` file that is flushed and renamed into place, so a failure leaves the previous publication intact and names the temporary path. `report` re-renders only the generated section from `exports.json` and never reopens `results.csv`; without `--write` it prints the section, and with `--write` it rewrites that section alone. That is how a prose or layout change reaches a published round without touching a number. `verify` recomputes the whole export from `manifest.json` and `results.csv` and compares `exports.json` and the generated section byte for byte; it writes nothing. A hand-edited number, a missing artifact or a moved input digest fails with `E_VERIFY_MISMATCH`.

Exit code 0 is success, 1 a rejected round or a verification mismatch, and 2 a usage, I/O or internal numeric failure. Every failure reads `CODE action: subject; recovery`, with `E_INVALID_INPUT`, `E_VERIFY_MISMATCH`, `E_USAGE`, `E_IO` or `E_NUMERIC`. A round is publishable only when `export`, `verify` and `report` succeed on its committed inputs.

## The two outputs, and why two runs agree

`exports.json` is the machine authority for every number: sorted keys, integer counts, shortest round-trip floating point, LF line endings and no timestamp. The README's generated section is rendered only from those values, so the report cannot hold a number the export lacks, and every deferred or failed coverage row appears there rather than disappearing. The prose around the section stays the round author's; [`methodology.md`](../../docs/benchmarks/methodology.md) remains the authority for the fields, estimators, sizing rules and publication policy, and this package restates none of them.

The bytes are a function of the inputs. Observations are grouped by an explicit cell key in `BTreeMap` order, each resample stream is SHA-256 over the length-prefixed `(seed_string, round, cell key, statistic)` with rejection sampling, and a moving block stays inside one round and cohort. `verify` therefore proves the published bytes, not the analyzer's memory of them.

## What it will not claim

`dip` is exported as a descriptive statistic; `dip_p` is always exported as `null` with status `unavailable` and estimator `dependent_unimodal_null_required`, because no validated dependent-null calibration exists. Resampling the observed cell is not a null, and iid uniform draws indexed by sample size do not control error for correlated observations, so neither is accepted as one. The report keeps mode centres, antimodes and shares as an unflagged descriptive summary and marks centre, cut and share intervals `not_applicable_without_calibrated_multimodality`. No inferential multimodality claim is available until a calibration is supplied and validated; Holm cannot repair an uncalibrated p-value. The analyzer produces no HTML either: a page may be derived from the report and exports later, and it is never an input to verification.

## Constraints

The crate has no features, so nothing is optional at build time. It names no product crate and no network, clock, environment or OS-entropy source; every map it renders is a `BTreeMap`, because a hash order would move a byte. The KDE convolution uses `rustfft` with its default features off, so a platform's SIMD is not one of the inputs. An input may hold at most 1,000,000 rows, and nothing is preallocated from a capacity the input declares. Only a `--write` run touches a file, each through a `.part` file flushed and renamed into place; `verify` writes nothing at all.

## Who uses it

A round author runs the three commands on a round directory, and a reviewer runs `verify` against what was committed. No package in this workspace depends on it: `cargo tree --workspace -i sprawling-benchstat -e normal --depth 1` prints the package alone.

## Specification

The SPEC is [`Spec.lean`](Spec.lean), with its part under [`spec/Estimators.lean`](spec/Estimators.lean); [`docs/ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 11, *Specifications in Lean*, describes that layout. Section 1 lists the modules, section 3 fixes the manifest facts the analyzer assumes, section 8 the command line, its exit codes and the fields a manifest must carry, and section 16 names the tests that hold what the proofs leave open. The Lean part proves that a nearest-rank index stays inside a non-empty sample, that a selected block length stays within `1..n`, and that a degenerate pilot's next target grows. The SPEC is written in Chinese with concept names in English, as the *Language* section of [`AGENTS.md`](../../AGENTS.md) requires.

## Checking it

```bash
cargo nextest run -p sprawling-benchstat              # the tests; `just test` runs them too
cargo clippy -p sprawling-benchstat --all-targets --all-features -- -D warnings
cargo publish -p sprawling-benchstat --dry-run --locked  # the packaging and build a crates.io publish makes, without uploading
lake build crates.benchstat.Spec                       # the SPEC's proofs; `just models` builds every specification
```

`tests/round.rs` writes a round into a temporary directory, exports it, publishes it, verifies it, and then hand-edits the generated section: `verify` fails with `E_VERIFY_MISMATCH`. The two examples are a one-off cross-check of the historical v0.0.11 subset, not part of reproducing a published round: `examples/convert_v0011.rs` writes the long-form round from a historical directory into a local one, and `examples/check_v0011.rs` compares those exports with the historical observations. Neither changes a published round.
