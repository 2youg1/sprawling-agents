# Archetype:Mei

[English](README.md) · [简体中文](README.zh-CN.md)

Mei is an academic prototype asking how small an Agent Harness can be: a model receives a task, acts through one Python 3 tool, receives the result, and repeats until it has a final answer. It is for learning, experiments, and secondary development, not a ready-made product.

> **Safety first:** Mei has no sandbox, permission prompts, or undo button. Generated Python can read and write files, use the network, and run commands with the current account's authority. Use only disposable project copies and a separate, revocable API key with a spending limit. Never supply private data, important credentials, or the only copy of a project.

## From zero on a new Windows laptop

This path assumes Windows 10/11, an internet connection, no repository, no Docker, no Linux, and no development tools. No prior Agent or programming experience is needed. You need administrator access and an API key for a model that supports tool calls; model use may cost money.

### The few words that matter

| Term | Meaning here |
|---|---|
| Agent | A model that can act through tools, not merely chat |
| Terminal | A text window for commands; PowerShell and Ubuntu are different terminals |
| WSL / Ubuntu | WSL runs Linux on Windows; Ubuntu is the Linux system installed below |
| Python | The only local tool Mei gives the model |
| Git | Downloads source and inspects project changes |
| Rust / Cargo | Mei's implementation language and build tool |
| Zig | Builds the terminal renderer used by `sprawling mei`; version 0.17.0 is required |
| magpie | A local gateway that connects Mei to different model providers |
| API key | A password-like credential issued by your model provider; never publish it |

WSL reduces accidental Windows file access but **is not a hard security boundary**: it can still reach Windows files and the network. Do not inspect private files under `/mnt/c`. For stronger isolation, use a disposable VM, container, bwrap, or dedicated account.

### 1. Install and open Ubuntu

Search **PowerShell** in the Windows Start menu, right-click it, choose **Run as administrator**, and run:

```powershell
wsl --install -d Ubuntu
```

Restart when asked. Then open **Ubuntu** from the Start menu. Its first launch asks for a Linux username and password. Nothing appears while typing the password; that is normal.

Open Ubuntu from the Start menu next time as well. **All commands below run in the Ubuntu terminal unless explicitly marked PowerShell.** Microsoft's [official WSL guide](https://learn.microsoft.com/windows/wsl/install) covers installation failures.

### 2. Install every dependency

In Ubuntu, run:

```sh
sudo apt update
sudo apt install -y ca-certificates curl git python3 build-essential xz-utils
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
curl -fsSL https://usemagpie.ai/install.sh | sh -s -- --tui
export PATH="$HOME/.local/bin:$PATH"
```

`sudo` may ask for the Ubuntu password. These steps install download and build tools, Git, Python, Rust, and magpie. Docker, Node.js, and a database are not needed.

Install the exact Zig version used by the terminal renderer. This block supports ordinary x86-64 and ARM64 laptops, verifies the official SHA-256, and prints `0.17.0` at the end:

```sh
case "$(uname -m)" in
  x86_64) ZIG_ARCH=x86_64; ZIG_SHA256=1cbe9df9f27e6b78d14ccbca43b6703a404ef79ef1c463de901d7f088d4e2026 ;;
  aarch64) ZIG_ARCH=aarch64; ZIG_SHA256=9e8d11661d4ae3bd57702a3832781e23ad151dde5798e16a5ccd503f65234ff8 ;;
  *) echo "unsupported CPU: $(uname -m)"; exit 1 ;;
esac
cd /tmp
curl -fSLO "https://ziglang.org/download/0.17.0/zig-$ZIG_ARCH-linux-0.17.0.tar.xz"
echo "$ZIG_SHA256  zig-$ZIG_ARCH-linux-0.17.0.tar.xz" | sha256sum -c -
sudo tar -C /opt -xf "zig-$ZIG_ARCH-linux-0.17.0.tar.xz"
sudo ln -sf "/opt/zig-$ZIG_ARCH-linux-0.17.0/zig" /usr/local/bin/zig
cd ~
zig version
```

Check the remaining tools:

```sh
git --version
python3 --version
cargo --version
magpie help
```

The first three print versions and the last prints magpie's commands.

### 3. Configure a model

Create an API key on your provider's website and give it a quota or spending limit. DeepSeek is the example below; `magpie presets` lists other providers.

```sh
read -rsp 'Paste the DeepSeek API key, then press Enter: ' DEEPSEEK_KEY; echo
magpie provider add deepseek "$DEEPSEEK_KEY"
unset DEEPSEEK_KEY
magpie models
```

The key remains invisible while pasted. The last command lists models as `provider/model`. Choose one that supports tool calls; `deepseek/deepseek-flash` works for the examples if it appears in your list.

Keep this window open and start the gateway:

```sh
magpie serve
```

Open **a second Ubuntu window** from the Start menu. All Mei commands below run there. Mei defaults to `http://127.0.0.1:3425` in the same Ubuntu environment, so no gateway address or gateway key needs configuring.

### 4. Download, build, and open Mei

In the second Ubuntu window:

```sh
source "$HOME/.cargo/env"
export PATH="$HOME/.local/bin:$PATH"
cd ~
git clone https://github.com/2youg1/sprawling-agents.git
cd sprawling-agents
cargo build -p sprawling-mei -p sprawling
```

Replace the model below with an actual `magpie models` entry:

```sh
export HARNESS_MODEL='deepseek/deepseek-flash'
mkdir -p ~/mei-playground
cd ~/mei-playground
MEI_BIN="$HOME/sprawling-agents/target/debug/mei" "$HOME/sprawling-agents/target/debug/sprawling" mei
```

`HARNESS_MODEL` selects the model and is forgotten when this window closes. The last command opens Mei's interactive terminal. The first task creates a `mei-….jsonl` file in the current directory and prints its full path.

### 5. Have the first simple conversation

At the Mei input line, type this and press Enter:

```text
Use Python to calculate 40 + 2, then answer with only the number.
```

The answer should be `42`. Mei waits for each complete non-streaming response, so a quiet screen does not necessarily mean it is stuck. The terminal shows model replies and Python results as they arrive.

The current JSONL contains the initial prompt and every successful message in this conversation. Later tasks in this console continue that history. Each task starts a fresh Python process; Python variables do not survive across tasks.

### 6. Features and commands

**Features:**

- One local `python` tool provides file, shell, network, and project-command access.
- Python variables, imports, working-directory changes, and environment changes persist within one task.
- Model and tool calls are serial. Final stdout contains only the final answer; observation messages can be saved as JSONL.
- Edit [`src/MEI.md`](src/MEI.md) to change the build-time system prompt, or use `MEI_TOOLS` to add provider-executed tools.
- There is no permission UI, spending cap, automatic retry, context compaction, or Python process recovery.

**Inside `sprawling mei`:**

| Input | What it does |
|---|---|
| Ordinary text, then Enter | Runs a task in the current JSONL conversation |
| `/new` | Creates a new JSONL and starts with empty history; old files stay untouched |
| `/resume <path.jsonl>` | Imports that complete JSONL and continues it; successful new segments append to it |
| Shift+Enter | Inserts a newline |
| Esc | Clears the draft, then stops the running task, then exits |

These are the only Mei slash commands. `/resume` needs a path and does not guess a recent file. Imported history must begin with the initial user prompt and end with an assistant `end_turn`; malformed or partial logs fail with `E_MEI_SESSION` before the next model request or Python process. Restoring messages never restores Python variables or process state.

For a read-only display of a log:

```sh
"$HOME/sprawling-agents/target/debug/sprawling" mei --replay run.jsonl
```

For scripts, the standalone binary remains one-shot and has no slash commands:

```sh
"$HOME/sprawling-agents/target/debug/mei" -p 'Use Python to calculate 40 + 2' 2>run.jsonl
printf 'Explain this directory' | "$HOME/sprawling-agents/target/debug/mei"
```

Its optional `--resume <jsonl>` flag is the process interface used by the console. Press Ctrl+C to stop a standalone task.

### 7. Start work on the first large project

Use a **disposable project copy**. Assuming the copy is on the Windows Desktop, replace the username and folder below:

```sh
mkdir -p ~/work
cp -r "/mnt/c/Users/WindowsUser/Desktop/ProjectCopy" ~/work/my-project
cd ~/work/my-project
export HARNESS_MODEL='deepseek/deepseek-flash'
MEI_BIN="$HOME/sprawling-agents/target/debug/mei" "$HOME/sprawling-agents/target/debug/sprawling" mei
```

`/mnt/c` is the Windows C: drive as seen from Ubuntu. Do not copy `.env` files, keys, or private material, and never operate on the only original.

First ask for a read-only survey:

```text
Read the project guide and directory layout without changing files. Tell me how to build and test it, then suggest the smallest useful first task.
```

After checking that answer, give one bounded task at a time:

```text
Goal: … . Scope: modify only … . Done means: run … successfully. If blocked, report the blocker instead of widening the change. Finish with changed files, test commands, and their results.
```

The console carries history in the current JSONL, but Mei does not preload an entire large project or compact an old conversation. Use `/new` when unrelated history would distract the model, then restate the goal, scope, and done condition. Avoid “improve the whole project.”

Verify the result yourself: in a Git project, run `git status --short` and `git diff`; otherwise compare against the original copy. Run the project's tests yourself rather than treating “done” as test evidence. There is no hard turn or spending ceiling: press Esc to stop and inspect usage in magpie.

### Reopen and troubleshoot

After reopening Ubuntu, run `magpie serve` in one window. In another, repeat `source "$HOME/.cargo/env"`, `export PATH="$HOME/.local/bin:$PATH"`, and `export HARNESS_MODEL=…`, enter the project directory, and start `sprawling mei`. Use `/resume <path>` to continue a particular log.

| Symptom | Repair |
|---|---|
| `cargo` or `magpie` not found | Repeat the `source` and `export PATH` lines above |
| Zig refuses the build | `zig version` must print exactly `0.17.0` |
| `E_MEI_CONFIG` / `HARNESS_MODEL` | Set the variable to an entry printed by `magpie models` |
| `E_MEI_TRANSPORT` / connection failure | Keep `magpie serve` running in another window in the same Ubuntu environment |
| `python3` not found | Run `sudo apt install -y python3` |
| `E_MEI_SESSION` | Choose a complete JSONL, or use `/new`; a partial task cannot be resumed |
| Provider 401, 402, 429, or a tool error | Check the key, quota, model id, and tool-call support; try another model if needed |

## Technical reference

### Shape

- One state object: the Messages request JSON, including every message.
- One loop: post, execute valid Python calls in content order, append results, and stop on `end_turn`.
- One local tool: a persistent Python 3 interpreter within each task.
- One protocol: non-streaming Anthropic Messages over HTTP/1.0.
- No sandbox, permissions UI, cost accounting, retries, compaction, provider abstraction, dedicated file/shell tools, or Python process recovery.

### Environment and process interface

| Variable | Meaning |
|---|---|
| `HARNESS_MODEL` | Required magpie `provider/model` spelling |
| `ANTHROPIC_BASE_URL` | Outbound Anthropic gateway; default `http://127.0.0.1:3425`; Mei never listens here or on `2501` |
| `ANTHROPIC_API_KEY` | Gateway key; default `magpie` |
| `HARNESS_EXTRA` | One JSON object shallow-merged once at startup; `stream: true` and `tools` are refused |
| `MEI_TOOLS` | Optional JSON array appended after the local Python declaration; set `max_uses` where supported |
| `HARNESS` | Recursive executable path; Mei fills it with `current_exe()` only when unset |

The standalone grammar is `mei [-p PROMPT] [--resume JSONL]`; without `-p`, non-terminal stdin supplies the complete prompt. Recovery imports only role/content in file order. Current environment settings choose the model, system prompt, tools, and gateway again.

Successful stdout is final text with no added newline. A successful stderr segment starts with the new user prompt, followed by assistant/tool-result JSONL. Assistant observations add `stop_reason`, which recovery strips. Fatal errors remain plain text; a first transport failure does not write a prompt ahead of the raw provider error.

### Boundaries that matter

- The 250-second Python alarm is best-effort: code can alter or ignore it, and C calls can outlast it. Use an outer process manager for a hard deadline.
- Each gateway address has a 10-second connect budget and a 600-second read/write budget. A stall reports `E_MEI_TRANSPORT`; the request is not retried.
- Successful interpreter shutdown has a 5-second grace, then kill and reap. This ordinary cleanup is not a task failure.
- Output is decoded incrementally with 8000-character head/tail windows. Over 16000 characters, output is truncated and the full temporary path is returned.
- A whole tool wave is validated before its first side effect. Duplicate ids, unknown client tools, malformed calls, and empty waves are protocol errors.
- Interpreter or private-channel death ends the task. Earlier effects are not rolled back and no complete result is invented.
- Recursive children share the filesystem and external isolation boundary, not parent messages or Python namespace.
- Transparent server-side tools can return arbitrarily large blocks that bypass Python's output window and are retransmitted each round. Bound their use in the declaration.

### Specification and checks

The core contract is [`Spec.lean`](Spec.lean), with the serial feedback model in [`spec/Agent.lean`](spec/Agent.lean) and explicit JSONL recovery in [`spec/Session.lean`](spec/Session.lean). The console adapter contract is `crates/sprawling/spec/Console.lean` §8-174.

```sh
cargo nextest run -p sprawling-mei
cargo test -p sprawling-mei --test cli e2e_beginner_workflow_from_first_prompt_to_project_change
cargo test -p sprawling --lib console::mei
cargo test -p sprawling --lib console::ui::mei
cargo clippy -p sprawling-mei --all-targets --all-features -- -D warnings
lake build Mei.Spec Mei.spec.Agent Mei.spec.Session
```

The second command is the beginner-flow E2E: real `mei` processes, real Python, a temporary project, and a local scripted gateway cover the first calculation, project change, and project tests without a real provider or API key. Resume tests cover complete-history import and pre-side-effect rejection. Console tests cover distinct `/new` files, untouched old files, explicit `/resume`, and success-only append. POSIX behavior is verified in the Unix lane.

### Observed through magpie

Live V1/V2 passed only with `deepseek/deepseek-flash` through local magpie; this is not a claim about every provider dialect. A direct Messages request returned HTTP/1.0 and non-streaming `end_turn`; a calculation completed a Python round trip while preserving signed thinking; another task read a file marker through Python. Original observation streams are local debugging artifacts, not Python-state recovery or provider-independent fixtures.
