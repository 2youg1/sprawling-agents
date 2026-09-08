# sprawling-desktop

An MCP server that gives an agent eyes and hands on this Windows desktop: it lists windows, reads their accessibility tree, clicks and types, captures images, records, and reads the clipboard. A city attaches it the way it attaches any other server, and it does only what one file on this machine says it may do.

**This build carries none of it out.** The tool names, their descriptions and their input schemas are final; every call is refused with `E_TOOL_UNAVAILABLE`, naming the platform or saying that this build has no implementation yet. The Windows implementation arrives with card 7.2. A refusal is the honest answer, and a fabricated success would be the expensive one, because a model reasons onward from whatever it is told.

## Why it lives outside the workspace

The workspace forbids `unsafe` outright, workspace-wide, and that rule holds because nothing in the city needs it. Driving Win32 does. Rather than open a hole in a wall that twelve crates stand behind, this package sits outside the workspace — the root `Cargo.toml` excludes it — and carries its own copy of the lint table with one line changed: `unsafe_code` is `deny` rather than `forbid`, so the Win32 boundary can relax it at one call site with a written reason. Everything else is identical: no `unwrap`, no `expect`, no `panic!`, no bare indexing, no `as` casts, checked arithmetic.

The seam that makes this possible is MCP itself (`ARCHITECTURE.md` section 8): outside applications are reached over a protocol, not linked in, so a separate binary is the ordinary shape here rather than an exception. It has its own `Cargo.lock` and uses the repository's `rust-toolchain.toml` by location.

## The scope file

The server reads a `DESKTOP.toml` named by its first argument, or by `SPRAWLING_DESKTOP_SCOPE`. **No file means nothing is permitted**, and so does a file this version cannot read.

```toml
# Window titles this server may touch. `*` stands for any run of
# characters; everything else stands for itself, ignoring ASCII case.
windows = ["*Notepad*", "Calculator"]
# Processes it may touch, matched the same way.
processes = ["notepad.exe"]
# Off unless switched on.
record = false
clipboard = true
```

Four rules follow from it, and each one is a refusal a caller can act on:

- A tool that touches one window has to name that window, by `title` or by `process`, and what it names has to be on the list.
- **A capture of the whole screen is refused.** The file lists windows, so a full-screen image would show everything it left out. Name a window instead.
- `desktop.record` needs `record = true`; `desktop.clipboard` needs `clipboard = true`.
- `desktop.windows` reports only the windows the file lists, so it is how a caller learns what it may name.

## How a city attaches it

One entry in a building's `.sprawling/CONFIG.toml`, and nothing else anywhere:

```toml
[[mcp]]
label = "desk"
transport = "stdio"
command = "sprawling-desktop"
args = ["C:/where/you/keep/DESKTOP.toml"]
```

The city starts it as a child process, opens with `initialize`, sends `notifications/initialized`, reads `tools/list`, and freezes the six tools into the run's tool table under the names `desk_desktop_windows`, `desk_desktop_snapshot`, `desk_desktop_act`, `desk_desktop_screenshot`, `desk_desktop_record` and `desk_desktop_clipboard`. The assembly layer needs no change for this server: `tests/smoke.rs` starts the real binary and drives that exchange through its own pipes, which is what makes the claim checkable rather than asserted.

The transport is JSON-RPC 2.0 over stdin and stdout, one message per line, with no async runtime: a blocking read loop, one reply per request. A notification is never answered. `ping` is answered at any time; `tools/list` and `tools/call` are refused until the handshake has finished.

## The honest-refusal rule

Every refusal carries a stable code — the city's own spelling, `E_GATE_DENIED`, `E_TOOL_UNAVAILABLE`, `E_TOOL_UNKNOWN`, `E_INVALID_ARGS`, `E_CONFIG_INVALID`, `E_WIRE_MISMATCH` — and three parts in its `data`: what was refused, why, and what can be done instead. Nothing here answers a question it cannot answer, and nothing here widens its own scope to avoid a refusal.

## Building and checking

```bash
cd desktop
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo nextest run
```

`just check` at the repository root does not reach this package, because it is not a workspace member. Run the three commands above from this directory.

Read `desktop-SPEC.md` for the interfaces, the decisions and the alternatives that were rejected.
