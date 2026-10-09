# Monitoring and performance evaluation

Use the performance page for a live city's resource readings, the conversation and cost views for model and tool activity, and `sprawling gauge` for repeatable command or process measurements. These answer different questions: provider response time includes external waiting, while a local harness benchmark measures the work sprawling owns.

The [performance register](../tools/xtask/budgets.toml) is the authority for project budgets, recorded baselines, measurement recipes and whether a reading is owed. The [architecture's performance section](../ARCHITECTURE.md#the-performance-register) renders its managed figures and explains their scope. Read each row's `status` before using a number: a reading from an earlier tree does not establish the performance of the current release. This guide supplies methods, not a second baseline table.

## Watch a city

Open Settings → Diagnosis → Performance, or the client's `#/monitor` route. The settings summary watches the core process alone; the full performance page also reads machine CPU, available memory and free space on the city's volume. The page's sampling control changes the city's beat. [Beat](../crates/sprawling/src/monitor/beat.rs) applies it immediately and saves it for the next serve; if saving fails, the console reports the failure and the change lasts only for this process. A missing saved value uses the default; an unreadable or invalid value also uses the default and reports the failure. [BeatMs](../crates/wire/src/frames/monitor.rs) owns the default and allowed range; [the sampler](../crates/sprawling/src/monitor/sampler.rs) owns the relation between memory sampling and emitted readings.

Private memory is sampled at the configured memory beat, and monitor readings are emitted after the sampler's `MEMORY_BEATS_PER_BEAT` memory beats. A reading's `core_private_bytes` is the largest memory value observed since the previous emitted reading, not its mean or an instantaneous value. Changing the memory beat also changes the emission interval; use `beat_ms` and the linked sampler's emission multiplier to interpret a saved stream. Scheduling and counter-read costs can lengthen real intervals, so this is a sampling cadence, not a deadline guarantee.

The history has a fixed point capacity, defined by [Monitor](../crates/sprawling/src/monitor.rs) and proved in [its Lean model](../crates/sprawling/spec/Monitor.lean). Its time span therefore changes with the beat. When nobody watches, the sampler releases counters and history at its next monitor tick. Opening the page again starts a new history. A summary watcher keeps sampling active, and a full-page watcher takes precedence over summary watchers. Close both when collecting an unwatched idle measurement.

A terminal can watch the same feed:

```sh
sprawling gauge --at <address>
```

Use the address the CLI or the quiet host shows; `top` is an alias for `gauge`. On the machine that serves the city, `gauge` reads the key file the city writes for that port, so it needs no `--token`. <!-- v0.0.11-verify --> Redirect stdout to keep JSON lines outside the city's history:

```sh
sprawling gauge --at <address> > city.jsonl
```

Stop the watcher when the workload ends. A city watcher also ends normally after transport silence reaches [the watcher's `SILENCE` limit](../crates/sprawling/src/wire_client/watching.rs), or the socket closes. This does not prove that the city stopped: a long configured emission interval can exceed that limit in an otherwise idle city. For a continuous terminal capture, choose a cadence whose emission interval stays below the limit with room for scheduling delays, and check that the recording covers the whole workload. `--every` and `--samples` belong to process and command measurements, so do not add them to the city form. Change the city's cadence through the performance page instead. Measurement files are disposable observations and do not enter the Ledger; [logging](logging.md) explains the distinction between history and diagnostics.

## Choose how the city uses the machine

Settings → Performance holds three choices, kept in `[core]` of `~/.sprawling/config.toml` ([the person's configuration](../crates/accounting/src/person.rs)); they apply after the city server restarts.

- **CPU placement** chooses how the core's threads and the runs' commands share the processors. `soft` is the default: on Windows it prefers the fastest cores, lifts power throttling and asks for equal CPU weight for each run; support differs by platform. `none` asks for nothing. `pinned` asks Windows for hard affinity to the placement plan, gives run jobs the remaining processors, prints a `taskset` command on Linux to use at startup, and does not pin on macOS. `soft_shares` ("soft with memory ceiling" on the page) is `soft` plus the per-run memory ceiling below. The [placement plan](../crates/sprawling/src/serving/placement.rs) decides the cores.
- **Core priority**: `raised` asks for above-normal priority for the core's hot threads, and the operating system may refuse it; `normal` leaves them at normal priority. Independently of this choice, run commands start below the core: below-normal priority class on Windows, `nice -n 10` on Unix ([`yielding.rs`](../crates/runtime/src/tools/exec/yielding.rs)), so a build does not starve the city that serves the page.
- **Per-run memory ceiling**, in bytes. There is no default ceiling: an empty field means none. A number applies only with `soft_shares`, and all commands of one run share it. Windows jobs and delegated Linux cgroups enforce it; elsewhere each command says the ceiling did not apply. An allocation above it fails inside that run, and the command's result and the monitor and inspector terminals say the ceiling was reached, or that it could not be applied or read back ([`Tools/Exec.lean`](../crates/runtime/spec/Tools/Exec.lean) D95).

The four placement arms were not measured against each other for this release; `soft` is the default by design, not by a reading.

Nor were the console and the local door measured for this release. The quiet host writes nothing to the terminal per record, and the CLI writes through one bounded channel, so neither should add work per event; a page visit adds one loopback round trip and one signature check when it obtains its session token, and each WebSocket upgrade or POST adds a header comparison and one digest lookup, with nothing added per frame. These are expectations from the design, not readings: the register holds no figure for them yet.

## Interpret the fields

The [wire Sample](../crates/wire/src/frames/monitor.rs), [counter readers](../crates/sprawling/src/monitor/counters.rs), [accounting health](../crates/accounting/src/worker/health.rs) and [sampler](../crates/sprawling/src/monitor/sampler.rs) define the following fields. A city field that the platform does not supply is zero; the first CPU reading is also zero because it has no earlier reading to compare with. These zeros do not prove absence of resource use.

| City field | Unit and meaning |
|---|---|
| `core_cpu_permille` | CPU time spent by the core process over the sampling interval, divided by available logical processors, clamped to a whole-machine share. A permille is one tenth of a percentage point. |
| `core_private_bytes` | Maximum of the core process's memory readings within this emission interval; the counter's platform meaning is listed below. |
| `core_working_set_bytes` | Resident physical memory of the core process at the emitted reading, including pages that need not be private. |
| `core_read_bytes`, `core_written_bytes` | Cumulative storage bytes for the core process, not bytes per second. The current own-process reader supplies them on Linux; elsewhere they remain zero. |
| `machine_cpu_permille` | Machine-wide CPU utilisation from sysinfo, in permille. |
| `machine_available_bytes` | Available machine RAM, not RAM assigned to the city. |
| `volume_free_bytes` | Free space on the volume containing the city. |
| `ledger_queue_depth` | Append requests waiting for the accounting thread. |
| `durable_lag` | Appends taken into a batch whose durability barrier has not returned; a count, not a duration. |
| `view_backlog` | Committed records handed to the view thread but not yet folded and broadcast. |
| `read_nanos` | Counter-reading work for the preceding emitted beat, measured with a monotonic clock; the first reading is zero. It excludes the intervening memory-only reads and does not measure the sampler's entire CPU cost. |
| `beat_ms` | Configured memory sampling beat in milliseconds. |
| `relay_p50_nanos`, `event_to_screen_p50_nanos`, `queued_runs` | Protocol fields with no production measurement source wired into this Sample yet. Their zero values are not measured latency or proof that no runs wait. |

CPU is normalised over the whole available machine, so a process occupying one logical processor need not display full utilisation. The city CPU reader divides by the nominal configured interval; oversleep and counter-read delays can bias its share upward. Compare cumulative I/O deltas only within the same process lifetime, and divide by observed elapsed time when calculating rates. Summary-only samples do not read machine or volume counters.

The terminal and client show bytes in base-1024 units, durations in ns, µs or ms, and permille as percentages. [The terminal formatter](../crates/sprawling/src/monitor/top.rs) and [client monitor projection](../client/src/core/monitor.ts) define display conversion and truncation. Keep raw integers when analysing a stream: displayed truncation loses detail. The page reports p50 and p99 over the visible point window, which can be shorter than the retained history. Page plots and terminal sparklines scale each row to that row's visible minimum and maximum, so equal heights in different rows do not mean equal values.

### Memory differs across platforms and subjects

[Memory D43](../crates/sprawling/spec/Serving/Memory.lean) and [OwnProcess](../crates/sprawling/src/monitor/counters/own_process.rs) define the city's own-process counter:

| Platform | What city `core_private_bytes` currently reads |
|---|---|
| Windows | Private committed memory through memory-stats' `PagefileUsage`. It is not the working set. |
| Linux | `Private_Clean` plus `Private_Dirty` from `/proc/self/smaps_rollup`; fallback to `/proc/self/status`'s `RssAnon` when no usable rollup value is available. The fallback omits swapped-out private pages and can understate the footprint. If neither source can be read, the value is zero. |
| macOS | Virtual size through memory-stats, pending an admitted safe interface for `phys_footprint`. This can greatly exceed private physical memory. |

Sample carries no source tag, so a saved Linux stream does not identify which fallback was used, and a macOS stream does not relabel the field as virtual size. Record the platform and counter source beside the result; do not rank these values as if the counters were interchangeable.

Process-tree gauge readings use a different adapter: [Tree](../crates/sprawling/src/monitor/tree.rs) sums sysinfo's `virtual_memory()` into `private_bytes` and `memory()` into `working_set_bytes`. In particular, a tree's `private_bytes` is not the Linux smaps value used by the city monitor. Name the subject and adapter when reporting memory. Windows command runs can additionally report platform-recorded direct-child peaks after exit; they do not cover the entire descendant tree.

An emitted city memory stream contains interval maxima. Percentiles over those lines are percentiles of interval maxima, not of every memory beat. The current feed does not expose the intervening values, so it cannot establish the raw-beat p50, p99 or exact time above p50. [The memory measurement plan](../crates/sprawling/spec/Serving/Memory.lean) also calls for a one-hour slope and allocator comparisons; these are measurement requirements, not statistics the page already computes.

### Model time, tokens and money

Conversation rounds expose model `first_us` (attempt sent to first content) and `took_us` (attempt sent to complete reply), plus tool `took_us`. These are whole microseconds from a monotonic clock. Older records may omit them; [Used and Call](../crates/wire/src/answer/rounds.rs) describe the fields and the view's fallback to differences between event moments. Event `t` values are integer milliseconds and can move backwards with the wall clock, so prefer explicit duration fields for sub-millisecond comparisons. Inspect records with:

```sh
sprawling view <city>
```

Keep model latency separate from local harness latency. Provider waiting, network and generation affect model time; tool duration can include a child command or external service. Neither is a measurement of only the city's accounting or rendering work. For generation throughput, state the exact token counter and timed interval rather than calling output tokens divided by whole-request duration a streaming generation rate.

[ModelUsage](../crates/kernel/src/model/usage.rs) defines `input_tokens` as the entire prompt, including cached tokens, and `output_tokens` as output. Cache-read and cache-write counts are parts of input, not extra tokens to add to it. Unreported cache usage differs from a reported zero. The version-aware reader handles older payloads; use the product's folded view rather than inventing a conversion from old JSON.

The cost view's [CostAnswer](../crates/wire/src/answer/cost.rs) uses integer `UsdMicros`, millionths of a US dollar. Its total contains authoritative priced amounts; `unpriced.calls` and `unpriced.tokens` preserve activity for which no amount was supplied. A zero total with unpriced calls does not mean the work was free. The limited `by_run` list can sum to less than the total; it is not a second city total. Cost and token usage come from recorded model activity, not the resource Sample.

## Measure a command or process

The shipped [gauge skill](../crates/city/skills/gauge/SKILL.md) gives the before/after procedure, and `sprawling help gauge` lists the grammar. Angle-bracket arguments in these examples are placeholders to replace.

```sh
sprawling gauge --samples 20 -- <program> <args> > before.jsonl
sprawling gauge --samples 20 -- <program> <args> > after.jsonl
sprawling gauge --pid <pid> --every 1000 --samples 20 > process.jsonl
```

For a command, wall time runs from immediately before spawn until wait observes exit; it is independent of the resource sampling beat. stdin is closed, and the command's stdout and stderr both go to gauge's stderr, leaving gauge's stdout for readings. Arguments after the first `--` belong to the measured command. This matters for interactive commands and commands whose output is itself a workload input.

A command writes a `run` line for every attempt and a final `spread` line. Keep the first run, which may be cold. Read `samples`, `failed`, `floor_us`, `p50_us`, `p95_us`, `p99_us`, `peak_us` and `suspicious`. [Spread](../crates/sprawling/src/monitor/spread.rs) uses nearest-rank percentiles and owns the suspicious-run threshold. Small sample counts cannot resolve a tail distribution: gauge does not emit p999, and the performance contract in ARCHITECTURE specifies when enough observations exist to report it.

A successful gauge exit means measurement completed, even when the child failed. Reject failed workload samples using each run's `exit` and the spread's `failed`; also handle gauge's own nonzero exit for an invalid invocation, missing program/process or unreachable city. An exit with no child code can be `null`. Preserve diagnostics alongside the reading.

The resource fields `seen_*` are what the sampled tree actually contained, and are lower bounds: a process born and gone between beats is missed. With no successful beat, they are `null`. `seen_cpu_us` comes from a millisecond-resolution platform counter despite its microsecond unit. `child_peak_*` is available for the direct child on Windows and `null` on other platforms. `read_cost_us` records process-table reading time, exposing part of measurement interference. Process-tree sampling walks the whole process table; [Every](../crates/sprawling/src/main/gauge.rs) owns its lower bound and default. Increasing command repetitions improves wall-time coverage but does not fix missed short-lived descendants.

Pin input digests and build settings, change one thing, then alternate before/after rounds on the same machine class. Report floor and p50 together, retain outliers, and state the background load. Use operation counts for regression gates where possible; shared-runner wall-clock thresholds can confuse contention with a code defect.

## Reproduce repository workloads

These are contributor commands from [justfile](../justfile). Run prerequisites first with `just prereqs`; the recipes build what they measure. Before `just mem` serves a city, run `just web-bundle` to prepare the real client: the memory recipe builds the binary but does not build its client. `just bench-startup` includes that prerequisite. Save build output separately from timed output, and use the product feature set and [release profile](../Cargo.toml). A debug build, a build with test-only features, or a binary without the real client is not the release workload. Record any `RUSTFLAGS`, target CPU setting or PGO profile rather than assuming the host's flags.

| Command | Work and timing boundary |
|---|---|
| `just bench` | Release citysim bench, followed by ignored production instruments. Ledger append/barriers, prefix assembly, run-history reads, large-ledger fold, new and kept worktree placement, local forwarding, and the registered instrument tests. The relay and throughput instruments drive production accounting rather than a copied loop. |
| `just bench-startup` | Builds the client and release binary, then measures install with the archive already present, process startup, raising a city, opening a session and first-byte startup. The harness prints sample counts, substeps and per-action process/file/barrier counts. It does not time network download. |
| `just mem` | Builds the release binary and reads an empty served city after it accepts a connection and settles. `just mem <pid>` reads an existing process. This is one observation through xtask's adapter, not a sampled distribution; its counter names travel with the output. |
| `just mem long-turn 500 100` | Builds the release long-turn instrument, reads memory at its synchronised pauses, and continues it after each reading. The arguments are workload steps and pause frequency, not a city monitor beat. |
| `just sim` | Deterministic scripted scenarios on a counted clock. This verifies behaviour and replay; virtual scenario time is not a wall-clock benchmark. |

[The xtask memory adapter](../tools/xtask/src/mem.rs) differs from both the city and process-tree adapters above. On Windows it reads `PrivateMemorySize64`, `PeakPagedMemorySize64` and `WorkingSet64`. On Linux its private value is the smaps private sum, its working set is `VmRSS`, and its field labelled peak private is `VmHWM`, a peak resident value that includes shared pages. On macOS it uses one current `ps rss` reading for all three values; it does not measure a private or historical peak value. Keep these source names with the result. For a served fixture, the adapter attempts to stop the process and clear its temporary city even when acceptance or measurement fails; an existing city supplied with `--city` is retained. A missing counter or failed platform command returns an error, so keep stderr and the command's exit code.

For first-byte startup on the harness's empty and historical cities, the recipe takes no argument; use the command sequence recorded in the register's `[first_byte].measured_by`:

```sh
just web-bundle
cargo build --release -p sprawling --locked
cargo run --release -p citysim --bin bench_startup --locked -- first-byte
```

[The registered bench fixture](../tools/citysim/src/bin/bench/scenarios.rs) owns the load sizes and pinned digest. Its digest check refuses changed fixture bytes before measurement. Record the emitted fixture identity and each row's sample count; different scenarios use different repetition counts, and repeated gauge invocations are not those internal samples. [Startup actions](../tools/citysim/src/bin/bench_startup/actions.rs) own the startup repetition count. [Bench specifications](../tools/citysim/spec/Bench.lean) and [startup specifications](../tools/citysim/spec/BenchStartup.lean) explain the excluded work. Local forwarding excludes network transport; run-history timing excludes socket/framing; stocked-worktree placement excludes preparing the stock before the clock starts.

The throughput workload and narrowing inputs are documented in `[throughput].measured_by` in the register and [Throughput.lean](../tools/citysim/spec/Throughput.lean). Record every selected arm and provider-latency input. Synthetic provider latency is not a live model benchmark. `just bench` selects the nextest `bench` profile so the throughput instrument can finish its provider-wait matrix; that profile has a dedicated timeout override in `.config/nextest.toml`. CPU placement's planned multi-arm comparison and the one-hour resident-memory evaluation remain unmeasured as described in ARCHITECTURE; no benefit follows merely from their existence in the design.

## Publish a reproducible result

Attach this provenance to each result, whether it is a project measurement or a community submission:

| Record | Required content |
|---|---|
| Build | Product version; full commit and tree SHA; clean/dirty state; binary/archive digest; toolchain; target; features; profile and extra compiler/PGO flags. |
| Machine | OS/build and architecture; available logical processors; RAM; disk kind; runner/VM class; relevant limits and background load. Omit account names, host names and personal paths. |
| Workload | Exact command; input and fixture digests; concurrency/selected arms; cold/warm preparation; repetition count; sampling beat and counter source. |
| Evidence | Raw observations, workload and measuring-process exit codes, failed/outlier counts, units, percentile method, scope/exclusions, and the downloadable log/artifact or public source. Actions results also need run URL, attempt and checked commit/tree. |

Keep the register as the sole project baseline authority. A new baseline or budget change belongs there through its existing review process; this guide and README should reference it or use managed spans. Community measurements remain attributed observations with their own hardware and source links; they neither replace that baseline nor form a cross-device speed ranking. No community results are included here yet.

Build-cache reuse and performance are separate evidence. The project does not introduce kache in this version. Reconsider it only after the existing cache/parallel verification changes have been measured and repeated workspace/Clippy compilation is still a main cost, or frequent worktree builds become a separate need. A subsequent fixed-version, isolated Rust-only experiment must count installation, restore/save and copying in net time, check input invalidation, trybuild and diagnostics, and verify wrapper-free builds. An upstream cache benchmark does not measure sprawling, and compiler caching does not cache a test verdict.

### Final release measurements through Actions

Use the final integrated ref, not an earlier branch's green result. The existing [on-demand workflow](../.github/workflows/on-demand.yml) can build a release artifact and run simulation or fresh-install acceptance:

```sh
gh workflow run on-demand.yml --ref <final-ref> -f job=release
gh workflow run on-demand.yml --ref <final-ref> -f job=sim
gh workflow run on-demand.yml --ref <final-ref> -f job=fresh
```

Its release artifact is named with the checked tree. The simulation result proves scenarios, and fresh acceptance checks the archive-to-dispatch route on the workflow's platform matrix; neither produces the bench or memory distributions above. Its `bench` choice runs `just bench` on a Windows runner and uploads raw logs and build provenance; with `responses_base=<ref>`, it instead compiles matched Responses instruments before alternating latency rounds on Windows and recording allocation profiles through heaptrack on Linux. Its `pgo` job trains and builds an optimised binary; it does not by itself establish an improvement over an unprofiled arm.

The `bench` choice supplies the Actions path for `just bench` and the matched Responses instruments; startup, first-byte and memory-beat workloads still need execution on the final integrated tree. Use the existing workflow as the integration point, with its owner reviewing any job extension, and dispatch it with `-f job=bench`; add `-f responses_base=<matched-ref>` for the Responses comparison. Upload provenance, raw logs, exit codes and fixture identities for each platform, with performance measurements run alone. Keep failure artifacts, separate compilation from measured spans, and compare only matched fixtures/builds/machine classes. A Windows hosted release job alone cannot establish Linux or macOS counter behaviour, physical-device latency, or the missing raw memory-beat distribution. Close each evaluation only when those actual observations exist; retain precise gaps otherwise.

### Check onboarding objectives separately

The fifteen-minute route for experienced agent users and thirty-minute route for chat users are guide objectives, not measured completion times. For each route, start a new user with the release archive/package and its prerequisites, record the starting state and elapsed time, then verify installation, migrated or first-time model/tool configuration, the first task, reading its report and stopping work. State whether downloads, account setup and provider waiting are included, and preserve failed steps as evidence. [The getting-started guide](getting-started.md) owns the route; fresh-install Actions can check its executable steps but cannot demonstrate human learning time or the interactive browser experience.
