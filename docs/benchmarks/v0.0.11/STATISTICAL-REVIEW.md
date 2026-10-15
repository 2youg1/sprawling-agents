# Statistical review — v0.0.11 benchmark subset

Independent re-analysis of `results.csv` (sha256 verified against `manifest.json`), `README.md`, `manifest.json` and `verify.py`. All exported quantiles recompute exactly and all 300 rows validate against the manifest coverage counts. This review does not rerun any workload; it evaluates **measurement methodology** and extracts **engineering signals** from the published numbers.

Verdict up front: measurement *hygiene* (integrity hashing, offline recomputation, scope disclaimers, missing-data policy) is well above typical practice. The *statistical power and summarization* do not yet support the granularity of the presented comparisons. The published numbers are best read as warm steady-state observations on one machine, and several headline figures are less stable than their formatting implies.

## 1. Measurement-method findings

### F1. P99 at n=200 is a 2-sample estimate and is unstable under heavy tails

Nearest-rank P99 with n=200 is the 198th order statistic — it is determined by only 2–3 tail samples. Bootstrap 95% CIs of the published P99 (resampling `values_json`):

| series | published P99 | bootstrap 95% CI | width / point estimate |
| --- | --- | --- | --- |
| sprawling help | 98.6 ms | [97.1, 946.5] ms | 862% |
| deepseek-harness first_read | 149.7 ms | [55.9, 218.6] ms | 109% |
| pi project_first_answer | 1061.0 ms | [680.4, 1120.9] ms | 42% |

sprawling/help contains a single 946.55 ms outlier (9.6× P99); under resampling the P99 cell jumps between ~97 and ~947. Working rule for this dataset: any cell with CV > 0.3 or max/P99 > 1.5 has an unreliable P99. That covers sprawling help (CV 0.94), browser_reload (0.80), sprawling first_read (0.46), codex help (0.42), codex first_read (0.40), deepseek-harness first_read (0.37) and others. Tight distributions (e.g. pi help, CV 0.047) are the exception where P99 is trustworthy.

### F2. Several headline distributions are multimodal; two-point summaries erase this

Two-cluster decomposition (1-D k-means, k=2) of published samples:

| series | cluster centers | low-cluster share |
| --- | --- | --- |
| codex help | 43 / 137 ms | 30% |
| deepseek-harness help | 137 / 213 ms | 10% |
| sprawling first_read | 5.4 / 15.2 ms | 44% |
| codex first_read | 11.6 / 28.5 s | 36% |
| hermes first_read | 2.3 / 3.7 s | 38% |
| hermes project_first_answer | 10.0 / 19.5 s | 56% |
| sprawling recover | ~8 / ~20–30 ms | varies 38–68% by cohort |

Implication: for a bimodal cell the P50 landing cluster is close to a coin flip. sprawling first_read P50 = 13.95 ms sits on the slow cluster; a small shift in cluster share would "improve" the P50 to ~5 ms with zero engineering change. Report cluster centers and shares (or a density plot), never P50/P99 alone for these cells. Multimodality also means cross-agent P50 comparisons compare differently-shaped distributions at different cut points.

### F3. "Median of 3 cohorts" hides between-cohort instability

Between-cohort range of per-cohort P99, relative to its median: codex exec 75%, deepseek-harness exec 94%, sprawling recover (P50) 77%. With only 3 cohorts, the median discards two thirds of the cohort-level information — and cohort-level spread is exactly the signature of environment drift. Prefer pooling the 600 samples into one quantile with a bootstrap CI, or report the cohort range explicitly.

### F4. The i.i.d. assumption is violated inside groups

- sprawling recover cohort 3: first-half median 12.1 ms vs second-half 23.9 ms (1.97× drift).
- deepseek-harness exec cohort 3: 25 samples >700 ms concentrated at indices 143–171 (burst contamination); cohort P99 2073 ms vs sibling cohorts 681 and 1479 ms.
- pi recover cohort 1: 1.54× split-half shift.

Non-exchangeable samples make per-group quantiles regime mixtures. Add a drift gate (split-half ratio or CUSUM) as an acceptance criterion for each group's data.

### F5. RAM series: pseudo-replication, inverted states, construct mismatch

- 40% of the 120 rows have window P50 == P99 == max: the 150 samples at 100 ms carry no distributional information. Each window is effectively one point estimate, so a cell rests on ~3 quasi-independent numbers (3 cohorts), not 150.
- 24/60 cells show active_waiting < idle (deepseek-harness up to −237 MiB at 40 sessions; codex, pi also affected). This is physically impossible if the two states are phase-aligned; they capture different process-lifecycle phases. Presenting them side by side invites a meaningless subtraction.
- hermes at 40 submitted sessions has in_flight=4 (its queue); all other agents measure 40 in-flight. The 40-session active column therefore measures different constructs across agents (queue memory vs concurrency memory).
- Redesign: one point per cohort with more cohorts; idle and active as separately measured cohorts; queue depth fixed across agents or modelled as a factor.

### F6. Task-outcome series are underpowered and mix failure modes

- n=6 per cell: 6/6 → Wilson 95% CI [61%, 100%]; 5/6 → [44%, 97%]. The intervals overlap completely; no pass-rate ranking is statistically supported.
- Power: distinguishing 70% vs 95% at α=0.05, power=0.8 needs n≈32 per arm; 83% vs 100% needs n≈22. Current n=6 is 4–5× short even for large effects.
- gpt_coding: all 5 failed cells are exactly the 5 usage-incomplete requests (one HTTP503, one pre-response connection failure, one non-terminating argument stream, two truncated streams). The pass counts therefore record transport/provider reliability events, not task capability. Infra failures must be a separate category, not a functional zero.
- deepseek-harness/toposort hit the 300 s deadline: recorded duration is a lower bound with unknown exit — right-censored. Any wall-time sum over such cells is biased downward.
- 22/60 planned coding cells unattempted, concentrated in jsonlines/money/redact/relative_path (all agents) plus csv (codex, hermes). Missingness is case-dependent (MNAR); per-agent pass rates over attempted cells are biased upward.

### F7. Remaining confounds in what each cell measures

- codex file tools run through PowerShell; the other agents use native file tools. Tool-cycle latency measures implementation × transport jointly.
- deepseek-harness speaks the Messages API; the others speak Responses. Serialization cost differs.
- sprawling GPT cells route through a compatibility gateway (prompt_cache_breakpoint stripped): not the published binary's native path.
- The GPT model identity is a provider-supplied label, not attested.

All four are disclosed — good practice — but they bound every cross-agent comparison to "as configured", not "as shipped".

## 2. Engineering signals (where the numbers say to optimize)

### S1. Model request count dominates wall time; local tool-cycle wins are rounding errors

Shared 5-case survivor subset: sprawling 646.9 s / 59 requests vs pi 282.2 s / 29 requests. Wall ratio 2.29×, request ratio 2.03×, per-request time nearly equal (11.0 s vs 9.7 s). Local tool cycles at ~54 ms contribute at most ~3.2 s of the 647 s total. Halving sprawling's request count to pi's level would cut wall time ~51%; reducing tool cycles to zero saves <1%. Corroborated by the DeepSeek series, where sprawling hit the 12-request guard in 4/6 tasks. Optimization priority: loop discipline and request budgeting, not micro-latency. (Caveat: survivor-selected subset.)

### S2. Session-memory scaling is linear; the slope is the metric that matters

Linear fit of published medians (MiB per session): pi 81.3 (R²=1.000 — process-per-RPC-session confirmed quantitatively), deepseek-harness idle 10.0, codex 5.0, hermes 0.62 (queue-bounded, but see the F5 construct mismatch), sprawling idle 0.11 / active 2.77. For capacity planning, slope × sessions + intercept is the right model. sprawling's slope advantage is real; the active-state numbers need the F5 fixes before they can be trusted.

### S3. Warm-up / cold paths are 18–39× steady state and invisible in the headline tables

First action per group vs steady-state P50: codex 18–39×, sprawling edit ~18× (~1 s), pi recover 2.4×, deepseek-harness 1.1–1.3×. Users pay this every session; the published distributions exclude it by design. Track cold-path latency as a first-class metric.

### S4. Tail outliers beyond P99 are profiling targets

sprawling help max 946.55 ms (9.6× P99); browser_reload max 2899 ms (11.2× P99); hermes help max 11.59 s (6.9× P99); deepseek-harness exec cohort bursts to 3.4 s. These look like GC / JIT / scheduling / flush events. A max-spike budget plus tracing on the slowest sample would identify them.

### S5. Bimodal code paths are optimization targets with known payoff

sprawling first_read: 5.4 ms vs 15.2 ms clusters (44/56) — making the slow path rare is worth ~3× on this metric. sprawling help: ~26 ms vs ~90 ms. hermes project_first_answer: 10 s vs 19.5 s. Each bimodal cell is a "find the second code path" task.

### S6. IO accounting separates architectures cleanly

Mean per basic task: codex writes ~189 MiB (DeepSeek series ~337 MiB — PowerShell pipeline/transcript artifacts worth investigating) and reads ~298 MiB; hermes reads ~341 MiB; sprawling reads ~239 MiB / writes ~1.2 MiB; pi ~20 MiB / ~0.1 MiB. For sprawling specifically, read amplification (~239 MiB per task) is the next thing to explain after request count.

## 3. Methodology recommendations for the next iteration

- R1. Distribution-first reporting: publish density shape or cluster parameters per cell; flag modality; never report P50/P99 alone for multimodal cells.
- R2. Attach bootstrap CIs to every reported quantile; choose sample size from a target CI width, not from convention (n=200).
- R3. Pool cohorts for quantiles; report cohort range as a drift indicator; add a split-half/CUSUM drift gate per group.
- R4. RAM: one point per cohort with more cohorts; idle and active as separate cohorts; queue depth fixed or modelled; never place non-phase-aligned states in one column pair.
- R5. Outcomes: switch primary metrics to continuous ones (requests, wall time, tokens) with pass/fail as a gate; if pass rates are compared, size cells from a power analysis (n≥22–32 per arm for the gaps of interest).
- R6. Classify failures: infrastructure (transport/provider) vs capability vs guard/timeout; exclude infra from pass-rate denominators and report reliability separately; treat deadline hits as censored.
- R7. Pre-register which claims each series is allowed to support (the current scope disclaimers already do much of this — keep and tighten).
- R8. Interleave/randomize run order across agents to bound provider drift; keep per-request latency logs so wall time decomposes into model time, tool time and queue time.

## Appendix. Reproduction

- Integrity: `sha256(results.csv)` equals `manifest.files.results.csv`; `python verify.py` re-checks all exported quantiles and counts.
- Cluster, bootstrap and slope numbers above were computed from `values_json` with short pandas/numpy scripts; they can be added as a companion `review.py` in a follow-up if wanted.
