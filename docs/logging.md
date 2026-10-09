# Logging — the diagnostic log, and why it is not history

> **For someone debugging this**, and for anyone changing the code who has to decide where a line belongs. It gives the division between the Ledger and the log, the five levels, and the rule that keeps them apart.
>
> It does not cover day-to-day operation ([`operating.md`](operating.md)), which is where the command for raising the level is.
>
> `runtime::diagnostics` implements this file; where the two disagree, this file moves first.

## 1 One rule ahead of the others

> **Decision and recovery logic reads no log output.**

A log is **a diagnostic for a person**, not data. Keeping it that way protects two things at once: the Ledger's standing as the only history, and deterministic replay — a log carries thread interleaving and whatever clock a sink adds, and neither reproduces.

The rule is held by structure rather than by discipline: the logging surface **has write methods and no read method**. Reading a log line back inside the code cannot be spelled.

## 2 The division between Ledger and log

This is the most expensive distinction in the design to get wrong.

| | Ledger | Log |
|---|---|---|
| Answers | **what happened** | **what it was thinking at the time** |
| Authoritative | yes | no |
| Disposable | no | yes, at any moment |
| Enters replay | yes | no |
| Cost of losing it | the city is gone | this debugging session is harder |
| Timestamps | integer milliseconds, passed in; a tool call's and a model reply's durations also as integer microseconds off the turn's monotonic clock | none in the line; a sink may add one |
| Floats | none | as you like |

**The test**: delete every log and the behaviour, the replay result, and the reconciliation totals stay byte-identical. When they do not, something has been written in the wrong place.

## 3 Five levels, each with a named reader

Levels here answer **who reads this, and when**, which is a question with a checkable answer.

| Level | Reader | When they read it | Example |
|---|---|---|---|
| `refuse` | the person, live | immediately | what a door refused, in three parts |
| `effect` | the person, afterwards | after something went wrong | which file was written, which provider was called |
| `decide` | the builder | when behaviour is wrong | why a verdict took the value it did |
| `trace` | the builder | when reproducing a defect | phase changes, locks, retries |
| `wire` | the builder | when a protocol does not connect | the bytes of a frame |

A level admits itself and every level above it in the table. **`effect` is the default**, so `refuse` and `effect` are on unless you ask otherwise: together they answer most of what a person asks — why it declined, and what it actually did — and they are small enough to leave on permanently.

`sprawling serve <city> --log <level>` sets the level, `--log off` writes nothing, and a city served with no console prints the level it runs at as `log: <level>` under its banner; the CLI and the quiet host do not print it. A word that is not a level is refused with the list of levels.

## 4 Structured, and anchored to a run

Every line carries three required fields: `run`, `seq` (the Ledger position at the time), and `module`.

`seq` is the one the design rests on: **it anchors the log to the only history**. From a surprising log line, go to that position in the Ledger and read what happened; from a surprising event, pull the logs around it. Two timelines line up on one integer, with no guessing from timestamps.

The format is one JSON object per line, for the same reason the wire format is: the receiver may be a browser, and a person can still read it. `runtime::diagnostics::render` is the one place a line becomes text. A served city sends its lines to **the log** lens of **the record**, and to the terminal only where the terminal is not the console: a city served with no terminal writes them to stderr, while the CLI and the quiet host keep the terminal for the conversation and for the address and pairing code, and print no log line. A line at `effect` names what was done and where, such as a command's name or a file's path, and never carries the text of a message or a reply: those are in the Ledger, and the terminal is not a place to keep them ([`operating.md`](operating.md), *What the terminal shows*).

## 5 Secrets and logs

Logs pass through the **same** secret scan as the Ledger — not a second scanner for logs, but the same function. Two scanners would be two authorities, and the one that misses something is always the one nobody watches. A hit is replaced in place rather than dropping the line, because the words around it are usually what the reader needed.

`Sealed<T>` has neither `Debug` nor `Display`, so a sealed value **cannot enter a log at the type level**. That is the first defence; the scan is the second.

## 6 When to write a log line

**Default to none.** A log follows the same rule as a comment: code that needs a log line to be understood is usually structured wrongly.

Three cases earn one:

1. **The inputs and result of a decision** (`decide`) — a verdict is an exhaustive enum, so one line locates the branch.
2. **Boundaries across processes and machines** (`wire`, `effect`) — there is no stack to read over there.
3. **Retries and backoff** (`trace`) — their correctness is a property of timing, which cannot be reconstructed from the final state.

## 7 Relationship to `tracing`

The surface writes JSON lines itself and takes no logging dependency. What `tracing` would add here is spans that carry the module name across `await` points, and the correlation a reader needs already travels on `run` and `seq`. A dependency whose only feature has no consumer is a cost with no payment, so it is not taken.

When a code path needs spans carried across `await` points, this decision is worth revisiting. The sink is a closure, so the change is at the assembly layer rather than in the surface.

File rotation and remote reporting stay out regardless: logs remain on this machine, and rotation belongs to the operating system. This is the same principle as depending on no hosted service.

## 8 What holds this in place

Three checks, because a design like this decays quietly otherwise.

Two compile-failure counterexamples prove a `Sealed` value cannot be formatted into a line or a record at all (`crates/runtime/tests/ui/log_a_credential.rs`, `crates/kernel/tests/ui/sealed_into_record.rs`), and the scan redacts plaintext that arrives as an ordinary string — two defences, in that order.

The deletion-invariance test (`deleting_every_log_line_leaves_the_history_byte_identical`, in `crates/accounting/src/worker/genesis/tests.rs`) runs the same work twice, once with every level on and once with logging off, and requires the two Ledgers to agree. It also checks that the noisy run was actually noisy: an invariance that held because nothing was written would prove nothing.

**A line carries `seq` and no clock reading**, because the library that writes it is not allowed to sample time. A sink that wants a wall clock adds one at the assembly layer, which is where sampling is sanctioned.
