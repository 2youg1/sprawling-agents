# Archetype:Mei

[English](README.md) · [简体中文](README.zh-CN.md)

Archetype:Mei is a minimal Agent Harness: a model receives a task, acts through one persistent **Python3** tool, sees each small result, and repeats until it has a final answer. A large or non-UTF-8 result stays complete on disk and returns only a notice with its path. This package is the **headless** core. Mei does not contain a terminal shell.

The interactive face is [Calyx](../Calyx/). `@sprawling/mei` ships both the headless `mei` binary and the `calyx` face; a TTY without `-p` becomes Calyx, while `-p` or a pipe stays headless. Scripts use the headless path alone.

> **Safety first:** Mei has no sandbox, permission prompts, or undo. Generated Python can read and write files, use the network, and run commands with the current account's authority. Use only disposable project copies and a separate, revocable API key with a spending limit. Never supply private data, important credentials, or the only copy of a project.

## Install and run

| Goal | Command |
|---|---|
| Script / one-shot | `bunx @sprawling/mei -p '…'` or `cargo install sprawling-mei --locked` then `mei -p '…'` |
| Interactive (`/new`, `/resume`, Esc) | `bunx @sprawling/mei` in the workspace, or `cargo install sprawling-mei sprawling-calyx --locked` then `calyx` (sibling `mei` or `MEI_BIN`) |

`@sprawling/mei` ships the headless core and Calyx face for Windows x64, macOS arm64, and Linux x64 musl. Mei never embeds a shell in-process; Calyx also serves the city console inside `sprawling`.

## Black-box path (keyboard only)

Needs internet, [Bun](https://bun.sh), Python 3 on PATH, Magpie, a tool-calling model API key, and a disposable folder. No clone of this repository.

### A. Script smoke

1. Check `bun --version` and `python --version` on Windows, or `python3 --version` elsewhere.
2. Install and open [Magpie](https://usemagpie.ai). Add the provider under **Providers → Add provider**, then keep the app open; it already serves the gateway. `magpie serve` is the terminal-only alternative.
3. Choose an entry from `magpie models`, then open a disposable workspace.

Bash:

```sh
export HARNESS_MODEL='deepseek/deepseek-flash'
mkdir -p ~/mei-playground && cd ~/mei-playground
```

Windows PowerShell:

```powershell
$env:HARNESS_MODEL='deepseek/deepseek-flash'
New-Item -ItemType Directory -Force "$HOME/mei-playground" | Out-Null
Set-Location "$HOME/mei-playground"
```

Run:

```sh
bunx @sprawling/mei -p 'Use Python3 to calculate 40 + 2, then answer with only the number.'
```

Stdout is `42`. Failures are a `toml` block with `description` / `cause` / `resolution`.

### B. First workspace (interactive)

1. Magpie as in A; set `HARNESS_MODEL`.
2. Copy a disposable project into a new directory.
3. In that directory:

```sh
bunx @sprawling/mei
```

| Keys | Effect |
|---|---|
| Text, Enter | Run a task; first success creates `mei-….jsonl` in cwd |
| `/new` | New JSONL; old files stay |
| `/resume <path.jsonl>` | Continue that complete log |
| Shift+Enter | Newline |
| Esc | Clear draft → stop run → quit |

Banner shows cwd; composer shows `HARNESS_MODEL · usage` (or `unknown` until a response carries usage). A live run shows Mei working; pulse lines are silent.

The Calyx face reports Program Status Protocol (OSC 7501) states: `working`, `done`, `error`, or `idle` after cancellation. A terminal or agent dashboard that understands the protocol can therefore observe a Mei child spawned by another agent without reading its conversation; reports carry counts, not prompt text. Mei sends no support probe, unsupported terminals ignore the sequence, and Calyx clears it before handing back the terminal. Set `SPRAWLING_PROGRAM_STATUS=never` to disable reporting. Headless `-p` and piped runs do not emit OSC 7501.

First prompt:

```text
Read the project guide and directory layout without changing files. Tell me how to build and test it, then suggest the smallest useful first task.
```

Verify with `git status` and the project's own tests.

From source without npm: `cargo install sprawling-mei sprawling-calyx --locked`, then `calyx` with `mei` on PATH or `MEI_BIN` set. Building `sprawling` from crates.io on Windows still needs Zig **0.17.0** for Calyx's frame leaf.

### C. Automated stranger-entry E2E (no API key)

```sh
cargo test -p sprawling-mei --test cli e2e_beginner_workflow_from_first_prompt_to_project_change
cargo test -p sprawling-mei --test observation
cd Calyx && zig build test
```

The first drives real `mei` + Python3 on Windows and Unix against a local gateway that answers correctly only when the next request carries the observed result. The second also covers a large capture staying on disk before a selective read, plus legacy and media-block resume passthrough. Neither installs npm packages or drives the TUI.

## Process Face

| Channel | Contract |
|---|---|
| cwd | Caller's working directory is the workspace |
| `-p` / stdin | Prompt; without `-p`, stdin must be a non-terminal complete prompt |
| `--replay` (Calyx) | Optional JSONL for the interactive face |
| `--resume <jsonl>` | Optional complete message log |
| stdout | Final answer, no added newline |
| stderr | Message JSONL; `mei.obs` pulse/usage; failures as Description/Cause/Resolution |
| cancel | Kill the process group |
| tool_result | Complete stdout+stderr up to 16 KiB; otherwise a notice and full-capture path |

Resume preserves legacy tool names and string/array content, including image, audio and video blocks. The provider and model must accept those blocks; Mei has no attachment flags and does not restore Python state.

| Variable | Meaning |
|---|---|
| `HARNESS_MODEL` | Required magpie `provider/model` |
| `ANTHROPIC_BASE_URL` | Default `http://127.0.0.1:3425`; Mei never listens |
| `ANTHROPIC_API_KEY` | Default `magpie` |
| `HARNESS_EXTRA` | One JSON object at startup; refuses `stream` / `tools` |
| `MEI_BIN` | Path Calyx uses to spawn this core; npm shim sets it to the sibling core |
| `SPRAWLING_PROGRAM_STATUS` | `never` disables OSC 7501 in the Calyx interactive face |

## System prompt and static overhead

`Mei/src/MEI.md` is compiled into `system`; the sole Python3 definition occupies `tools[0]`. With zero bytes of user input, these two values are the fixed content beyond that input. The table excludes message envelopes and anything a provider adds itself.

| Part | UTF-8 bytes | Estimated tokens |
|---|---:|---:|
| `system` | 257 | 63 |
| Complete compact `tools[0]` JSON | 1131 | 246 |
| Total | 1388 | 309 |

Token estimates use `@google/genai` **2.21.0** `LocalTokenizer` with `gemini-2.0-flash-001` (gemma3), counting the two values separately and adding the results. They describe this tokenizer only, not a provider's usage or billing. The JSON below is pretty-printed for reading; the byte count uses its compact wire form.

### `system`

```text
<agent-harness>
You are operating inside Mei.
No files are injected or pre-loaded; fetch what you need.
Workflow: Sense → Act → Verify → Adapt…
Aligned with context, fulfill user intent thoroughly and resourcefully and stay scoped.
</agent-harness>
```

### `tools[0]`

```json
{
  "description": "<python3>\nA persistent Python3 interpreter. Variables, imports, and the working directory (cwd) persist across calls. An exception prints its traceback without ending the session, so you can recover and continue without rebuilding state. Expressions do not auto-echo, so explicitly print any values you need to inspect. If combined UTF-8 stdout/stderr is up to 16 KiB, the tool returns it inline. Larger or non-UTF-8 output returns a notice and the path to a file containing the full output; filter that file, or re-run with narrower output, in the next Python call. Run shell commands and CLIs through subprocess, capture their output, and check return codes. Start Python with sys.executable rather than bare python3, so child processes inherit the active environment. Because capabilities like web_search, web_fetch or sub-agent delegation are not exposed as built-in tools, Python serves as your interface to invoke external APIs, CLIs, headless tools, and more directly via subprocess.\n</python3>",
  "input_schema": {
    "properties": {
      "code": {
        "type": "string"
      }
    },
    "required": [
      "code"
    ],
    "type": "object"
  },
  "name": "Python3"
}
```

Mei keeps one provider-neutral schema and validates every tool wave in the agent loop; it does not declare provider-specific `strict` modes.

## Boundaries

No in-process Python3 timeout; connect 10s / IO 600s; empty output is empty text; output over 16 KiB or non-UTF-8 returns only a notice and path, never a head/tail sample; no `MEI_TOOLS`, runtime AGENTS/Skills injection, or in-core TUI.

## Spec and checks

[`Spec.lean`](Spec.lean), [`spec/Agent.lean`](spec/Agent.lean), [`spec/Session.lean`](spec/Session.lean); terminal [`Calyx/Spec.lean`](../Calyx/Spec.lean).

```sh
cargo nextest run -p sprawling-mei
cargo test -p sprawling-mei --test cli
cargo test -p sprawling-mei --test observation
cargo test -p sprawling-calyx --lib console::mei
cargo clippy -p sprawling-mei -p sprawling-calyx --all-targets -- -D warnings
lake build Mei.Spec Mei.spec.Agent Mei.spec.Session
```
