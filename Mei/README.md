# Archetype:Mei

[English](README.md) · [简体中文](README.zh-CN.md)

Archetype:Mei is a minimal Agent Harness: a model receives a task, acts through one persistent **python** tool, sees each small result, and repeats until it has a final answer. A large or non-UTF-8 result stays complete on disk and returns only a notice with its full path, or a Python expression that losslessly reconstructs a non-Unicode native path. This package is the **headless** core. Mei does not contain a terminal shell.

The interactive face is [Calyx](../Calyx/). `@sprawling/mei` ships both the headless `mei` binary and the `calyx` face; a TTY without `-p` becomes Calyx, while `-p` or a pipe stays headless. Scripts use the headless path alone.

> **Safety first:** Mei has no sandbox, permission prompts, or undo. Generated Python can read and write files, use the network, and run commands with the current account's authority. Use only disposable project copies and a separate, revocable API key with a spending limit. Never supply private data, important credentials, or the only copy of a project.

## Install and run

| Goal | Command |
|---|---|
| Script / one-shot | `bunx @sprawling/mei -p '…'` or `cargo install sprawling-mei --locked` then `mei -p '…'` |
| Interactive (`/new`, `/resume`, Esc) | `bunx @sprawling/mei` in the workspace, or `cargo install sprawling-mei sprawling-calyx --locked` then `calyx` (sibling `mei` or `MEI_BIN`) |

`@sprawling/mei` ships the headless core and Calyx face for Windows x64, macOS arm64, and Linux x64 musl. Mei never embeds a shell in-process; Calyx also serves the city console inside `sprawling`.

## Black-box path (keyboard only)

Needs internet, [Bun](https://bun.sh), Magpie, a tool-calling model API key, and a disposable folder. Python 3 and uv may already be installed; Mei's doctor repairs them when required. No clone of this repository.

### A. Script smoke

1. Check `bun --version`.
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
bunx @sprawling/mei -p 'Use python to calculate 40 + 2, then answer with only the number.'
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
cargo test -p sprawling-mei --test workspace
cargo test -p sprawling-mei --test capture
cd Calyx && zig build test
```

These drive real `mei` + Python on Windows and Unix against a local gateway, including observed results, large-output recovery, workspace freezing and legacy/media resume. They use an isolated doctor cache and never install npm packages, Python, uv, or a TUI.

## Workspace, runtime and captures

When a new session starts, Mei reads only the workspace root's regular `AGENTS.md`, if present, without following symbolic links or opening non-regular entries, and places it before the actual prompt in the first user message. The log freezes that opening. Later tasks and `/resume` neither reread nor backfill AGENTS, even when the file appears, changes or disappears; `/new` starts a fresh reading. Skills, MCP configuration and environment tools are not automatically exposed.

On the first start of a Mei version, doctor probes Python and uv and caches the complete environment. A valid cache for that version skips all probes and network access. A supported Python is reused; a missing, broken or end-of-life Python is replaced by the latest stable managed installation. Missing or broken uv is installed. Python repair refreshes Mei's private uv first, so an old uv release does not define which Python is newest.

Installs stay in the per-user doctor directory and never replace system Python, modify the global PATH, edit shell profiles, or write the Windows Python registry. The selected absolute paths stay local; the tool description carries only `version` and `uv_version` attributes. uv is available on PATH for subprocesses started by the python tool. One complete uv+Python repair has a 900-second deadline, while a concurrent doctor waits an additional 5 seconds for cache publication. Repaired versions are checked again before caching. Doctor never updates Mei itself. `MEI_DOCTOR_DIR` overrides the per-user directory for isolation and tests.

Every run owns a new capture directory. Small UTF-8 captures are inlined and their files are reclaimed after fd transfer or interpreter shutdown; after a successful interpreter close, an inline cleanup failure is reported separately without discarding the answer or spilled files. Large and non-UTF-8 captures are spilled: interactive paths belong to the session under `<session>.captures/<run>` and survive resume; headless paths belong to the caller and survive process exit. Failed or cancelled interactive runs delete only the directory Calyx exclusively created for that run. Mei refuses to reuse a pre-existing capture directory unless the caller marks its newly created empty directory with `MEI_CAPTURE_CREATED=1`. Forced host crashes can leave orphan directories; no crash janitor is claimed.

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
| tool_result | Complete stdout+stderr up to 16 KiB; otherwise a notice and full-capture path or lossless native-path Python expression |

Resume preserves legacy tool names and string/array content, including image, audio and video blocks. The provider and model must accept those blocks; Mei has no attachment flags and does not restore Python state.

| Variable | Meaning |
|---|---|
| `HARNESS_MODEL` | Required magpie `provider/model` |
| `ANTHROPIC_BASE_URL` | Default `http://127.0.0.1:3425`; Mei never listens |
| `ANTHROPIC_API_KEY` | Default `magpie` |
| `HARNESS_EXTRA` | One JSON object at startup; refuses `stream` / `tools` |
| `MEI_BIN` | Path Calyx uses to spawn this core; npm shim sets it to the sibling core |
| `MEI_DOCTOR_DIR` | Optional isolated doctor cache and private runtime directory |
| `MEI_CAPTURE_DIR` | Internal per-run directory supplied by a face; it must not already exist unless exclusively created by Calyx |
| `MEI_CAPTURE_CREATED` | Internal `1` marker proving Calyx created the empty capture directory for this run |
| `SPRAWLING_PROGRAM_STATUS` | `never` disables OSC 7501 in the Calyx interactive face |

## Prompt and tool contract

`Mei/src/MEI.md` is compiled into `system`; the sole python definition occupies `tools[0]`. The table below uses a fixed documentation fixture with Python `3.12.13` and uv `0.12.3`; actual version attributes come from doctor. Byte counts exclude message envelopes and anything a provider adds itself.

| Part | UTF-8 bytes |
|---|---:|
| `system` | 297 |
| Complete compact `tools[0]` JSON | 1192 |
| Total | 1489 |

The JSON below is pretty-printed for reading; the byte count uses its compact wire form.

### `system`

```text
<agent-harness>
You are operating inside Mei.
No additional documents or environment capabilities are injected or exposed; fetch what you need.
Workflow: Sense → Act → Verify → Adapt…
Aligned with context, fulfill user intent thoroughly and resourcefully and stay scoped.
</agent-harness>
```

### `tools[0]`

```json
{
  "description": "<python version=\"3.12.13\" uv_version=\"0.12.3\">\nA persistent python interpreter. Variables, imports, and the working directory (cwd) persist across calls. An exception prints its traceback without ending the session, so you can recover and continue without rebuilding state. Expressions do not auto-echo, so explicitly print any values you need to inspect. If combined UTF-8 stdout/stderr is up to 16 KiB, the tool returns it inline. Larger or non-UTF-8 output returns a notice and the full path, or a Python expression that losslessly reconstructs a non-Unicode native path; filter that file, or re-run with narrower output, in the next python call. Run shell commands and CLIs through subprocess, capture their output, and check return codes. Start child interpreters with sys.executable, so they inherit the active environment. Because capabilities like web_search, web_fetch or sub-agent delegation are not exposed as built-in tools, python serves as your interface to invoke external APIs, CLIs, headless tools, and more directly via subprocess.\n</python>",
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
  "name": "python"
}
```

Mei keeps one provider-neutral schema and validates every tool wave in the agent loop; it does not declare provider-specific `strict` modes.

## Boundaries

No in-process python timeout; connect 10s / IO 600s; empty output is empty text; output over 16 KiB or non-UTF-8 returns only a notice and path or lossless path expression, never a head/tail sample; no `MEI_TOOLS`, automatic Skills/MCP exposure, Mei self-update, or in-core TUI. Only a new session's regular top-level AGENTS is injected.

## Spec and checks

[`Spec.lean`](Spec.lean), [`spec/Agent.lean`](spec/Agent.lean), [`spec/Session.lean`](spec/Session.lean), [`spec/Environment.lean`](spec/Environment.lean), [`spec/Capture.lean`](spec/Capture.lean); terminal [`Calyx/Spec.lean`](../Calyx/Spec.lean).

```sh
cargo nextest run -p sprawling-mei
cargo test -p sprawling-mei --test cli
cargo test -p sprawling-mei --test observation
cargo test -p sprawling-mei --test workspace
cargo test -p sprawling-mei --test capture
cargo test -p sprawling-calyx --lib console::mei
cargo clippy -p sprawling-mei -p sprawling-calyx --all-targets -- -D warnings
lake build Mei.Spec Mei.spec.Agent Mei.spec.Session Mei.spec.Environment Mei.spec.Capture
```
