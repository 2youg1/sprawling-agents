# Archetype:Mei

[English](README.md) · [简体中文](README.zh-CN.md)

Archetype:Mei is a minimal Agent Harness: a model receives a task, acts through one persistent **python** tool, sees each small result, and repeats until it has a final answer. A large or non-UTF-8 result stays complete on disk and returns only a notice with its full path, or a Python expression that losslessly reconstructs a non-Unicode native path. This package is the **headless** core. Mei does not contain a terminal shell.

The interactive face is [Calyx](../Calyx/). `@sprawling/mei` ships both the headless `mei` binary and the `calyx` face; a TTY without `-p` becomes Calyx, while `-p` or a pipe stays headless. Scripts use the headless path alone. `mei --acp` speaks [Agent Client Protocol](https://agentclientprotocol.com/) version 1 as an agent over stdio so an ACP client (including the sprawling city harness) can call Mei like any other ACP agent; see [integrations](../docs/integrate/integrations.md).

> **Safety first:** Mei has no sandbox, permission prompts, or undo. Generated Python can read and write files, use the network, and run commands with the current account's authority. Use only disposable project copies and a separate, revocable API key with a spending limit. Never supply private data, important credentials, or the only copy of a project.

## Install

| Route | Command | What arrives |
|---|---|---|
| Bun | `bun install --global @sprawling/mei@latest` | The npm package carries the headless `mei` core and the Calyx face as `mei` and `calyx` commands. On a TTY, `mei` opens Calyx; `mei -p`, `mei --acp`, and pipes stay headless. `bunx @sprawling/mei` runs once without a global install. |
| crates.io | `cargo install sprawling-mei --locked` | The headless `mei` core only. A TTY `mei` without `-p` refuses with `E_MEI_USAGE` and points here: the Calyx face is not on this route. Use Bun or a source build for interactive use. |

After a Bun global install, open a **new** terminal and run `mei` or `calyx`. The package `bin` field is enough: Bun links both names into its global bin without needing a lifecycle script. If the new shell cannot see them, run `bun pm bin --global`, then (session only):

```sh
export PATH="$(bun pm bin --global):$PATH"
```

Add that directory to the file your login shell reads so later terminals keep it. When Bun does run the package postinstall, it may also place wrappers in the bin directory under `~/.local` and print the same **Fix your PATH** block; if lifecycle scripts were skipped, the global-bin PATH fix above is still the path. Reinstall with the same Bun command; it refreshes the shim and platform binaries together. Do not omit optional dependencies: the platform package holds both binaries.

The published packages trail this tree until the next release. Build from source for the version in this repository.

### Which command to type

| How you installed | Interactive | Headless / CI |
|---|---|---|
| Bun `@sprawling/mei` | `mei` on a TTY (npm shim opens Calyx); `calyx` also works | `mei -p …`, `mei --acp`, or a pipe |
| Source build / release zip | `calyx` only | `mei -p …`, `mei --acp`, or a pipe |
| crates.io `sprawling-mei` | No Calyx face; TTY `mei` without `-p` refuses with `E_MEI_USAGE` | `mei -p …` or a pipe |

Bun users only need to remember `mei`. The headless `mei` binary from source, a release zip, or crates.io never opens Calyx; run `calyx` from a build that includes `sprawling-calyx`.

### Update and uninstall

Mei has no self-updater. Install the version you want again through the same channel:

```sh
bun install --global @sprawling/mei@latest
# or
cargo install sprawling-mei --locked
```

To remove a Bun global install, uninstall the package and drop any leftover platform optional dependency for your OS:

```sh
bun remove --global @sprawling/mei
```

A crates.io install is removed with `cargo uninstall sprawling-mei`. For a source build, delete only the `mei` and `calyx` programs under `target/debug` (or `target/release`); leave the shared target tree. For a release zip, delete the unpacked `mei`/`calyx` files. None of these paths rewrite shell profiles.

## Build from source

### 1. Start Magpie

Install and open [Magpie](https://usemagpie.ai), add a provider, and configure the model routing you want. Keep it open on `127.0.0.1:3425`. Mei discovers the preferred route, context window, reasoning levels, actual model, and actual effort through the gateway. You do not need to copy a provider key or set `HARNESS_MODEL`.

### 2. Clone and build

Install the Rust toolchain and the Zig version pinned in [`Calyx/ffi/zig-version`](../Calyx/ffi/zig-version).

```sh
git clone https://github.com/2youg1/sprawling-agents.git
cd sprawling-agents
cargo build --locked -p sprawling-mei -p sprawling-calyx
```

The two programs are `target/debug/mei` and `target/debug/calyx` (`.exe` on Windows). Calyx finds the sibling Mei core automatically.

### 3. Open a workspace

Bash:

```sh
export PATH="$PWD/target/debug:$PATH"
mkdir -p /tmp/mei-playground
cd /tmp/mei-playground
calyx
```

Windows PowerShell:

```powershell
$env:PATH = "$PWD\target\debug;$env:PATH"
New-Item -ItemType Directory -Force "$HOME/mei-playground" | Out-Null
Set-Location "$HOME/mei-playground"
calyx
```

No model, gateway, or extra argument is required. Run `calyx` for the interactive face; `mei -p` and pipes stay headless. Non-interactive CI must use `mei -p` or a pipe so the process never waits on a Calyx face.

## Use one step at a time

### 1. Put the first task in `AGENTS.md`

For example, place `first.png` in the workspace and write:

```markdown
# Picture task

When the user says "start", call `python` exactly once and run
`show_image('first.png')`. Then describe the picture's background, shapes,
colours, and exact text. Do not use OCR, pixel statistics, or another model.
```

Mei injects the workspace root's regular `AGENTS.md` into the first message of each new session. Type only:

```text
start
```

The single Python call returns both the image and any printed text. PNG, JPEG, GIF, and WebP are accepted, up to 5 MiB per image before base64 encoding, at most two pictures per call, attached on that call's own thread; a third picture, or one attached from another thread or after the call ends, is refused with the reason in the tool result. This is visual input to the model, not a path or an image encoded as ordinary text.

### 2. Continue or switch sessions

| Keys | Effect |
|---|---|
| Text, Enter | Run one task |
| Shift+Enter | Insert a newline |
| `/new`, `/n` | Start a new JSONL session; the old file stays |
| `/resume`, `/r` | Resume the non-empty session you just left |
| `/resume path.jsonl`, `/r path.jsonl` | Resume a named complete log |
| `/e`, `/effect` | Choose a Magpie reasoning level for the next run |
| Esc | Clear a draft, or stop the current run and its process tree; never quits Calyx |
| `/quit`, `/q` | Close this Calyx and its process tree; other sessions are independent |

Model routing stays in Magpie; there is no separate `/model` command.

### 3. Read the status line

The composer shows:

```text
actual-model · effort high · Agent Step 2 · Input 487
```

Before Magpie confirms a route, the model and effort are marked `requested`. `Agent Step` counts model responses over the session. `Input` totals the session's input, cache creation and cache read tokens. With incomplete historic usage, `—` means unknown and `≥` shows the lower bound.

### 4. Steer mid-task and close safely

While a run is working, submitting ordinary text does not interrupt the current tool call: the words go to that run's `MEI_STEER_PATH`, and Mei appends them to the latest tool_result text after the wave finishes (prefixed `user: `). Esc still stops only that run and its descendants; `/q` closes that Calyx. Esc on an idle face does nothing.

## Workspace, runtime and captures

When a new session starts, Mei reads only the workspace root's regular `AGENTS.md`, without following symbolic links or opening non-regular entries, and places it before the actual prompt in the first user message. The log freezes that opening. Later tasks and `/resume` neither reread nor backfill AGENTS, even when the file appears, changes or disappears; `/new` starts a fresh reading. Skills, MCP configuration and environment tools are not automatically exposed.

On the first start of a Mei version, doctor probes Python and uv and caches the complete environment. A valid cache for that version skips all probes and network access. A supported Python is reused; a missing, broken or end-of-life Python is replaced by the latest stable managed installation. Missing or broken uv is installed. Python repair refreshes Mei's private uv first.

Installs stay in the per-user doctor directory and never replace system Python, modify the global PATH, edit shell profiles, or write the Windows Python registry. One complete uv+Python repair has a 900-second deadline. Doctor never updates Mei itself. `MEI_DOCTOR_DIR` overrides the per-user directory for isolation and tests.

Every run owns a new capture directory. Small UTF-8 captures are inlined and reclaimed; large and non-UTF-8 captures are spilled with a full-capture path or lossless native-path expression. Interactive spilled paths belong to the session under `<session>.captures/<run>` and survive resume. Failed or cancelled interactive runs delete only the directory Calyx exclusively created for that run. Forced host crashes can leave orphan directories; no crash janitor is claimed.

## Process Face

| Channel | Contract |
|---|---|
| cwd | Caller's working directory is the workspace |
| `-p` / stdin | Prompt; without `-p`, stdin must be a non-terminal complete prompt |
| `--replay` (Calyx) | Optional JSONL for the interactive face |
| `--resume <jsonl>` | Optional complete message log |
| stdout | Final answer, no added newline |
| stderr | Message JSONL; `mei.obs` model/route/pulse/usage; failures as Description/Cause/Resolution |
| cancel | Kill the process group |
| tool_result | Text and optional image blocks; text over 16 KiB or non-UTF-8 becomes a notice and full-capture path |

Resume preserves legacy tool names and string/array content, including image blocks. Python state is not restored. The provider and model must accept the preserved media.

| Variable | Meaning |
|---|---|
| `HARNESS_MODEL` | Optional explicit Magpie route; unset selects the catalog's preferred route |
| `ANTHROPIC_BASE_URL` | Default `http://127.0.0.1:3425`; Mei never listens |
| `ANTHROPIC_API_KEY` | Default `magpie` |
| `HARNESS_EXTRA` | One JSON object merged last; defaults request adaptive thinking and `high` effort; refuses `stream` / `tools` |
| `MEI_BIN` | Optional path Calyx uses to spawn the core; sibling and PATH resolution need no setting |
| `MEI_DOCTOR_DIR` | Optional isolated doctor cache and private runtime directory |
| `MEI_CAPTURE_DIR` | Internal per-run directory supplied by a face |
| `MEI_CAPTURE_CREATED` | Internal `1` marker proving Calyx created the empty capture directory |
| `MEI_STEER_PATH` | Face-supplied steer JSONL (one `{"text":"…"}` per line); absent means no steers |
| `SPRAWLING_PROGRAM_STATUS` | `never` disables OSC 7501 in the Calyx interactive face |

Calyx reports Program Status Protocol (OSC 7501) states: `working`, `done`, `error`, or `idle` after cancellation. Unsupported terminals ignore it. Headless `-p` and piped runs do not emit OSC 7501.

## Memory

One reading on Windows 11 x86-64 (16 cores), release binaries from this tree, using the Windows working set of each process. The session holds 6.8 MiB of message text: Mei stores a 6.8 MiB JSONL and Pi 1.5.2 loads the same text from a 7.1 MiB session. Pi ran with `--offline` and its default context files, skills and extensions. No model provider was contacted; Mei's one request went to a local stand-in. Calyx was read 30 seconds after `/resume`; Mei's peak was taken while one request carried the history.

| Process | Empty session | 6.8 MiB session |
|---|---:|---:|
| Calyx (idle) | 8.3 MiB | 33.8 MiB |
| Mei core (peak) | 5.8 MiB | 22.1 MiB |
| python guest (peak) | 13.5 MiB | 13.5 MiB |
| **Mei + Calyx + python** | **27.6 MiB** | **69.4 MiB** |
| Pi TUI (idle) | 120.8 MiB | 144.1 MiB |

The core and its python guest exist only while a task runs; Calyx alone is the idle cost. The large session adds about 26 MiB to Calyx and 16 MiB to the core. These are working-set readings on one machine, not a cross-platform benchmark; a different counter, session or platform gives different numbers.

## Prompt and tool contract

The system prompt lives in [`src/MEI.md`](src/MEI.md). The single `python` tool occupies `tools[0]`. The [Agent SPEC](spec/Agent.lean) defines its schema and call validation. Doctor supplies the detected Python and uv version attributes.

## Boundaries

No in-process python timeout; connect 10s / IO 600s; empty output is empty text; output over 16 KiB or non-UTF-8 returns only a notice and path or lossless path expression, never a head/tail sample. No `MEI_TOOLS`, automatic Skills/MCP exposure, Mei self-update, in-core TUI, OCR fallback, or second image tool. Only a new session's regular top-level AGENTS is injected.

## Spec and checks

[`Spec.lean`](Spec.lean), [`spec/Agent.lean`](spec/Agent.lean), [`spec/Session.lean`](spec/Session.lean), [`spec/Environment.lean`](spec/Environment.lean), [`spec/Capture.lean`](spec/Capture.lean), [`spec/Gateway.lean`](spec/Gateway.lean); terminal [`Calyx/Spec.lean`](../Calyx/Spec.lean).

The offline checks drive real `mei` and Python against local gateways and call no model API:

```sh
cargo nextest run -p sprawling-mei
cd Calyx && zig build test
cd .. && lake build Mei.Spec Mei.spec.Agent Mei.spec.Session Mei.spec.Environment Mei.spec.Capture Mei.spec.Gateway
cargo clippy -p sprawling-mei -p sprawling-calyx --all-targets -- -D warnings
```

A live Magpie run is opt-in and spends model tokens. Run the `AGENTS.md` picture task, then `/new` and `/resume` with a second image, and check Esc and `/q` in two independent Calyx windows. If a step fails, keep the command, session JSONL and gateway response before changing the product.
