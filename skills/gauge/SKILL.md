---
name: gauge
description: "Measure before and after a performance change with `sprawling gauge`: a command's wall time over n runs, a running process tree's CPU, memory and storage, or a served city's monitor. Use when making something faster or smaller, checking a regression, or reporting a timing — in this repository or any other."
license: MPL-2.0
---

# Measuring with `sprawling gauge`

A reading is evidence only when the load, the build and the machine class behind it are written beside it. Work through the steps in order; each ends on a condition you can check.

## Steps

1. **Pin the load.** Run the same input on the same build each time — the product's own feature set, not a debug build of something else. When the input is a file, record its digest (`sha256sum`, `Get-FileHash`); two readings compare only when their digests match. Done when you can name the command line, the build and the digest.

2. **Take the baseline.**

   ```text
   sprawling gauge --samples 20 -- <program> [arg...] > before.jsonl
   ```

   Every run writes one `run` line and the last line is the `spread`. The first run is the cold one and is kept. Read `floor_us` and `p50_us`: the floor is what the work costs while the machine is quiet; p50 carries the rest of the machine's load. When `suspicious` is not 0, look at which runs were more than three times p50, and whether one of them was the first, before you trust the spread. When `failed` is not 0, the command failed on some runs; its exit codes are in the `run` lines, and `gauge` itself still exits 0.

3. **Change one thing, then alternate.** Take `after.jsonl` the same way. Run a few rounds of before-then-after, so a slow minute on the machine falls on both sides. Compare floor with floor and p50 with p50, and only between readings from the same machine class.

4. **Gate on a count, record the clock.** A wall-clock threshold on a shared runner turns red and green on its own. For a regression gate, find a count that grows with the work — bytes read, rows checked, file operations — and assert it at two sizes, N and 2N. Keep the wall-clock readings as a record beside the gate.

5. **Read resources when time is not the question.**
   - A long command: the `run` line's `seen_*` fields are what the beats saw of its process tree (lower bounds: a process born and gone between two beats is missed), and `child_peak_*` is the platform's exact peak for the direct child, on Windows.
   - A process already running: `sprawling gauge --pid <pid> [--every <ms>] [--samples <n>]`, one `tree` line per beat.
   - A served city: `sprawling gauge --at <addr> > city.jsonl` records in the background for as long as it runs; a 0 in a `city` line can mean "not read", while `null` in every other line means "not measured".
   - `read_cost_us` on a `run` line is the time spent reading the process table: how much the measuring itself disturbed the run. `--every` is 250 ms at the shortest for that reason.

6. **On sprawling itself.** How long a model or tool call took is in the Ledger: `sprawling view <city>` shows the `t` of `model_called` and `model_returned`, and of `tool_called` and `tool_result`. How long the city took to open is the `opened the city in` line `serve` writes. A dispatch's preparation is a trace line; serve with `--log trace` to see it. Operations a person waits on belong in milliseconds and internal hot paths in microseconds; a reading in seconds is a finding to act on.

7. **Report.** Give the machine class — cores, memory, disk kind — and the load's digest, with floor, p50 and the number of samples. Leave out machine names, account names and absolute paths.

## Reference

- `sprawling help gauge` prints every flag. The first `--` ends gauge's own words; everything after it is the measured command's, `--help` and `--version` included.
- Lines are JSON when stdout is a pipe or a file, and lines in units at a terminal. Keys end in their unit (`_us`, `_ms`, `_bytes`, `_permille`); a key with no unit is a count.
- Exit codes: 0 the measurement was made; 1 the program could not be started or the process does not exist; 2 the command line could not be read; 4 no city at `--at`.
