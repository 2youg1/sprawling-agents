<!-- coderabbit-ai-generated:v1 -->

<!-- coderabbit-ai-generated-section:v1:168466b0eb1d9aa58c75673f28d8da92d845895eed269ff6a83736114223a650 -->

# Cross-agent benchmark measurement method

For future rounds, this file binds the cross-agent benchmark to a design that a reader can audit without rerunning it. It names the series, estimators, sample-size rules, parameters, schemas and commands behind every published statistic.

The [performance guide](../performance.md#evaluate-a-change-without-hiding-its-distribution) governs every published evaluation and wins if the two documents disagree. This file supplies the benchmark-specific choices that the guide leaves open. A round directory owns its observations and exports; it does not redefine this method.

The method, claims, thresholds and seeds are pre-registered before the round's first observation. A later change creates a new analysis version rather than silently editing an already published round.

## What a round may claim

Each manifest claim names its series, primary metric, unit, direction, minimum worthwhile change, multiplicity family and falsification condition. A claim written after seeing the result is a description, not a pre-registered claim.

The report states the highest supported rung on this ladder:

| Rung | Required evidence |
| --- | --- |
| Description | Observations, their distributions and intervals. |
| Contrast | A named contrast on the same measure, with an interval interpreted against the pre-registered no-benefit bounds. |
| Ranking | A sized cell per subject, a contrast that survives multiplicity correction, and no unresolved drift flag. |
| Causal | An unaffected control, balanced interleaving and a credible identifying assumption; change claims use difference-in-differences as the performance guide specifies. |

A single-arm round supports no ranking. A survivor-selected subset remains an engineering lens, not a suite result. One build supports no change claim. A statistically non-significant contrast is not evidence of equivalence.

## Series coverage is an executable contract

Coverage has three separate owners:

1. **Design:** the registry below lists every long-term series and its status.
2. **Round:** the manifest records coverage as `measured`, `deferred` or `failed`.
3. **Report:** the generated report prints every deferred and failed row with its reason and reopening condition; it never omits an unmeasured series silently.

Design status has four values:

| Status | Meaning |
| --- | --- |
| `required` | Every round measures the series or records a bounded deferral or failure. |
| `optional` | The round measures the series when a pre-registered claim depends on it. |
| `retired` | A ruling removed the series from future rounds; the registry retains the ruling and reason. |
| `unsupported` | The instrument cannot observe the quantity in principle; the registry retains the mechanism preventing measurement. |

`retired` and `unsupported` belong only to the design registry and are invalid in a round's coverage. A required series does not become optional because a round could not afford it.

### Design registry

This is the complete design boundary. Historical coverage does not shrink it. `case`, `state` and `workload` split a series into cells without creating new methods or registry entries.

| Series | Status | Reading and claim boundary |
| --- | --- | --- |
| `startup` | `required` | Help exit and new-project first completed answer are separate cases with explicit start and ready endpoints. |
| `first_pairing` | `required` | First successful pairing latency, retaining failed and censored attempts. |
| `cold_boot_gui` | `required` | Cold process start to the declared usable GUI endpoint; distinct from a warm reload. |
| `browser_reload` | `optional` | Warm browser reload to the declared input endpoint, required for a GUI-responsiveness claim. |
| `local_tool` | `required` | First and steady-state read, create, search, edit, execute and recovery cycles are separate cases; a first action per group is not relabelled as a per-session cost. |
| `request_latency` | `required` | Per-request model, tool and queue durations, sufficient to decompose task wall time. |
| `task_wall` | `required` | End-to-end task duration with right censoring retained. |
| `task_requests` | `required` | Model-request count per task. |
| `task_tokens` | `required` | Known input and output token counts, with incomplete usage marked rather than zero-filled. |
| `task_outcomes` | `required` | Functional pass, contract pass and normal completion remain separate outcomes. |
| `session_memory` | `required` | One phase-aligned reading per independent cohort for each declared state and session count. |
| `sustained_memory` | `required` | Ordered readings and a slope or change analysis over a declared sustained window. |
| `real_model_capacity` | `required` | Completion and resource observations at declared real-model concurrency; memory alone is not capacity. |
| `task_io` | `optional` | Process-tree read and write counter deltas, required for an I/O or architecture claim. |
| `monitor_beat` | `optional` | The named source instrument's beat readings, required for a claim about monitor cadence or cost. |
| `view_backlog` | `optional` | The named source instrument's backlog readings, required for a claim about view congestion. |
| `cross_machine_absolute_time` | `unsupported` | Absolute times from different machines are not directly comparable; use a same-machine contrast or a design with a common clock and instrument. |

No series is retired. Adding or retiring a series changes this registry and the validator contract in the same change.

### Round coverage entries

Every coverage entry has `series`, `status`, `reason`, `reopen_when`, `measured_by` and `reading`:

- `measured` names its instrument in `measured_by` and its raw source in `reading`; the other explanatory fields are null.
- `deferred` states the concrete blocker in `reason` and an observable condition in `reopen_when` that restarts measurement.
- `failed` states the failed action and observed failure in `reason`, and the recovery or retry condition in `reopen_when`; retain any partial raw source in `reading`.

The validator rejects a missing `required` series, an omitted `optional` series referenced by a claim, an unknown series, a design-only status in round coverage, a measured entry without an instrument or source, and a deferred or failed entry without its required explanation. Cell-level missingness remains visible even when the series as a whole is measured. A claim whose required reading is deferred or failed remains unearned and is printed as a deferred claim, not as a conclusion supported by the remaining cells.

## Independent units and run order

The independent unit is the round: one complete interleaved sweep of every subject on one machine class and one build. Observations inside a round share machine conditions, provider state and thermal history, so they are not independent draws.

- Interleave subjects within each round. Derive the starting subject and any randomized order from the round's pre-registered seed, and record the realized order per observation.
- Keep all candidates from a round together in a paired bootstrap draw. Pooling rounds requires the same build, arm, machine class, counter source and compatible conditions; it is a pre-registered decision, not a convenience.
- Cold and warm cells are separate cases. Never pool them, and publish no ratio between them without both intervals.
- Record per-request model, tool and queue latency so that wall time can be decomposed rather than attributed to local tool speed by subtraction.

## Size every cell before the confirmatory stage

Convention does not size a cell. Record the sizing arithmetic and target before stage two.

### Quantitative cells

Stage one is a pilot of 100 observations in at least four cohorts. From it, the analyzer exports the block-bootstrap BCa half-width `h` of the cell's headline quantile. With pilot count `n0`, observations per cohort `m`, and pre-registered target `h_target`, the stage-two target is

```text
n = ceil(n0 * (h / h_target)^2 / m) * m
```

so that the target is a whole number of cohorts, each carrying the pilot's observation count.

The target half-width is 10% of the pilot point estimate for P50 and 25% for P95 or P99. Before the first observation, register the headline quantile, its relative target, an absolute target for non-positive estimates, and the pilot-to-target calculation. A P95 headline uses its own pilot P95 interval and estimate to compute `n95`; a P99 headline registers both `n99` and fallback `n95` calculations. Freeze the resulting numerical width targets after the pilot, before stage two. The square-root scaling is a sizing rule derived from the pilot, not a claim that observations within a round are independent. A zero or undefined pilot half-width does not justify a zero sample target: add one fresh pilot-sized cohort and recompute, stopping with a sizing failure if the pre-registered resource ceiling is reached before a valid non-zero width is observed. Retain the failed BCa analysis while that fresh evidence is missing; never relabel a substitute interval as BCa. A non-positive point estimate needs a pre-registered absolute width target instead of a relative one. Stage two adds fresh cohorts until the cell reaches `n` (for a P99 headline, `max(200, n99, n95)`); the final estimate and interval are recomputed from all eligible stages, and the achieved width is printed beside the target. A pilot that poorly represents the workload can therefore leave a visible miss rather than a hidden pass.

A published P99 requires `n >= 200` and an interval half-width no greater than 25% of its point estimate. These are publication policies, not tests. If either fails, the export sets `p99_publishable: false` with the failed condition, and the headline reports P95 with its interval and the maximum instead. The fallback retains the pre-registered `n95` and frozen P95 width target; it neither substitutes `n99` for `n95` nor resizes after seeing the confirmatory result. A missed P95 count or width target is printed and permits description only, with no quantile ranking. The analyzer still retains the P99 estimate and interval in the exports for diagnosis; it cannot be used for a ranking.

### Outcome cells

Pre-register the eligible-attempt estimand, assumed proportions, two-sided family alpha, power target, family size `K`, allocation ratio and rejection rule. Use the common alpha and power values below. For genuinely independent, unpaired Bernoulli trials only, enumerate both arms' possible counts under the score-test rejection rule at conservative single-comparison alpha `alpha / K`. Choose the smallest per-arm count meeting the power target, or the smallest total count along the registered unequal-allocation ratio. This guarantees the target under Holm's stricter first rejection threshold; unadjusted-alpha power is insufficient. Report family alpha, allocated alpha, assumed proportions, allocation, achieved power and counts per arm. Approximations are cross-checks only.

Repeated attempts within a round and candidates observed in the same round require a joint method: pre-register a round-level dependence model (including within-round correlation and cross-candidate pairing), cluster sizes, an interval estimator and test that preserve whole paired rounds, and power simulation of that same rejection rule at `alpha / K`. Size independent rounds along the registered allocation, report attempts per round as well as round counts, and demonstrate the power target and null error control under the registered model and sensitivity scenarios. Independent-binomial enumeration cannot size this design. Until these methods and their calibration checks are specified and implemented in the analyzer, pass-rate ranking is unavailable. An unaffordable target also precludes ranking. Descriptive counts and rates remain reportable; independent-binomial intervals cannot serve as confirmatory intervals for correlated attempts.

### Continuous primaries

Model requests, task wall time and known token counts are the primary task metrics. Functional, contract and normal-completion outcomes are gates and separately reported quantities, not a substitute for a powered continuous comparison. A wall-time sum excludes right-censored cells and states that exclusion; a survival-style summary is required to include their information.

## One estimator per exported statistic

Every reported number appears in `exports.json`, with its estimator, parameters and seed. Two kinds of number remain distinct in the report: a **test** has a null distribution and calibrated error rate, while a **publication policy** is a pre-registered editorial rule. The P99 width gate, sizing targets and split-half warning are policies; dip, Pettitt, score tests and Holm adjustment are statistical procedures.

All primary confidence intervals are two-sided 95%. Confirmatory tests use two-sided alpha 0.05, all families use Holm at that level, and sizing targets power 0.8. Every bootstrap, Monte Carlo calibration and permutation analysis uses 10,000 replicates; the tables below name the resampling structure rather than redefining those values.

| Export | Estimator and parameters | What it supports |
| --- | --- | --- |
| `n`, `min`, `max`, `p50`, `p95`, `p99` | Hyndman–Fan type 1, the inverse empirical CDF: for `q > 0`, the one-based order statistic `ceil(q*n)` | Point estimates with their sample count. |
| `p50_ci`, `p95_ci`, `p99_ci` | BCa interval from dependence-preserving bootstrap replicates; percentile interval retained as sensitivity | Uncertainty of the named quantile. |
| `cv`, `max_over_p99`, `over_2x_p99`, `over_5x_p99` | Arithmetic on observed values | Descriptive tail mass and profiling targets, not a stability test. |
| `dip`, `dip_p` | Hartigan's dip statistic using AS 217; `dip_p` requires the dependence calibration below, followed by Holm within the registered modality family | Rejection of unimodality, not proof of a particular execution path. |
| `modes`, `antimodes`, `shares` and their intervals | Gaussian KDE with Sheather–Jones bandwidth; grid, support, matching and interval rules below | Shape summaries for a multimodal cell. |
| `pettitt`, `pettitt_p` | Pettitt single-change-point statistic with unit-preserving permutations; Holm adjustment within the drift family | A calibrated drift reading and estimated change point. |
| `split_half_ratio`, `cusum`, `cohort_range_over_median` | Half-medians, normalized cumulative sums and per-cohort quantiles | Descriptive effect sizes and plots; the ratio warning named below is a policy. |
| `slope`, `intercept`, `r_squared`, `slope_ci` | Ordinary least squares on one point per independent cohort; pairs bootstrap over cohorts | Memory scaling with uncertainty and fit quality. |
| `wilson_ci` | Wilson score interval for independent Bernoulli trials only | Unavailable as a confirmatory interval for repeated or paired attempts. |
| `risk_difference`, `risk_difference_ci` | Difference of proportions; Newcombe hybrid score interval only for independent, unpaired trials; otherwise the registered round-aware interval | A two-arm rate contrast; never interval overlap as a substitute test. |
| `required_n` | The quantitative pilot rule or applicable outcome sizing rule above | The cell's sample-size rationale. |

### Resampling, order and deterministic seeds

For a paired contrast, resample whole interleaved rounds. For an ordered single-cell series, use a moving-block bootstrap over the declared collection order, without crossing cohort or round boundaries. Select moving-block length separately for each series with the Politis–White automatic procedure and export the selected length. When several rounds form the ordered series, apply the procedure to whole rounds and export the per-round statistic used to select the block length. Circular or stationary bootstrap results are declared sensitivity analyses, not unrecorded variations.

BCa bias correction comes from the bootstrap distribution; acceleration comes from a jackknife that deletes the same independent units or blocks used by the bootstrap. Every replicate preserves the original number of units and all within-round pairing. Record the number of usable replicates. Numerical failure, too few distinct units for the jackknife or an unstable endpoint produces a failed analysis row; a percentile fallback is labelled and disclosed rather than substituted silently.

An empty `index` declares that collection order was not preserved. Such a cell may receive an iid bootstrap sensitivity interval, clearly labelled, but not a confirmatory dependent-data interval.

Derive each statistic's random stream as SHA-256 over the length-prefixed UTF-8 fields `(seed_string, round, cell key, statistic)`. The stream therefore depends on the cell's identity rather than hash-map iteration or CSV row order. The manifest fixes `seed_string`. Dip calibration keys also include the registered null model, cluster layout and dependence parameters; a cache keyed only by sample size is invalid. Every calibrated export records the draw count and calibration revision.

All families use Holm's procedure at the level above. The manifest defines family membership before measurement. Reports include raw and adjusted p-values. A stricter pre-registered simultaneous procedure may replace the common interval confidence level for its family.

### Modality calibration and shape definitions

`dip` remains a descriptive statistic for correlated observations. A confirmatory `dip_p` requires a pre-registered unimodal null model that generates whole rounds with the observed cluster sizes, within-round dependence and candidate pairing. Record the generator, nuisance-parameter treatment, statistic, rejection rule and simulation seed. Before using it, validate type-I error over the registered null/dependence parameter range, including boundary cases, with Monte Carlo uncertainty reported. Resampling the empirical multimodal distribution is not null calibration, and iid uniform draws indexed by sample size do not control errors for this design. No validated dependent-null calibration is specified or implemented yet: export `dip_p: null` with status `unavailable`, and make no inferential multimodality claim until one is supplied. Holm cannot repair an uncalibrated p-value.

For reproducible descriptive shape summaries, evaluate the Gaussian KDE on 4097 equally spaced points from `min - 4*bw` to `max + 4*bw`, using the positive Sheather–Jones solve-the-equation bandwidth `bw`. Export the bandwidth, grid and selector variant from the pinned analyzer; a failed selector makes the shape analysis unavailable. Use this same grid and bandwidth for every dependence-preserving bootstrap replicate. A local maximum is an interior grid point higher than both neighbours; collapse an exactly equal plateau to its leftmost point when both outside neighbours are lower. Between adjacent maxima, the lowest grid value is the antimode (leftmost on ties). The two exterior basins extend to infinity. `shares` are empirical fractions in these basins, with an observation exactly on a cut assigned to the basin on its left; shares sum to one.

Match replicate maxima to the original maxima by original basin membership. When several replicate maxima enter a basin, keep the one nearest its original maximum, breaking ties to the left; other maxima are unmatched and their count is exported. An empty basin has an unmatched original mode. A supported mode has a match in at least 95% of replicates; this threshold is a descriptive policy. Export support frequencies for all original maxima. Remove unsupported maxima, recompute antimodes between adjacent retained maxima, and compute the final shares. `modes` contains retained maxima in increasing coordinate order, `antimodes` their intervening cuts, and `shares` their basin fractions; no retained maxima means an unavailable shape summary.

For intervals, match replicates again to the retained modes using their final basins and the same tie rules. Compute replicate cuts between adjacent matched modes and replicate empirical shares using those cuts. If any retained mode is unmatched, mark that replicate's shape vector missing; do not set its share to zero, silently renormalize or relabel remaining modes. Export the missing fraction and unmatched counts. Centre, cut and share intervals use BCa only when every bootstrap and delete-unit jackknife shape vector is complete and the BCa checks pass; otherwise export null intervals with an explicit topology-change or numerical-failure reason. Do not present intervals conditional on a convenient subset of mode counts as unconditional uncertainty. While no calibrated dip p-value flags a cell as multimodal, its shape remains an unflagged descriptive summary: export support but mark centre, cut and share intervals not applicable. The interval rule applies as soon as such a cell is flagged or a pre-registered descriptive shape interval is requested.

## Distribution first, then drift

Every latency cell publishes its ECDF and quantiles with intervals. A memory cell publishes its one point per cohort, the fit over them, and a distribution only when at least eight cohorts make one readable. A density chart uses the Gaussian kernel and Sheather–Jones bandwidth named above. Keep cold observations and outliers visible: publish the maximum and the tail counts beyond two and five times P99.

A cell with an available, calibrated Holm-adjusted dip p-value that rejects unimodality at that level is flagged multimodal. Its report prints mode centres, antimode cut points and component shares beside the quantiles, with intervals or the explicit interval-unavailability reasons required above. Do not cut at the midpoint of the dip modal interval and do not use forced two-cluster k-means as modality evidence. A mode tree or SiZer may support exploration, but an exploratory grouping does not replace the pre-registered analysis.

Multimodality does not identify a second execution path. That claim needs tracing, profiling or equivalent execution evidence named beside it. If KDE bandwidth or mode estimation fails, the report carries a failed analysis row and the raw observations rather than inventing one cluster.

Pettitt supplies the primary per-group drift test. Report its estimated change point, raw and adjusted p-values, and the before/after observations. A series with fewer than four ordered observations has no admissible change point and returns a failed analysis row instead of a p-value. CUSUM remains a descriptive plot, and the split-half ratio remains an effect size with a 1.25 policy warning. Report per-cohort quantile ranges even when quantiles are estimated from pooled cohorts. A significant change point or an unresolved split-half warning requires a rerun or a published flag with no unqualified ranking claim.

For sustained memory, report ordered readings and the pre-registered slope or change analysis with its interval. Two similar endpoints and a peak-memory reading do not establish sustained stability.

## Outcomes, failures, censoring and missingness

Every outcome row has one failure class:

| Class | Meaning and denominator rule |
| --- | --- |
| `none` | Normal completion with no failure. |
| `infrastructure` | Transport or provider failure; exclude from the capability pass-rate denominator and report separately as provider reliability. |
| `capability` | The eligible attempt did not satisfy the functional or contract check. |
| `guard` | A declared request, resource or execution guard stopped the attempt. |
| `timeout` | The declared deadline stopped the attempt. |

For `functional` and `contract`, the denominator includes every selected, attempted, non-infrastructure outcome with a known check result, including guard and timeout attempts. Score each check independently: `1` only if its declared check passes, otherwise `0`; stopping does not erase an already verified result. A check not run or with unknown evidence is missing, with its count and reason reported separately. For `normal_completion`, all selected, attempted, non-infrastructure outcomes with known termination status are eligible: normal exit is `1`, and guard, timeout or abnormal exit is `0`, regardless of check results. Failure class describes termination or failure cause; it does not substitute for the three metric-specific results.

For example, one normally completed attempt passing both checks plus nine timeouts failing both checks gives `functional = 1/10`, `contract = 1/10` and `normal_completion = 1/10`. If all ten fail a check, that check's rate is `0/10`; if the nine timed-out checks were never evaluated, its observed rate is `1/1` with nine missing results, and is not a suite pass rate or ranking basis. Report numerator, eligible denominator and missing count for each metric.

Guard and timeout outcomes remain visible separately because their recovery differs. An infrastructure exclusion is not a success. Missing and unknown outcomes remain missing; they are not encoded as either success or failure.

A deadline hit is right-censored: its value is a lower bound, `censored = 1`, and the report writes `>= T`. It is not a completed duration. Exclude it from ordinary wall-time sums and state the exclusion, or use a pre-registered censored-data estimator that actually uses the bound.

Report successes and eligible trials for each proportion and the risk difference for each contrast. Wilson intervals, Newcombe intervals and the score test apply only to independent, unpaired trials. Repeated or paired attempts use the matched sizing, interval and testing methods required under Outcome cells; absent those methods, report descriptive rates with confirmatory intervals and tests unavailable. A ranking requires the difference interval to exclude the pre-registered no-benefit region. An interval covering those bounds does not support ranking and does not prove equality.

Practical equivalence uses two one-sided tests against pre-registered no-benefit bounds. The bounds are policy choices justified before measurement. Excluding zero alone establishes neither practical benefit nor equivalence.

Record missingness per cell as design-excluded, not attempted, or outcome-unknown, with a concrete reason. Report attempted and planned denominators and the case mix. A pass rate over attempted cells is not the suite rate. Do not attach an `MNAR` label or assert a bias direction without a missingness model and supporting evidence. Do not impute outcomes unless the imputation model was pre-registered.

## Memory uses independent cohorts

Memory contributes one phase-aligned point per cohort, not hundreds of autocorrelated window samples presented as independent observations.

- Use at least eight cohorts per `(subject, state, session count)` cell. Take each point after the declared settling condition and retain the underlying time series for audit; extra samples within the run do not increase the independent cohort count.
- Measure idle and active in separate runs. Their process lifecycles differ, so a side-by-side value is not a before/after pair and their subtraction is not a generation cost.
- Fix in-flight queue depth across subjects, or record it per cohort and include it as a pre-registered factor. Subjects with different in-flight counts at the same submitted-session level do not share an unlabelled comparison axis.
- A capacity claim reports slope, intercept, `r_squared` and the slope interval rather than per-level point tables alone. The counter name and adapter travel with every value.
- The analyzer refuses a slope from fewer than four cohorts or fewer than three session levels. The design target remains at least eight cohorts; meeting only the refusal floor is not the planned confirmatory design.
- Pre-register a positive near-zero slope threshold `s0` and positive absolute half-width target `h_abs`, both in the slope's units, before the pilot. Use fresh cohorts and recorded pilot arithmetic to size the slope interval: when `abs(slope) <= s0`, require half-width at most `h_abs`; otherwise require at most 25% of `abs(slope)`. Export the selected target and achieved width. Zero or negative slopes remain valid fits when estimable; report their sign and interval without calling sign an analysis failure. A precision miss is a reported target miss. Numerical instability, non-finite estimates, insufficient cohorts or levels, and failed bootstrap/jackknife checks remain analysis failures.

A high `r_squared` describes linear fit; it does not confirm a process model. Maximum feasible concurrency needs direct completion and resource evidence, not extrapolation from a memory slope alone.

## Round artifacts and schemas

```text
docs/benchmarks/<round>/
  README.md        the authoritative Markdown report: prose, coverage and generated number spans
  manifest.json    claims, builds, environment, workload and fixture digests, coverage, method version and seed
  results.csv      raw observations and explicitly identified cell-level counts or flags
  exports.json     every statistic consumed by the report
```

`results.csv` uses LF line endings and this header:

```text
schema_version,observation_id,task_attempt_id,request_id,span_start,span_end,round,build,arm,agent,series,workload,case,cohort,state,sessions,metric,unit,index,value,failure_class,missing_reason,censored,usage_complete,selected,in_flight,counter_source,condition
```

- `observation_id` identifies one metric row, stable and unique within the published round directory; `series` comes from the design registry. It is not a task or request identifier.
- `task_attempt_id` uniquely identifies an execution attempt within that directory across all cells and series; retries and retained alternatives get distinct IDs. Every task outcome, wall, request count, token and request-latency row carries it. Rows sharing it must agree on subject, build, arm, workload, case and interleaved round. Non-task readings leave it empty.
- `request_id` identifies one model or tool invocation within its task attempt; the pair `(task_attempt_id, request_id)` joins all metrics for that invocation. Concurrent requests have distinct IDs; a request retry is a new invocation. Task aggregates leave `request_id` empty. Queue, model and tool spans carry their owning invocation's pair, so interleaving in a cell is unambiguous.
- `span_start` and `span_end` record monotonic seconds relative to the owning attempt's start for each task wall and request latency span. Export zero as the task start and the measured or censored end as its wall endpoint. Spans must lie within that window; retain cancellation activity after the wall endpoint as separately labelled diagnostic spans excluded from the decomposition. Validate duration against endpoint differences and count distinct model invocation IDs against `requests`. Decompose wall time using the union of request spans and its uncovered residual; report overlaps separately, since concurrent span durations cannot simply be added. Missing spans make the decomposition unavailable rather than implying zero overhead.
- `metric` names the raw quantity, such as `latency`, `memory_private`, `wall`, `requests`, `input_tokens`, `output_tokens`, `read_bytes`, `write_bytes`, `functional`, `contract`, `normal_completion`, `model_latency`, `tool_latency` or `queue_latency`.
- `value` is in `unit`; a censored value is a lower bound. Rate rows use `0` or `1`, with eligibility determined by the failure and missingness fields rather than by dropping inconvenient rows.
- `index` is position in collection order. `state` and `sessions` apply to memory rows; `in_flight` records actual queue depth and `counter_source` names the instrument and adapter.
- `usage_complete` distinguishes known token totals from unknown usage; `selected` distinguishes a pre-registered primary attempt from a retained alternative. `condition` records configuration differences such as a compatibility gateway or non-native tool route.

The manifest's coverage entries use the exact fields and status rules above. It also records an immutable `analysis_revision` (full Git commit SHA containing this method and the analyzer), input hashes, toolchain, machine class, estimator version, bootstrap unit, resampling count, block lengths, seed and balanced-order schedule. Unknown units, duplicate observation identifiers, inconsistent task ownership, orphan request spans, invalid span endpoints and non-finite estimates are validation errors.

`exports.json` has sorted keys, integer counts, shortest-round-trip floating-point formatting, LF line endings and no timestamp. It does not round small non-zero slopes or interval endpoints to zero. Identical observations, manifest and estimator version therefore produce identical bytes; a diff identifies changed input or analysis rather than generation time.

`README.md` is the authoritative Markdown report and holds the round's prose, claim ladder, coverage table and limits; its numbers live in generated spans that `benchstat export` rewrites and `benchstat verify` re-checks, so a hand-edited number fails verification instead of surviving as prose. Every number in the generated spans comes from `exports.json`, and deferred and failed coverage rows appear there. An agent may derive HTML from the report and exports, but HTML is not a publication or verification artifact.

## Reproduce a published round

The round's analyzer is the Rust workspace package `sprawling-benchstat` under `crates/benchstat/`, holding validation, estimators, verification and report generation. Its library and executable are both named `benchstat`. It depends on no product crate, so an analysis cannot change what it measures, and its estimator core has no I/O beyond the round directory. Its generality, audit importance and size justify a package separate from `xtask`; crates.io distribution lets an outside analyzer use the same implementation without adding it to the product graph.

To reproduce a future round, read `analysis_revision` from its manifest, run `git worktree add --detach <analysis-checkout> <analysis_revision>`, and build there with `cargo build --locked -p sprawling-benchstat`, using that revision's recorded toolchain. Run its `target/debug/benchstat` with the commands below against an absolute path to a copy of the published round. The analysis revision must be an ancestor available in repository history; record round input digests separately so the analyzer commit need not contain its own SHA. Never use the latest analyzer as a substitute.

Build the package with `cargo build --locked -p sprawling-benchstat`; its executable exposes:

```sh
benchstat export docs/benchmarks/<round> --write
benchstat verify docs/benchmarks/<round>
benchstat report docs/benchmarks/<round> --write
```

`export` validates the manifest, registry coverage and observation schema, computes every exported statistic, then writes `exports.json` and the README's generated spans. `verify` recomputes every export from the published subset in a fresh invocation and compares `exports.json` and the generated spans byte for byte. `report` re-renders only exported values into the README. A round is publishable only when all three succeed on its committed inputs, and its README names the analysis version and the commands that produced it.

Historical artifacts, including the [v0.0.11 observations](v0.0.11/README.md), remain unchanged. Their historical verifier is not rewritten or presented as the next round's analyzer.

## Design decisions

**Rust with one executable verifier, not parallel Python estimators.** Repository tooling is Rust, and one implementation avoids disagreement between generator and verifier. The dip implementation follows the published AS 217 algorithm and is checked against the reference package's public vectors. GPL-licensed `diptest` and `multimode` code is not vendored into this MPL-2.0 tree. The acceptance requirement is complete recomputation from the published subset, not a particular verifier filename.

**BCa and automatic block lengths, not percentile intervals with a fixed cube-root rule.** The interval must respond to observed bias and dependence, and the selected block length must be visible enough to reproduce and challenge.

**KDE modes and antimodes, not forced two-cluster k-means or a dip-interval midpoint.** Forced k=2 returns two centres even for unimodal data and moves with outliers; a midpoint is not a named cut-point estimator. Dip supplies the modality diagnostic, while KDE supplies the shape summary.

**Pettitt and descriptive effect sizes, not a threshold presented as a change-point test.** The permutation test supplies calibration; split-half and CUSUM displays show the magnitude and shape of the change.

**Outcome methods follow the independent unit.** Independent, unpaired trials permit Newcombe contrasts and exact binomial sizing at the allocated alpha. Repeated and paired attempts require joint round-aware sizing and inference; rankings wait for validated implementation. Interval overlap and independent-trial formulas cannot establish a correlated contrast.

**One authoritative Markdown report beside canonical JSON, not a second required view.** The JSON preserves full numeric precision and the Markdown remains reviewable as text. HTML may be derived later without becoming another input to verification.

**One row per observation with explicit order and conditions.** The schema preserves the independent unit, censoring, failure class, counter source and configuration needed to reproduce or reject an analysis. An array packed into a CSV cell would hide the order and make row-level validation less direct.

## Method references

A reference names the source of a procedure, not decorative support for a conclusion. Changing a procedure requires changing its binding above and its implementation conformance checks together.

1. J. A. Hartigan and P. M. Hartigan, “The Dip Test of Unimodality”, *Annals of Statistics* 13(1), 1985, 70–84.
2. P. M. Hartigan, “Computation of the Dip Statistic to Test for Unimodality”, *Applied Statistics* 34(3), 1985, 320–325, algorithm AS 217.
3. R. J. Hyndman and Y. Fan, “Sample Quantiles in Statistical Packages”, *The American Statistician* 50(4), 1996, 361–365.
4. B. Efron and R. J. Tibshirani, *An Introduction to the Bootstrap*, Chapman & Hall, 1993.
5. B. Efron, “Better Bootstrap Confidence Intervals”, *Journal of the American Statistical Association* 82(397), 1987, 171–185.
6. H. R. Künsch, “The Jackknife and the Bootstrap for General Stationary Observations”, *Annals of Statistics* 17(3), 1989, 1217–1241.
7. D. N. Politis and H. White, “Automatic Block-Length Selection for the Dependent Bootstrap”, *Econometric Reviews* 23(1), 2004, 53–70.
8. S. J. Sheather and M. C. Jones, “A Reliable Data-Based Bandwidth Selection Method for Kernel Density Estimation”, *Journal of the Royal Statistical Society: Series B* 53(3), 1991, 683–690.
9. S. Holm, “A Simple Sequentially Rejective Multiple Test Procedure”, *Scandinavian Journal of Statistics* 6(2), 1979, 65–70.
10. A. N. Pettitt, “A Non-Parametric Approach to the Change-Point Problem”, *Applied Statistics* 28(2), 1979, 126–135.
11. E. B. Wilson, “Probable Inference, the Law of Succession, and Statistical Inference”, *Journal of the American Statistical Association* 22(158), 1927, 209–212.
12. R. G. Newcombe, “Interval Estimation for the Difference Between Independent Proportions: Comparison of Eleven Methods”, *Statistics in Medicine* 17(8), 1998, 873–890.
13. M. W. Fagerland, S. Lydersen and P. Laake, “Recommended Confidence Intervals for Two Independent Binomial Proportions”, *Statistical Methods in Medical Research* 24(2), 2015, 224–254.
14. D. J. Schuirmann, “A Comparison of the Two One-Sided Tests Procedure and the Power Approach for Assessing the Equivalence of Average Bioavailability”, *Journal of Pharmacokinetics and Biopharmaceutics* 15, 1987, 657–680.
15. D. B. Rubin, “Inference and Missing Data”, *Biometrika* 63(3), 1976, 581–592.
16. B. A. Nosek, C. R. Ebersole, A. C. DeHaven and D. T. Mellor, “The Preregistration Revolution”, *PNAS* 115(11), 2018, 2600–2606.
17. J. P. Simmons, L. D. Nelson and U. Simonsohn, “False-Positive Psychology: Undisclosed Flexibility in Data Collection and Analysis Allows Presenting Anything as Significant”, *Psychological Science* 22(11), 2011, 1359–1366.

## Interpretation limits

Configuration confounds remain: shell-backed versus native tools, different provider protocols and a compatibility gateway measure the disclosed configuration rather than an abstract product. Provider and model drift are bounded by interleaving, not removed. A different machine class or model version is a different round.

A round measures only its named fixtures and actual conditions. Coverage rows state what a particular round deferred or failed to measure and when that measurement reopens; historical omissions never become permission to omit a required series later.

Sizing depends on a representative pilot, so the report always exposes achieved interval width beside its target. Statistical uncertainty cannot repair incompatible endpoints, queue regimes, counter adapters or workload definitions. No generated ranking, equivalence statement, capacity claim or execution-path explanation may be stronger than its observations, interval, test and coverage record support.


<!-- /coderabbit-ai-generated:v1 -->