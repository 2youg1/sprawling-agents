# sprawling-desktop and sprawling-desktop-ffi

Two workspace packages live here, and `cargo -p` takes either package name.

`sprawling-desktop`, library `desktop`, is an MCP server that gives an agent eyes and hands on this Windows desktop: it lists windows, reads one window's accessibility tree, clicks and types, captures an image, records a window, and reads or writes the clipboard. It is compiled into the `sprawling` binary, and a city starts it for a building whose `RULES.toml` says `desktop = true`. It does only what one file on this machine says it may do, and that file lists windows.

`sprawling-desktop-ffi`, library `desktop_ffi`, is the FFI seam beneath this server and two other packages: the Win32 call groups that no safe crate offers, carried out whole by a Zig leaf in `crates/desktop/ffi/zig`, behind a boundary where Rust lends a buffer and its length and reads back a step and an error code. The seam decides nothing. Which window, which refusal, what a black picture means and every sentence a caller reads stay on the Rust side of the boundary.

Neither package picks a window a call did not name, captures a whole screen, starts a model, or reads an image, and neither answers a question it cannot answer. OCR and transcription are endpoints a person attaches; the desktop server offers the tree's text and the pixels.

## How it is shaped

The server's public surface is one function, `session::serve_stdio(scope_path: Option<&Path>)`. Everything else about a server is reached through its protocol.

- `rpc` (adapter) — one JSON-RPC 2.0 message per line, the caller's `id` echoed untouched, a line it cannot read answered rather than dropped. `serde_json` writes no newline, so the framing contract holds by construction.
- `session` (adapter; its `Phase` is a typestate's runtime projection) — the blocking read loop and the handshake order. No async runtime: one caller over one pair of pipes has one thing to wait for.
- `refusal` (value) — the three-part refusal and the closed six codes it may answer with.
- `answer` (value) — what a carried-out call answers: one text block of single-line JSON, a picture as an image block, a recording's sound as an audio block. The base64 is written here and nowhere else.
- `tools` (data) — the six tool cards, and `ToolName` as the one authority on their names. Editing this file is editing behaviour.
- `scope` (decision) and `scope::pattern` (value) — `DESKTOP.toml` read as an allowance, and the one glob rule. Nothing here reaches the platform, so every clause is checkable while no Win32 call has run.
- `outline` (data plus decision) — how one window's tree reads to a model: `e<n> <role> "<name>"`, two spaces of indentation per level, a closed role vocabulary that is UI Automation's own control types in lower case, names cleaned of control characters and cut to `outline::LABEL_MOST`.
- `platform` (adapter, two `cfg` arms and no trait) — `windows` routes an admitted call to one of the six; `elsewhere` speaks the whole protocol and refuses every call with `E_TOOL_UNAVAILABLE`, naming the platform it runs on.

Under `platform/windows`, one file per call group. Seven reach the platform and decide almost nothing: `enumerate`, `tree`, `focus`, `act`, `capture`, `dpi`, `clipboard`. The judgement sits in modules that run no Win32 call — `target` (exactly one window, or a refusal that lists what matched), `views` (what a snapshot minted, and which generation a window is on), `strokes` (the events one action is, and what is still held down when the desktop takes part of a batch), `encode` (a percentage scale and one of three encoders), `keys` (the key-name table), `geometry` (a screen point and a window inset as two types), `focus::settled` (whether the keyboard and the point under the pointer belong to the window this action named) — beside `reading` (each argument field refused by name rather than defaulted) and `fault` (the one place a Win32 failure becomes a refusal). That split is the Humble Object of [ARCHITECTURE.md](../../docs/ARCHITECTURE.md) §9: a machine with no desktop still checks which window was chosen, that an action against a stale generation is refused, what a picture scales to, and which keys stay held.

The seam's modules, one decision each:

- `boundary` — the four buffer rules the leaf writes Rust's memory with, public so the equivalence judge reaches them through the door production uses.
- `top_level`, `capture`, `clipboard`, `dpi` — the four desktop call groups, offered as safe functions with one `unsafe` block and one `SAFETY:` line per call into the leaf.
- `cpu_set` — the `GetSystemCpuSetInformation` records, walked by each record's own `Size` and never past the buffer. It is pure and compiled on every platform, so a record read on one machine is parsed on another.
- `cpu` — this machine's CPU sets, the calling thread's processor group, the process at full speed, a job's CPU weight and memory limit, and the completion port that reports a job that ran into its memory limit. It decides nothing about what to do with them.
- `confinement` — one native Windows process launched into an AppContainer and a job and owned for as long as the run needs it: launch, poll, terminate, cleanup, and the launch packet that carries argv and environment.
- `step` and `ended` — the step vocabulary, defined once in Rust with `zig/step.zig` held to its rendering by `build.rs`, and the reading of a number from across the boundary that refuses a value nobody defined.
- `leaf` — every export declared once, and the seam's only `unsafe extern` block.
- `fixture` — the window a contract test opens and reads back, compiled only with the `fixture` feature.

`platform.rs`'s two arms are chosen by `cfg` and never both present. The one trait either package declares is `Branches` in `platform/windows/tree.rs`, `pub(crate)`, with the UI Automation tree and a tree a test writes down as its two implementations; the walk that reads a tree is judged on both, so it runs without a desktop. On a target that is not Windows the seam compiles to `cpu_set` and `step` alone.

## When a call fails

`refusal::Refusal` carries the code, the action refused, the subject and the recovery, and it has two exits: `as_tool_result` for everything a named tool says, and `as_error` for a fault in the protocol itself. `RefusalCode` is a closed set of six that maps onto `kernel::AxCode`, so a new code is minted in `kernel` first.

| Code | refused when |
|---|---|
| `E_WIRE_MISMATCH` | the line is not JSON, not an object, or names no `method` |
| `E_TOOL_UNKNOWN` | the method or the tool name is not one this server offers |
| `E_INVALID_ARGS` | `params` does not read as the shape the schema promised, or `tools/call` names no tool |
| `E_GATE_DENIED` | the scope does not permit it: no file, a window it does not list, `record` or `clipboard` switched off, a whole-screen capture, or a handshake that has not finished |
| `E_CONFIG_INVALID` | the scope file has a syntax error or a key this version does not know, which closes the scope rather than narrowing it |
| `E_TOOL_UNAVAILABLE` | this build cannot do it: another platform, a request the desktop would not take, or a DPI declaration that did not hold |

A refusal a tool returns is an `isError` result whose text is the three parts: `E_…: cannot <action> — <subject>`, then `instead: <recovery>`. Only the handshake, an unknown method or tool, an unreadable line and unreadable `params` are JSON-RPC errors. A refusal that happened after part of the request was handed to the desktop carries `_meta: { "sprawling/effect-unknown": true }`, whose key is `agent_protocols::EFFECT_META_KEY`. The city marks that answer `Retry::Unknown`, a value a caller sends again only when it knows the action is idempotent, and a click is not.

Each recovery is the call site's, and it names something a caller can do: add the window's pattern to `DESKTOP.toml`, name a window instead of the screen, set `record = true` or `clipboard = true`, write `sound = "<device>"`, send `initialize` then `notifications/initialized`, read `tools/list`, run this server on the Windows machine whose desktop is to be driven.

These requests are refused rather than attempted:

- A window name that matches no window, or more than one. Two matches is a refusal that lists their titles; this server never picks one.
- A capture of the whole screen, and a capture whose pixels come back all black, which some windows answer to `PrintWindow`.
- A keystroke while another window holds the keyboard, and a pointer action whose point is covered by another window. Both refusals say that nothing was sent.
- An action against a stale generation, or against a `ref` no snapshot minted.
- A `scale` that would produce zero pixels, and a `quality` or `scale` outside its domain, refused at the parse point rather than clamped.
- A key name not in `keys`, refused with the whole table in the refusal.
- Starting a recording twice, stopping one that never started, `audio: true` with no `sound` in the scope file, and `audio: true` on a machine with no `ffmpeg`.

A batch of input the desktop takes only in part is reported as one refusal with the count it took, marked effect unknown, and after it only the lifts of keys and mouse buttons still held are sent: never a second press, never the whole batch again.

The seam answers a step rather than a sentence. `desktop_ffi::step::Step` names where an export stopped (`Finished`, `Absent`, `NoRoom`, `NoWindow`, `Drawing`, `Handing` …), `ended` turns the returned number into one of them so a leaf from another build meets a refusal, and `Failure::At { step, code }` carries the calling thread's Win32 error while `Failure::Unspelled` is a number no step defines. `confinement` keeps its own `Failure`, which carries the action, the code, the phase it stopped in, and the resources a refused cleanup left behind. `NoRoom` carries the length that would have fitted, and the two buffered calls (`top_level::windows`, `clipboard::text`) retry with a larger buffer a bounded number of times. Which sentence a step becomes is `crates/desktop/src/platform/windows/fault.rs`'s and not the seam's.

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

Four refusals follow from it, and each one is a refusal a caller can act on:

- A tool that touches one window has to name that window, by `title` or by `process`, and what it names has to be on the list. When both are given, both have to match.
- **A capture of the whole screen is refused.** The file lists windows, so a full-screen image would show everything it left out. Name a window instead.
- `desktop.record` needs `record = true`; `desktop.clipboard` needs `clipboard = true`.
- `desktop.record` with `audio: true` hears the device `sound` names and no other; this server never picks one. Its `stop` answers with the sound as an audio block, which the city stores and names by its content address.

`desktop.windows` reports only the windows the file lists, matched by the patterns that admitted the call, so it is how a caller learns what it may name.

## How a city starts it

One line in a building's `.sprawling/RULES.toml`, and no `[[mcp]]` entry:

```toml
desktop = true
```

The city starts its own executable as `sprawling desktop <building>/.sprawling/DESKTOP.toml`, opens with `initialize`, sends `notifications/initialized`, reads `tools/list`, and freezes the six tools into the run's tool table under the names `desktop_desktop_windows`, `desktop_desktop_snapshot`, `desktop_desktop_act`, `desktop_desktop_screenshot`, `desktop_desktop_record` and `desktop_desktop_clipboard`. `crates/sprawling/tests/desktop.rs` starts the real binary that way and checks the six names the model is offered; it is the one test that proves the city and this server meet. A building that already names a server `desktop` in its own `[[mcp]]` keeps that server, and the city starts none of its own.

The transport is JSON-RPC 2.0 over stdin and stdout, one message per line. A notification is never answered; answering one would leave a line in the pipe and every later call would read the answer to the message before it. `ping` is answered at any time; `tools/list` and `tools/call` are refused until the handshake has finished, and `notifications/initialized` arriving before `initialize` is refused with the phase left where it was.

## Where `unsafe` lives

The server inherits the workspace's `unsafe_code = "forbid"` like every other crate: its production code and its tests hold none. Input and the facts of a window go through `winsafe`, the accessibility tree through `uiautomation`, each admitted call by call against a contract test that passed on the hand-written version first.

The seam writes out a lint table of its own — the workspace's table with one line changed, `unsafe_code` at `deny` — so each call into the leaf can relax the lint at that one statement. `cargo xtask guard` holds every other key of that table to the workspace's. Every `unsafe` in production code is one call into the leaf, under `crates/desktop/ffi/src`, and carries a `SAFETY:` line stating the precondition that makes the call sound — the thing a reader could in principle find false, never a restatement of the call. "We call the leaf" is not a precondition; "the vector holds as many initialised slots as the capacity the leaf is held to, lent to this call alone" is. The way to review the seam is to ask each `SAFETY:` line whether what it claims could be wrong; if it could not, it is a paraphrase and not a precondition. `crates/desktop/Spec.lean` §8-12 is the one list of them, and `crates/desktop/ffi/Spec.lean` proves what the leaf writes into the buffers it is lent and that it gives back what it takes.

Building either package on Windows needs the Zig that `crates/desktop/ffi/zig-version` pins, which is 0.17.0. `crates/desktop/ffi/build.rs` refuses a `zig` it cannot start, and one whose `zig version` differs from the pin, and prints the `winget install --id zig.zig -e --version 0.17.0` line that installs it; the check runs on a Windows target, because the leaf is built nowhere else. When `DOCS_RS` is set, `build.rs` returns without Zig, because docs.rs has none and rustdoc does not link the leaf (`desktop_ffi` D7). `sprawling doctor`'s `zig` row compares the installed version with the same file on every platform, and CI's install step reads it too. `just check-desktop` and `just fmt-check` run whichever `zig` is first on `PATH`.

`desktop_ffi::fixture` is compiled only under the `fixture` feature, and only `crates/desktop/Cargo.toml`'s Windows dev-dependencies turn it on; `cargo xtask artifact` counts `fixture` among the test features, so no shipped binary carries it. The window a contract test opens, and the `unsafe` that opening one costs, therefore live in the seam rather than in the server's tests.

## Constraints a reader acts on

- `sprawling-desktop` declares no feature. `sprawling-desktop-ffi` declares one, `fixture`, off by default.
- The server starts no async runtime, no HTTP client and no GUI framework, and holds no process-wide state: a `Desk` belongs to one connection, so a second connection under a second scope file cannot stop the first one's recording. It runs one thread for the whole package, the frame grabber in `record::sink`, stopped by one flag and joined by `stop`.
- A recording lands under `std::env::temp_dir()/sprawling-desktop/`, never inside a city: the scope file permits windows and says nothing about writing files. A screenshot is answered, never saved. The two `ffmpeg` processes a recording may start come from `child::command`, so they inherit neither the city's secret keys nor its console.
- `sprawling-desktop` may name `kernel`, `agent_protocols`, `child`, and `desktop_ffi` on Windows. It reads four facts rather than copying them: `kernel::AxCode` for the codes, `kernel::consts_policy::IMAGE_QUALITY` for the `quality` domain, and `agent_protocols::PROTOCOL_VERSION` with `EFFECT_META_KEY` for the revision and the effect-unknown key. `sprawling-desktop-ffi` may name `child` and nothing else by the depmap; its manifest declares no workspace crate, and takes `winsafe` and `windows` from outside.
- D2: everything a named tool says, refusals included, is a `CallToolResult` with `isError`, so a client must read the text rather than a JSON-RPC `data` field.
- D3: effect unknown travels in `_meta` under `sprawling/effect-unknown`, so a client that ignores `_meta` will retry an action that may have landed.
- D5: a `ref` is a rectangle and a generation, keyed by window handle, and dies when the window moves or resizes. A caller takes a fresh snapshot after either.
- D6: every rectangle the desktop reports and every coordinate an action is computed from is a physical pixel of the virtual desktop, while a caller's `point` is measured inside the window it named. `scale` is a percentage of the window's own pixels, not the device pixel ratio the browser tool calls by the same name.
- D13: a recording's sound arrives as an audio block in the `stop` answer, capped by `SOUND_CARRIED_MOST`; a longer one is named by its path instead.
- D15: `notifications/initialized` before `initialize` is refused and leaves the phase unchanged, so a client sends the two in the order the protocol states.
- `desktop_ffi` D1 and D5: one export performs a whole operation and no transient handle crosses the boundary, while a native `exec` process is held by a safe Rust owner across polls — its cleanup is explicit and retryable, and dropping the owner is emergency teardown that writes what it could not release to stderr.

## Who uses it

`cargo tree --workspace -i sprawling-desktop -e normal --depth 1` prints `sprawling`: the assembly root links this package as a library, `bin::main::desktop` reads `serve_stdio`, and the city starts the same binary as a child.

`cargo tree --workspace -i sprawling-desktop-ffi -e normal --depth 1` prints `sprawling`, `sprawling-desktop` and `sprawling-runtime`. `sprawling-desktop` reads the four desktop call groups; `sprawling` reads the processor topology, the calling thread's group and the power-throttling opt-out for its placement plan; `sprawling-runtime` reads the AppContainer launch and the run job's CPU weight, memory limit and memory-hit watch for a native `exec` command.

The crates each package may name are its rows in the `depmap` block of [ARCHITECTURE.md](../../docs/ARCHITECTURE.md) §3.

## The specification

`crates/desktop/Spec.lean` is the server's entry, and its parts live under `crates/desktop/spec/`: `Scope.lean` and `Scope/Pattern.lean` for which request is admitted and the glob rule, `Session.lean` for the handshake, `Outline.lean` for how a tree folds into text, and `Platform/Windows/Strokes.lean`, `Views.lean` and `Target.lean` for what stays held, which generation is stale and which window is chosen. §1 splits the units, §2 states each one's acceptance criterion, §3 the assumptions, §7 the module boundary, §8 the interfaces and the six tool cards, §11 the boundary cases, §13 the dependencies with their reasons, §14 every constant with whose fact it is, §15 what a change reaches, §16.2 how a thing that needs a desktop is tested, and §17 what to re-read when something changes. §10 holds the ten designs and the decisions after them about partial input, generations, coordinates, recording and the clipboard. §12 holds the code table with D2 and D3. The decisions D16 and D17 follow §14's constant table, and §6 fixes this crate's own words: scope, reference and generation.

`crates/desktop/ffi/Spec.lean` is the seam's entry and has no parts under a `spec/` directory. §1 splits it into buffer rules, resource pairing, the processors-and-shares group and the Rust face; §2 says what the equivalence checks cover and what they do not; §8 gives the exports; §10 proves the buffer rules, the record walk and the resource pairing; §12 records the steps, and the decisions D1 to D6 sit across §12 and §14.

[ARCHITECTURE.md](../../docs/ARCHITECTURE.md) §11, *Specifications in Lean*, says how a SPEC is laid out, and [docs/glossary.md](../../docs/glossary.md) holds three of the words these files use: desktop connector, three-part refusal, undoable effect.

## Checking it

```bash
cargo nextest run -p sprawling-desktop -p sprawling-desktop-ffi   # both packages; the Windows arm compiles on Windows only
cargo clippy -p sprawling-desktop -p sprawling-desktop-ffi --all-targets --all-features -- -D warnings
lake build crates.desktop.Spec                                    # the server's Lean specification
lake build crates.desktop.ffi.Spec                                # the seam's
just check-desktop                                                # the leaf's own Zig tests, on Windows; a line elsewhere
just fuzz-desktop 1000000 5eedf022                                # the leaf against its Rust reference, rounds from a seed
```

`just test`, `just clippy` and `just fmt-check` judge both packages with the whole workspace. `cargo xtask guard` holds the seam's lint table to the workspace's key by key, with `unsafe_code` the one recorded exception; `cargo xtask artifact` refuses a shipped binary that carries `fixture`.

A machine with no desktop still checks the decisions: which window a name selects, whether a stale generation is refused, what a picture scales to, which keys stay held, which windows `desktop.windows` may report, and the four buffer rules. The Win32 call groups are checked for the honesty of their refusals rather than for a click that landed, because a build agent has no desktop; whether a click lands where it was aimed is the operator check `crates/desktop/Spec.lean` §16.2 records as a named gap.
