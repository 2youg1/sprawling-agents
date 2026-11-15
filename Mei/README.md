# Archetype:Mei

[English](README.md) · [简体中文](README.zh-CN.md)

Archetype:Mei is a minimal Agent Harness: a model receives a task, acts through one Python 3 tool, receives a **path handle** to each effect, and repeats until it has a final answer. This package is the **headless** core. Mei does not contain a terminal shell.

The interactive face is [Calyx](../Calyx/). `@sprawling/mei` ships both the headless `mei` binary and the `calyx` face; a TTY without `-p` becomes Calyx, while `-p` or a pipe stays headless. Scripts use the headless path alone.

> **Safety first:** Mei has no sandbox, permission prompts, or undo. Generated Python can read and write files, use the network, and run commands with the current account's authority. Use only disposable project copies and a separate, revocable API key with a spending limit. Never supply private data, important credentials, or the only copy of a project.

## Install and run

| Goal | Command |
|---|---|
| Script / one-shot | `bunx @sprawling/mei -p '…'` or `cargo install sprawling-mei --locked` then `mei -p '…'` |
| Interactive (`/new`, `/resume`, Esc) | `bunx @sprawling/mei` in the workspace, or `cargo install sprawling-mei sprawling-calyx --locked` then `calyx` (sibling `mei` or `MEI_BIN`) |

`@sprawling/mei` ships the headless core and Calyx face for Windows x64, macOS arm64, and Linux x64 musl. Mei never embeds a shell in-process; Calyx also serves the city console inside `sprawling`.

## Black-box path (keyboard only)

Needs internet, a tool-calling model API key, and a disposable folder. No clone of this repository.

### A. Script smoke

1. Install [bun](https://bun.sh) if needed.
2. Magpie in one terminal:

```sh
curl -fsSL https://usemagpie.ai/install.sh | sh -s -- --tui
magpie provider add deepseek
magpie serve
```

3. Second terminal:

```sh
export HARNESS_MODEL='deepseek/deepseek-flash'
mkdir -p ~/mei-playground && cd ~/mei-playground
bunx @sprawling/mei -p 'Use Python to calculate 40 + 2, then answer with only the number.'
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

First prompt:

```text
Read the project guide and directory layout without changing files. Tell me how to build and test it, then suggest the smallest useful first task.
```

Verify with `git status` and the project's own tests.

From source without npm: `cargo install sprawling-mei sprawling-calyx --locked`, then `calyx` with `mei` on PATH or `MEI_BIN` set. Building `sprawling` from crates.io on Windows still needs Zig **0.17.0** for Calyx's frame leaf.

### C. Automated stranger-entry E2E (no API key)

```sh
cargo test -p sprawling-mei --test cli e2e_beginner_workflow_from_first_prompt_to_project_change
cargo test -p sprawling-calyx --lib console::mei
cargo test -p sprawling-calyx --lib console::ui::mei
```

The first drives real `mei` + Python against a local scripted gateway (calc → project edit → tests). It does not install npm or drive the TUI.

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
| tool_result | Path to captured stdout+stderr |

| Variable | Meaning |
|---|---|
| `HARNESS_MODEL` | Required magpie `provider/model` |
| `ANTHROPIC_BASE_URL` | Default `http://127.0.0.1:3425`; Mei never listens |
| `ANTHROPIC_API_KEY` | Default `magpie` |
| `HARNESS_EXTRA` | One JSON object at startup; refuses `stream` / `tools` |
| `MEI_BIN` | Path Calyx uses to spawn this core; npm shim sets it to the sibling core |

## Boundaries

No in-process Python timeout; connect 10s / IO 600s; empty tool output is still a path; no `MEI_TOOLS`, no output truncation window, no runtime AGENTS/Skills injection, no in-core TUI.

## Spec and checks

[`Spec.lean`](Spec.lean), [`spec/Agent.lean`](spec/Agent.lean), [`spec/Session.lean`](spec/Session.lean); terminal [`Calyx/Spec.lean`](../Calyx/Spec.lean).

```sh
cargo nextest run -p sprawling-mei
cargo test -p sprawling-mei --test cli
cargo test -p sprawling-calyx --lib console::mei
cargo clippy -p sprawling-mei -p sprawling-calyx --all-targets -- -D warnings
lake build Mei.Spec Mei.spec.Agent Mei.spec.Session
```
