# sprawling-desktop

An MCP server that gives an agent eyes and hands on this Windows desktop: it lists windows, reads their accessibility tree, clicks and types, captures images, records, and reads the clipboard. It is compiled into the `sprawling` binary, and a city starts it for a building whose `RULES.toml` says `desktop = true`. It does only what one file on this machine says it may do.

**On Windows this build carries all six out.** Elsewhere every call is refused with `E_TOOL_UNAVAILABLE` naming the platform, because a refusal is the honest answer and a fabricated success would be the expensive one: a model reasons onward from whatever it is told.

The judgement is deliberately kept away from the operating system. Five modules touch Win32 and decide almost nothing; five decide and touch nothing — which window a name meant, whether an action's view has moved on, what a percentage scales an image to, what a key name is, and what a call's arguments actually say. Those five are tested on any machine, with or without a desktop. Whether a click lands where it was aimed is checked by an operator on a real desktop, and `desktop-SPEC.md` §16.2 records that as a named gap rather than papering over it with a test that would pass on an empty build agent.

## Where `unsafe` lives

The workspace forbids `unsafe`, and this package inherits that rule like every other crate: its production code and its tests hold none. Input and the facts of a window go through `winsafe`, and the accessibility tree through `uiautomation`, each admitted call by call against a contract test that passed on the hand-written version first. The four call groups with no admitted safe interface — enumerating windows, capturing one, the clipboard, and the DPI declaration — are carried out whole by a Zig leaf in `ffi/zig/`, behind a boundary where Rust lends a buffer and its length and reads back a step and an error code. No handle, device context or global block crosses it, so every resource is released inside the call that took it.

**The seam, `ffi/`, is the one crate with a lint table of its own.** It is the workspace's table with one line changed: `unsafe_code` is `deny` rather than `forbid`, so each call into the leaf can relax it at that one statement, with a written reason. `cargo xtask guard` holds every other key of that table to the workspace's. Every `unsafe` in production code is one call into the leaf, in `ffi/src/`, and carries a `SAFETY:` comment stating the precondition that makes the call sound — the thing a reader could in principle find false, never a restatement of the call. "We call the leaf" is not a precondition; "the vector holds as many initialised slots as the capacity the leaf is held to, lent to this call alone" is. The way to review the seam is to ask each `SAFETY:` line whether what it claims *could be wrong*. If it could not, it is a paraphrase and not a precondition. `desktop-SPEC.md` section 8-12 is the one list of them; `ffi/Spec.lean` proves what the leaf writes into the buffers it is lent and that it releases what it takes, and a Rust reference checks the leaf's buffer rules on drawn inputs. The window a contract test opens is the seam's too, in `ffi/src/fixture.rs`, compiled only with the `fixture` feature that this package's Windows dev-dependencies switch on.

`sprawling` links this package as a library, and still reaches it over MCP in a process of its own: the city starts its own executable as `sprawling desktop <DESKTOP.toml>` and talks to it over that child's pipes. The COM state, `SendInput` and every call into the leaf therefore run in the child, never in the process that writes the city's Ledger. Building on Windows needs the Zig that `ffi/zig-version` names; `ffi/build.rs` refuses any other version and says how to install that one.

## The scope file

The server reads the `DESKTOP.toml` named by the argument after `sprawling desktop`. A city passes the building's own file, `<building>/.sprawling/DESKTOP.toml`, which the settings page writes. **No file means nothing is permitted**, and so does a file this version cannot read.

```toml
# Window titles this server may touch. `*` stands for any run of
# characters; everything else stands for itself, ignoring ASCII case.
windows = ["*Notepad*", "Calculator"]
# Processes it may touch, matched the same way.
processes = ["notepad.exe"]
# Off unless switched on.
record = false
clipboard = true
# The one sound device a recording may hear, by its DirectShow name;
# `ffmpeg -hide_banner -list_devices true -f dshow -i dummy` lists them.
# Absent, a recording that asks for sound is refused.
sound = "Microphone (USB Audio)"
```

Four rules follow from it, and each one is a refusal a caller can act on:

- A tool that touches one window has to name that window, by `title` or by `process`, and what it names has to be on the list.
- **A capture of the whole screen is refused.** The file lists windows, so a full-screen image would show everything it left out. Name a window instead.
- `desktop.record` needs `record = true`; `desktop.clipboard` needs `clipboard = true`.
- `desktop.record` with `audio: true` hears the device `sound` names and no other; this server never picks one. Its `stop` answers with the sound as an audio block, which the city stores and names by its content address.
- `desktop.windows` reports only the windows the file lists, so it is how a caller learns what it may name.

## How a city starts it

One line in a building's `.sprawling/RULES.toml`, and no `[[mcp]]` entry:

```toml
desktop = true
```

The city starts its own executable as `sprawling desktop <building>/.sprawling/DESKTOP.toml`, opens with `initialize`, sends `notifications/initialized`, reads `tools/list`, and freezes the six tools into the run's tool table under the names `desktop_desktop_windows`, `desktop_desktop_snapshot`, `desktop_desktop_act`, `desktop_desktop_screenshot`, `desktop_desktop_record` and `desktop_desktop_clipboard`. `crates/sprawling/tests/desktop.rs` starts the real binary that way and checks the six names the model is offered. A building that already names a server `desktop` in its own `[[mcp]]` keeps that server, and the city starts none of its own.

The transport is JSON-RPC 2.0 over stdin and stdout, one message per line, with no async runtime: a blocking read loop, one reply per request. A notification is never answered. `ping` is answered at any time; `tools/list` and `tools/call` are refused until the handshake has finished.

## The honest-refusal rule

A tool's own refusal is an MCP `isError` result whose text carries the stable code — the city's own spelling, `E_GATE_DENIED`, `E_TOOL_UNAVAILABLE`, `E_TOOL_UNKNOWN`, `E_INVALID_ARGS`, `E_CONFIG_INVALID`, `E_WIRE_MISMATCH` — and the three parts: what was refused, why, and what can be done instead. Only protocol faults (the handshake, an unknown method or tool, an unreadable line) are JSON-RPC errors, with the same three parts in their `data`. Nothing here answers a question it cannot answer, and nothing here widens its own scope to avoid a refusal.

## Building and checking

Both packages are workspace members, so the workspace's own commands judge them: `just fmt-check`, `just clippy` and `just test`, or `cargo nextest run -p sprawling-desktop -p sprawling-desktop-ffi` for these two alone. On Windows `just check-desktop` runs the leaf's own Zig tests, and `just fuzz-desktop <rounds> <seed>` runs the leaf's equivalence with its Rust reference for as long as asked.

Read `desktop-SPEC.md` for the interfaces, the decisions and the alternatives that were rejected.
