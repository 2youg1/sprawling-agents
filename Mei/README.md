# Archetype:Mei

Archetype:Mei is an academic prototype for exploring how small an Agent Harness can be: one mutable Anthropic Messages request, one serial feedback loop, and one persistent Python 3 interpreter. It is intended primarily for secondary development and technical discussion, not as a ready-made product.

Expect to integrate and debug the surrounding pieces yourself, or to modify Mei before using it. A Provider gateway such as magpie, shell tools such as git, and an external isolation boundary are deliberately not built in. Mei inherits the caller's filesystem, network, environment, and credentials; run it only in a disposable container, VM, bwrap, or dedicated account.

## Shape

- One state object: the Messages request JSON, including every message.
- One loop: post the request, execute valid Python calls in content order, append results, and stop on `end_turn`.
- One local tool: `python`, a persistent interpreter whose namespace, imports, working directory, and environment changes survive calls.
- One protocol: non-streaming Anthropic Messages over HTTP/1.0.
- No session recovery, sandbox, permissions UI, cost accounting, retries, compaction, provider abstraction, or dedicated file/shell tools.

The system prompt lives in [`src/MEI.md`](src/MEI.md) and is injected at build time. Edit it to change the harness's starting instructions. The source is the supported extension point for academic experiments; there is no second configuration framework.

## Build and run

Mei targets POSIX systems. It uses `python3`, `SIGALRM`, and Unix file-descriptor semantics; native Windows is not supported.

```sh
cargo build -p sprawling-mei
HARNESS_MODEL=provider/model target/debug/mei -p "Explain what this directory contains"
printf 'Explain what this directory contains' | HARNESS_MODEL=provider/model target/debug/mei
```

Environment:

| Variable | Meaning |
|---|---|
| `HARNESS_MODEL` | Required magpie `provider/model` spelling. |
| `ANTHROPIC_BASE_URL` | Outbound Anthropic gateway; default `http://127.0.0.1:3425`. Mei never listens on this address or on `2501`. |
| `ANTHROPIC_API_KEY` | Gateway key; default `magpie`. |
| `HARNESS_EXTRA` | One JSON object, shallow-merged once at startup. `stream: true` and `tools` are refused. |
| `MEI_TOOLS` | Optional JSON array appended after the local Python declaration, for transparent server-side tools. Set `max_uses` where the Provider supports it. |
| `HARNESS` | Recursive executable path. Mei fills it with `current_exe()` only when it is not already set, so an external wrapper can enforce its own recursion policy. |

The default `3425` destination is magpie's listening socket, not a socket Mei opens. Multiple clients can connect to that gateway simultaneously.

Successful stdout contains only the final text, with no added newline. Stderr contains one JSON line for each assistant/tool-result message appended to history; fatal errors are plain text. Save the observation stream with `2>run.jsonl` if useful; that file is an external debugging artifact, not a resumable session.

## Sprawling console

The existing terminal UI has a separate Mei backend:

```sh
sprawling mei
sprawling mei --replay run.jsonl
```

`MEI_BIN` selects the executable, defaulting to `mei` on `PATH`. Replay and live processes share one message converter. The console supplies display-local run ids and timestamps, shows Python waves without inventing timings, maps plain errors to notes, and reports working/done/error state through OSC 7501. Esc terminates the whole process group on Unix.

## Boundaries that matter

- A 250-second alarm is best-effort. Each call reinstalls the handler and clears the alarm afterward, but executed code can change it or ignore the exception, and C calls can outlast it. The 250 seconds are meant to outlast network hiccups (a stalled request while a cable is reconnected, for example); use an outer process manager for a hard deadline.
- The transport gives each gateway address a 10 s connect budget and a generous 600 s read/write budget. A gateway that accepts a connection and then stalls returns an `E_MEI_TRANSPORT` diagnostic instead of hanging forever; the request is not retried.
- On success the interpreter gets a 5 s grace to exit after its stdin closes. Model code that leaves non-daemon threads behind would otherwise block Python's exit forever, so after the grace Mei kills and reaps the interpreter; that forced cleanup is ordinary shutdown, not a run failure.
- Output is captured in a binary temporary file and decoded incrementally. Only the decoding block and the 8000-character head/tail windows are held in memory. Output over 16000 characters is truncated and its full path is returned; other capture files are deleted.
- A whole tool wave is validated before the first side effect. Duplicate ids, unknown client tools, malformed calls, and an empty wave are protocol errors.
- A dead interpreter or private channel ends the run. Earlier effects are not rolled back, and no complete result message is invented.
- Recursive children share the filesystem and external isolation boundary, not the parent's messages or Python namespace. A child stays within the parent's best-effort timeout. Only its stdout is used by default.
- Transparent server-side tools can return arbitrarily large blocks which bypass Python's output window and are retransmitted on every round. Bound their use in the tool declaration.

## Specification and checks

The crate contract is [`Spec.lean`](Spec.lean), with the serial feedback model in [`spec/Agent.lean`](spec/Agent.lean). The console adapter contract is `crates/sprawling/spec/Console.lean` §8-174.

```sh
cargo nextest run -p sprawling-mei
cargo clippy -p sprawling-mei --all-targets --all-features -- -D warnings
lake build Mei.Spec Mei.spec.Agent
```

Scripted closures exercise every control path without a model. TCP fixtures exercise the real transport, real Python processes exercise the interpreter protocol, and CLI fixtures check stdout, stderr, and exit status.

## Observed through magpie

Live V1/V2 passed with `deepseek/deepseek-flash` through local magpie. The observations below describe that path only, not compatibility with every Provider's dialect.

- A direct `curl --http1.0` Messages request returned `HTTP/1.0 200 OK` with `Content-Length`, no chunked transfer, `end_turn`, and the exact text `42`.
- With thinking enabled, a request to calculate `40 + 2` produced one Python `tool_use`, one matching `tool_result`, and a final answer. The appended trace had the shape assistant → user/tool-result → assistant; both assistant turns carried signed thinking blocks, and the Provider accepted the original thinking/signature on the following request. Successful stdout was exactly `42`, with no added newline.
- In a separate small task, the model chose Python to read a file containing a known marker. One tool call returned the file contents, and the final stdout was exactly that marker. This exercises the intended file-access path through general Python rather than a dedicated file tool.

The original observation streams remain local debugging artifacts. They are not resumable sessions and are not shipped as Provider-independent fixtures.
