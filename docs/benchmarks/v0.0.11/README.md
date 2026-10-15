# v0.0.11 Windows benchmark observations

These observations cover the fixed release `v0.0.11-Alpha-261009` on a Windows 11 x86_64, i7-14650HX, 32 GiB, NVMe machine. They are preliminary observations of the named versions and workloads, not the current main branch, a cross-device ranking, or an update to the project performance register. Other project names identify the necessary subjects of independently collected measurements; no upstream implementation or model instructions are reproduced.

The measured release has low local process-start and session-memory overhead, and its search cycle is short. Those properties do not establish the best end-to-end task performance: editing, shell execution, model round trips and normal task termination expose different trade-offs.

## Evidence and scope

- [results.csv](results.csv) contains latency samples, per-cohort memory summaries and task outcome records.
- [manifest.json](manifest.json) identifies the target commit/tree, entrypoint digests, conditions, accounting and incomplete coverage.
- `python verify.py` recalculates exported distributions and outcome counts without a provider or credential. It does not rerun native agents or independently validate generated task code.
- All fixed-provider local groups make zero upstream model requests. Live model experiments are separate.
- Published-binary toolchain/features are not independently attested. Raw memory time series, live runners and generated task artifacts are outside this public subset.

| Agent | Version |
| --- | --- |
| sprawling | sprawling 0.0.11 (alpha), released 2026-10-09 |
| codex | codex-cli 0.162.0-alpha.17.2 |
| hermes | Hermes Agent v0.21.6+767.g745380d (2026.9.24) · stable · local 745380d4959e |
| pi | 0.84.4 |
| deepseek-harness | 0.2.0-rc.2 |

## Startup and fixed first answer

Each cell reports nearest-rank P50 / P99 in milliseconds from 200 fresh processes or projects. OS caches are warm. The answer is fixed by a local provider; this is neither model response time nor first browser pairing. Sprawling uses a private runtime-data location and authenticated-ready condition.

| Agent | Help exit | New project to first completed answer |
| --- | --- | --- |
| codex | 129.14 / 160.42 | 347.86 / 532.21 |
| hermes | 1581.98 / 1683.44 | 12466.67 / 25680.71 |
| pi | 442.93 / 518.16 | 463.11 / 1060.99 |
| deepseek-harness | 211.17 / 240.29 | 2516.20 / 3447.07 |
| sprawling | 88.74 / 98.55 | 907.45 / 1764.08 |

Sprawling has the lowest help-exit median in this group. Codex reaches the fixed first answer sooner; a lightweight command startup is a different boundary from preparing a usable project.

The separate sprawling browser reload-to-input-echo workload passes 200/200 trials: P50 223 ms, P99 259 ms, maximum 2899 ms. It uses a paired in-app browser, a running city, warm cache and a 1440x900 viewport, including automation overhead. It does not measure cold boot, first pairing or another product's GUI.

## Fixed file tools

The cycle starts when the local provider emits complete tool arguments and ends when the next model request contains the actual result. It includes native scheduling, execution, serialization and loopback transport, not model selection. Codex uses PowerShell; the other agents use their native file tools.

| Agent | First 1 KiB read P50 / P99 ms | First 1 KiB create P50 / max ms |
| --- | --- | --- |
| codex | 25016.81 / 41933.20 | 13694.27 / 32282.56 |
| hermes | 3570.85 / 4414.64 | 3209.09 / 3714.75 |
| pi | 18.17 / 24.28 | 19.78 / 22.94 |
| deepseek-harness | 44.88 / 149.73 | 53.60 / 83.18 |
| sprawling | 13.95 / 17.58 | 32.00 / 46.84 |

Reads have 200 samples per agent and creates have 20. Creates check exact 1024-byte contents and preserved inputs; fsync semantics are not standardized. First shell preparation is included. These values are not physical disk latency.

The following table reports the median of three independent groups' P50 / P99, in milliseconds. Each group has one separate first action and 200 subsequent distinct actions. Edit includes read-before-edit; recovery includes its error and recovery stages.

| Agent | Search | Edit | Shell exec | Recovery |
| --- | --- | --- | --- | --- |
| codex | 363.91 / 394.94 | 658.35 / 725.85 | 316.11 / 355.74 | 704.73 / 753.89 |
| hermes | 844.57 / 928.92 | 3385.35 / 3747.31 | 378.00 / 446.56 | 2039.70 / 2186.21 |
| pi | 195.62 / 237.64 | 12.14 / 25.33 | 157.60 / 168.79 | 8.18 / 21.96 |
| deepseek-harness | 163.78 / 191.46 | 60.24 / 85.17 | 643.16 / 1478.92 | 49.59 / 66.35 |
| sprawling | 20.13 / 30.22 | 54.53 / 74.11 | 469.14 / 636.75 | 11.98 / 30.96 |

Search P50 is 20.13 ms for sprawling and 195.62 ms for Pi; shell-exec P50 is 469.14 ms and 157.60 ms respectively. No single-tool result establishes an all-tools ranking. Repetition guards remain enabled. The DSH exec groups use a separately repaired test-directory permission condition; initial failures are not represented as successful latency samples.

## Session memory

Each cell is idle / active-waiting private MiB: the median of three independent cohorts' time-window P50 values. Cohorts hold 20 fixed history turns and sample each state for 15 seconds at 100 ms. This is Windows PrivateUsage, not physical resident RAM, GPU weights or a maximum-session capacity result.

| Sessions | codex | hermes | pi | deepseek-harness | sprawling |
| --- | --- | --- | --- | --- | --- |
| 1 | 29.00 / 29.95 | 210.09 / 210.12 | 99.01 / 80.52 | 163.36 / 126.29 | 6.66 / 9.77 |
| 10 | 93.78 / 99.38 | 221.27 / 221.68 | 819.23 / 813.85 | 215.62 / 169.11 | 8.88 / 35.03 |
| 20 | 147.23 / 156.09 | 230.08 / 230.80 | 1633.90 / 1627.93 | 324.66 / 149.16 | 9.84 / 61.90 |
| 40 | 227.99 / 191.60 | 235.56 / 236.05 | 3266.95 / 3259.36 | 544.04 / 319.32 | 11.32 / 118.14 |

At 40 submitted sessions, Hermes has four actual in-flight requests; its queue is not equivalent to 40 simultaneous generations. Pi uses a process per RPC session. Sprawling's active total includes its service and CLI clients. Lower active than idle readings in some agents reflect phase/process lifecycles and do not imply generation has no memory cost.

The separate live DeepSeek generation matrix has 17 normal cells out of 20. Pi/40 and sprawling/20,/40 lack complete normal responses; they are not zero-memory points or evidence of stable high-concurrency completion.

## Live six-case tasks

Each entry is artifact-pass count; normal-completion count, out of six tasks. Tasks cover read, search, precise edit, repair, recovery and CSV aggregation. Each primary task has at most 12 model requests with 4096 output tokens per request. Protocols and native prompts differ as recorded in the manifest.

| Agent | DeepSeek | GPT relay |
| --- | --- | --- |
| codex | 6/6; 5/6 | 5/6; 5/6 |
| hermes | 6/6; 6/6 | 6/6; 6/6 |
| pi | 6/6; 6/6 | 6/6; 6/6 |
| deepseek-harness | 6/6; 6/6 | 6/6; 6/6 |
| sprawling | 5/6; 2/6 | 6/6; 5/6 |

GPT primary data selects 30 cells from 31 retained attempts. Hermes/multi first hits a batch-wide 120-request guard; its separate per-task-budget attempt is primary, and the interrupted attempt remains selected=0. This is not a claim that all original attempts completed.

Sprawling reaches the local 12-request guard in four DeepSeek tasks. In GPT repair, the artifact passes but the native run ends cancelled. Codex's DeepSeek repair artifact passes after the outer timeout; GPT repair does not pass under the request cap. Local guard refusals are not provider rate-limit evidence.

Sprawling's native GPT preflight receives HTTP400 for prompt_cache_breakpoint. Its GPT results use a disclosed gateway removing only that field; they do not establish unmodified release compatibility. The GPT model name is a provider-returned label without independent backend attestation.

## Coding extension

The extension has 12 small Python repair cases with 19 public and 44 holdout probes plus input/file/alias invariants. Each cell permits one attempt, 24 model requests and 300 seconds; it is not a large-repository benchmark. Thirty-eight of 60 cells are attempted, with 22 unattempted.

| Agent | Attempted / planned | Function passes | Contract passes | Normal endings |
| --- | --- | --- | --- | --- |
| codex | 7/12 | 7 | 7 | 7 |
| hermes | 7/12 | 6 | 6 | 6 |
| pi | 8/12 | 8 | 8 | 8 |
| deepseek-harness | 8/12 | 5 | 5 | 5 |
| sprawling | 8/12 | 7 | 6 | 7 |

The five shared coding cases that all agents pass normally are intervals, overlay, pagination, dedupe and quantile. Their recorded serial total is 646.88 seconds / 59 model requests for sprawling and 282.23 seconds / 29 requests for Pi. This matched-success subset describes only those cases, retains survivor selection, and is not a whole-suite success-rate or throughput ranking.

Five coding requests lack complete usage: one HTTP503, one pre-response connection failure, one nonterminating argument stream and two prematurely ended argument streams. Token columns contain known usage only; usage_complete=0 means a total cannot be established. The timeout record has an unknown exit code and a last-resource-sample duration lower bound. Missing usage and unattempted cases are not assigned zero.

## Interpretation limits

- Two hundred samples provide an empirical tail, not a stable cross-machine P99. Small task sets have no task P99. RAM time-window quantiles are not independent-run quantiles.
- Model time, local tool time, request count and successful completion are separate outcomes. More model round trips can outweigh a short local tool cycle.
- The public subset records checker outcomes but excludes generated solutions; the verifier validates counts and numeric distributions, not fresh task correctness.
- Cold-boot GUI, first pairing, sustained-memory slope, real-model maximum capacity, and monitor_beat/view_backlog source instruments are outside completed coverage.
- These observations leave the authoritative performance register unchanged. No account quotas, monetary estimates or personal environment records are part of this publication.
