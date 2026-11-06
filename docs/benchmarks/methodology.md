# Cross-agent benchmark measurement method

This file binds the cross-agent benchmark to a design that a reader can audit without rerunning it. It names the series, estimators, sample-size rules, parameters, schemas and commands behind every published statistic.

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

The target half-width is 10% of the point estimate for P50 and 25% for P99. The square-root scaling is a sizing rule derived from the pilot, not a claim that observations within a round are independent. A zero pilot half-width does not justify a zero sample target: add one fresh pilot-sized cohort and recompute, stopping with a sizing failure if the pre-registered resource ceiling is reached before a non-zero width is observed. A non-positive point estimate needs a pre-registered absolute width target instead of a relative one. Stage two adds fresh cohorts until the cell reaches `n`; the final estimate and interval are recomputed from all eligible stages, and the achieved width is printed beside the target. A pilot that poorly represents the workload can therefore leave a visible miss rather than a hidden pass.

A published P99 requires `n >= 200` and an interval half-width no greater than 25% of its point estimate. These are publication policies, not tests. If either fails, the export sets `p99_publishable: false` with the failed condition, and the headline reports P95 with its interval and the maximum instead. The analyzer still retains the P99 estimate and interval in the exports for diagnosis; it cannot be used for a ranking.

### Outcome cells

Size a two-arm pass-rate comparison by exact binomial power at the pre-registered two-sided alpha and power target. Enumerate the possible outcome counts under the planned allocation and the pre-registered score-test rejection rule; choose the smallest per-arm count whose exact power reaches the target, or the smallest total count along a pre-registered unequal-allocation ratio. Report the assumed proportions, allocation, alpha, power and resulting count per arm. Score-based normal and arcsine approximations are cross-checks only, especially near proportions zero or one.

A round that cannot afford the exact-power target still reports each proportion's Wilson interval and each contrast's Newcombe interval, but supports no pass-rate ranking.

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
| `dip`, `dip_p` | Hartigan's dip statistic using AS 217; uniform-null Monte Carlo draws per sample size; Holm adjustment within the pre-registered modality family | Rejection of unimodality, not proof of a particular execution path. |
| `modes`, `antimodes`, `shares` and their intervals | Gaussian KDE with Sheather–Jones bandwidth; supported local maxima are centres and intervening local minima are cut points; the same bootstrap supplies intervals | Shape summaries for a multimodal cell. |
| `pettitt`, `pettitt_p` | Pettitt single-change-point statistic with unit-preserving permutations; Holm adjustment within the drift family | A calibrated drift reading and estimated change point. |
| `split_half_ratio`, `cusum`, `cohort_range_over_median` | Half-medians, normalized cumulative sums and per-cohort quantiles | Descriptive effect sizes and plots; the ratio warning named below is a policy. |
| `slope`, `intercept`, `r_squared`, `slope_ci` | Ordinary least squares on one point per independent cohort; pairs bootstrap over cohorts | Memory scaling with uncertainty and fit quality. |
| `wilson_ci` | Wilson score interval | Uncertainty of one eligible pass count. |
| `risk_difference`, `risk_difference_ci` | Difference of proportions with the Newcombe hybrid score interval | A two-arm rate contrast; never interval overlap as a substitute test. |
| `required_n` | The quantitative pilot rule or exact binomial power rule above | The cell's sample-size rationale. |

### Resampling, order and deterministic seeds

For a paired contrast, resample whole interleaved rounds. For an ordered single-cell series, use a moving-block bootstrap over the declared collection order, without crossing cohort or round boundaries. Select moving-block length separately for each series with the Politis–White automatic procedure and export the selected length. When several rounds form the ordered series, apply the procedure to whole rounds and export the per-round statistic used to select the block length. Circular or stationary bootstrap results are declared sensitivity analyses, not unrecorded variations.

BCa bias correction comes from the bootstrap distribution; acceleration comes from a jackknife that deletes the same independent units or blocks used by the bootstrap. Every replicate preserves the original number of units and all within-round pairing. Record the number of usable replicates. Numerical failure, too few distinct units for the jackknife or an unstable endpoint produces a failed analysis row; a percentile fallback is labelled and disclosed rather than substituted silently.

An empty `index` declares that collection order was not preserved. Such a cell may receive an iid bootstrap sensitivity interval, clearly labelled, but not a confirmatory dependent-data interval.

Derive each statistic's random stream as SHA-256 over the length-prefixed UTF-8 fields `(seed_string, round, cell key, statistic)`. The stream therefore depends on the cell's identity rather than hash-map iteration or CSV row order. The manifest fixes `seed_string`. The dip null depends only on sample size, so it is generated once per `n` using `dip-null` as the statistic key and the sample size in the cell-key field; every export records the draw count.

All families use Holm's procedure at the level above. The manifest defines family membership before measurement. Reports include raw and adjusted p-values. A stricter pre-registered simultaneous procedure may replace the common interval confidence level for its family.

## Distribution first, then drift

Every latency cell publishes its ECDF and quantiles with intervals. A memory cell publishes its one point per cohort, the fit over them, and a distribution only when at least eight cohorts make one readable. A density chart uses the Gaussian kernel and Sheather–Jones bandwidth named above. Keep cold observations and outliers visible: publish the maximum and the tail counts beyond two and five times P99.

A cell whose Holm-adjusted dip p-value rejects unimodality at that level is flagged multimodal. Its report prints mode centres, antimode cut points and component shares with intervals beside the quantiles, never P50 or P99 alone. Do not cut at the midpoint of the dip modal interval and do not use forced two-cluster k-means as modality evidence. A mode tree or SiZer may support exploration, but an exploratory grouping does not replace the pre-registered analysis.

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

Guard and timeout outcomes remain visible separately because their recovery differs. An infrastructure exclusion is not a success. Missing and unknown outcomes remain missing; they are not encoded as either success or failure.

A deadline hit is right-censored: its value is a lower bound, `censored = 1`, and the report writes `>= T`. It is not a completed duration. Exclude it from ordinary wall-time sums and state the exclusion, or use a pre-registered censored-data estimator that actually uses the bound.

For one proportion, report successes, eligible trials and the Wilson interval. For two proportions, report the risk difference and Newcombe interval; when a hypothesis test is needed, use the corresponding score test in the declared multiplicity family. A ranking requires the difference interval to exclude the pre-registered no-benefit region. An interval covering those bounds does not support ranking and does not prove equality.

Practical equivalence uses two one-sided tests against pre-registered no-benefit bounds. The bounds are policy choices justified before measurement. Excluding zero alone establishes neither practical benefit nor equivalence.

Record missingness per cell as design-excluded, not attempted, or outcome-unknown, with a concrete reason. Report attempted and planned denominators and the case mix. A pass rate over attempted cells is not the suite rate. Do not attach an `MNAR` label or assert a bias direction without a missingness model and supporting evidence. Do not impute outcomes unless the imputation model was pre-registered.

## Memory uses independent cohorts

Memory contributes one phase-aligned point per cohort, not hundreds of autocorrelated window samples presented as independent observations.

- Use at least eight cohorts per `(subject, state, session count)` cell. Take each point after the declared settling condition and retain the underlying time series for audit; extra samples within the run do not increase the independent cohort count.
- Measure idle and active in separate runs. Their process lifecycles differ, so a side-by-side value is not a before/after pair and their subtraction is not a generation cost.
- Fix in-flight queue depth across subjects, or record it per cohort and include it as a pre-registered factor. Subjects with different in-flight counts at the same submitted-session level do not share an unlabelled comparison axis.
- A capacity claim reports slope, intercept, `r_squared` and the slope interval rather than per-level point tables alone. The counter name and adapter travel with every value.
- The analyzer refuses a slope from fewer than four cohorts or fewer than three session levels. The design target remains at least eight cohorts; meeting only the refusal floor is not the planned confirmatory design.
- Size the memory-slope interval to a half-width no greater than 25% of the slope estimate using fresh cohorts and the recorded pilot arithmetic. Report non-positive or unstable slopes as analysis failures rather than hiding the width ratio.

A high `r_squared` describes linear fit; it does not confirm a process model. Maximum feasible concurrency needs direct completion and resource evidence, not extrapolation from a memory slope alone.

## Round artifacts and schemas

```text
docs/benchmarks/<round>/
  README.md        the authoritative Markdown report: prose, coverage and generated number spans
  manifest.json    claims, builds, environment, workload and fixture digests, coverage, method version and seed
  results.csv      raw observations and explicitly identified cell-level counts or flags
  exports.json     every statistic consumed by either report
  report.html      the derived self-contained chart report
```

`results.csv` uses LF line endings and this header:

```text
schema_version,observation_id,round,build,arm,agent,series,workload,case,cohort,state,sessions,metric,unit,index,value,failure_class,missing_reason,censored,usage_complete,selected,in_flight,counter_source,condition
```

- `observation_id` is stable and unique within the round; `series` comes from the design registry.
- `metric` names the raw quantity, such as `latency`, `memory_private`, `wall`, `requests`, `input_tokens`, `output_tokens`, `read_bytes`, `write_bytes`, `functional`, `contract`, `normal_completion`, `model_latency`, `tool_latency` or `queue_latency`.
- `value` is in `unit`; a censored value is a lower bound. Rate rows use `0` or `1`, with eligibility determined by the failure and missingness fields rather than by dropping inconvenient rows.
- `index` is position in collection order. `state` and `sessions` apply to memory rows; `in_flight` records actual queue depth and `counter_source` names the instrument and adapter.
- `usage_complete` distinguishes known token totals from unknown usage; `selected` distinguishes a pre-registered primary attempt from a retained alternative. `condition` records configuration differences such as a compatibility gateway or non-native tool route.

The manifest's coverage entries use the exact fields and status rules above. It also records input hashes, toolchain, machine class, estimator version, bootstrap unit, resampling count, block lengths, seed and balanced-order schedule. Unknown units, duplicate observation identifiers and non-finite estimates are validation errors.

`exports.json` has sorted keys, six-decimal numeric formatting, LF line endings and no timestamp. Identical observations, manifest and estimator version therefore produce identical bytes; a diff identifies changed input or analysis rather than generation time.

`README.md` is the authoritative Markdown report and holds the round's prose, claim ladder, coverage table and limits; its numbers live in generated spans that `benchstat export` rewrites and `benchstat verify` re-checks, so a hand-edited number fails verification instead of surviving as prose. `report.html` is generated from the same exports and manifest, contains inline SVG and no remote resource, and opens directly from a file. Every number in either report comes from `exports.json`, and deferred and failed coverage rows appear in both.

## Reproduce a published round

The round's analyzer is one Rust workspace package named `benchstat`, holding validation, estimators, verification and report generation. It depends on no product crate, so an analysis cannot change what it measures, and its estimator core has no I/O beyond the round directory. The ruling that tools live under `crates/` places it there with `publish = false`: it is a repository tool, and publishing waits until a consumer outside this tree exists.

The package is not written yet, so no round meets this contract today and `benchstat` is not yet a command a reader can run. The interface it must expose is the completion condition for the next stage:

```sh
benchstat export docs/benchmarks/<round> --write
benchstat verify docs/benchmarks/<round>
benchstat report docs/benchmarks/<round> --write
```

`export` validates the manifest, registry coverage and observation schema, computes every exported statistic, then writes `exports.json`, the README's generated spans and `report.html`. `verify` recomputes every export from the published subset in a fresh invocation and compares `exports.json` byte for byte. `report` re-renders only exported values. A round is publishable only when all three succeed on its committed inputs, and its README names the analysis version and the commands that produced it.

Historical artifacts, including the [v0.0.11 observations](v0.0.11/README.md), remain unchanged. Their historical verifier is not rewritten or presented as the next round's analyzer.

## Design decisions

**Rust with one executable verifier, not parallel Python estimators.** Repository tooling is Rust, and one implementation avoids disagreement between generator and verifier. The dip implementation follows the published AS 217 algorithm and is checked against the reference package's public vectors. GPL-licensed `diptest` and `multimode` code is not vendored into this MPL-2.0 tree. The acceptance requirement is complete recomputation from the published subset, not a particular verifier filename.

**BCa and automatic block lengths, not percentile intervals with a fixed cube-root rule.** The interval must respond to observed bias and dependence, and the selected block length must be visible enough to reproduce and challenge.

**KDE modes and antimodes, not forced two-cluster k-means or a dip-interval midpoint.** Forced k=2 returns two centres even for unimodal data and moves with outliers; a midpoint is not a named cut-point estimator. Dip supplies the modality diagnostic, while KDE supplies the shape summary.

**Pettitt and descriptive effect sizes, not a threshold presented as a change-point test.** The permutation test supplies calibration; split-half and CUSUM displays show the magnitude and shape of the change.

**Newcombe differences and exact binomial power, not Wilson-overlap rules and arcsine-only sizing.** Each two-rate claim receives an interval for the actual contrast, and near-certain proportions do not rely on a normal approximation for their primary sample-size calculation.

**One self-contained HTML view beside the Markdown report, not LaTeX or PDF.** Inline SVG carries the charts without network access, the Markdown remains reviewable as text, and both consume the same exports.

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
