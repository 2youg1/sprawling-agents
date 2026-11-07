# sprawling-console-ffi

The console's renderer. Rust writes a scene — the lines of the transcript, the live area, the composer and where its cursor stands — and a Zig leaf, `crates/console_ffi/zig/leaf.zig`, turns it into the bytes of one terminal frame. What the console shows is decided in `crates/sprawling`, whose `crates/sprawling/spec/Console.lean` section 8-11 says so; this package decides how it looks. Rendering is a pure function: the same scene gives the same bytes, and the leaf reads no clock, opens no file and keeps no state between calls.

It does not write to a terminal, hold a screen, or decide which frame comes next. `draw` fills a buffer the caller lends and answers where the cursor ended up, so the caller owns the device and this package owns the bytes.

## How it is shaped

<!-- coderabbit-ai-generated:v1 -->

<!-- coderabbit-ai-generated-section:v1:32db9fcb103384d86c55c0833ec025502209ec3e4dadbe41c6a40fb8dcb9da06 -->

- `console_ffi` — the package's face: `draw`, `Frame` and `Refused`, re-exported from `leaf`.
- `leaf` — the leaf's one export, declared once, and the safe function around it. Only a `Full` result retries, while the buffer is below 64 MiB, with a buffer at most twice as long and capped at 64 MiB. This is an automatic growth limit, not a hard maximum: a caller-provided capacity above 64 MiB is used as supplied.
- `part` — the words a scene is written in: the kind of frame, the kind of transcript line, the rows of the live area, the outcome of a tool call and of a run, the answer to a request, and the leaf's return status. This file is the only definition; `build.rs` reads the same file.
- `scene` — what a frame is drawn from, and the bytes the leaf reads it as. It is the one writer of a scene, and it says what is on the screen without laying any of it out: a tool call and how long it took, the characters being typed and where the cursor stands among them.
- `zig/leaf.zig` and its neighbours each hold one decision: `scene.zig` reads a scene with every read bounded by its own length, `text.zig` gives a character's column width and draws a control character as U+FFFD, `paint.zig` is the one writer of the frame's bytes, `flow.zig` lays running text into a column, `format.zig` spells a time of day, a duration and a count, `grid.zig` fixes the columns, `transcript.zig` writes the lines that enter the scrollback once, `live.zig` draws the rows redrawn each frame, `quiet.zig` draws the quiet host, `frame.zig` wraps one frame as a synchronized update, and `part.zig` repeats the words the build holds it to.

<!-- /coderabbit-ai-generated:v1 -->

This is the adapter of the seven shapes in [`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 9, and the humble object: the hard-to-test end is a byte writer, and the tests judge it through a terminal emulator instead of reading escape sequences.

A main-screen frame erases the live region from the row the last frame left the cursor on, writes the transcript lines that are new, draws the live area and puts the cursor back in the composer; the whole frame sits between the synchronized-update sequences, which a terminal that does not know them draws the same way as one that does. The quiet host draws in the alternate screen instead, and its frame carries no transcript: an address, a pairing code, and at most one transient line.

## When a call fails

`draw` answers `Result<Frame, Refused>`, and `Refused` has two variants:

<!-- coderabbit-ai-generated:v1 -->

<!-- coderabbit-ai-generated-section:v1:78b465fd735d6115572ec14cd9e5022cc86923302d754f41a6a76fa93e66fcd4 -->

| Variant | When | What a caller does |
|---|---|---|
| `Malformed` | The leaf could not read the scene it was given | Report it. A scene is produced by the console in the same city, so a scene the leaf cannot read is a defect in the console, not an input a person can supply. |
| `TooLong` | The leaf returned `Full` with a buffer of at least 64 MiB | Reduce the text rendered in one frame. User text and its rendered transcript can legitimately exceed 64 MiB; this need not be an internal defect. |

`Refused` implements `Display` and `Error`. Its message says the city keeps serving: `Malformed` asks for a report against `crates/console_ffi`, while `TooLong` asks the caller to reduce the text rendered in one frame. There is no `AxCode` in this package.

Two refusals happen instead of a partial drawing. A frame past the longest buffer is never written partially, and a scene the leaf cannot read is never drawn as far as it reads. Only `Full` retries while the buffer is below the automatic growth limit, at most doubling its length up to that limit. `Malformed` returns immediately without retrying.

<!-- /coderabbit-ai-generated:v1 -->

## Where `unsafe` lives

Rust lends the leaf the scene's bytes and an output buffer, and reads back the frame's length and the row the cursor ended on. That is this package's only `unsafe`: one `unsafe extern "C"` declaration in `crates/console_ffi/src/leaf.rs` and one `unsafe` block inside the safe function `console_ffi::draw`. The block carries a `SAFETY:` line stating the precondition it rests on — both slices are live and their lengths travel beside their pointers, the buffer the leaf writes into is one it was lent, and the leaf keeps none of the pointers past the call — and the declaration and the call each relax the lint at that one item with an `#[expect]` that gives its reason.

Because of that, the package writes out a lint table of its own: the workspace's table with `unsafe_code` at `deny` instead of `forbid`. `cargo xtask gates guard` holds it to the workspace's key by key, and names this package and `crates/desktop/ffi` as the only two allowed to have one. `crates/console_ffi/Spec.lean` D1 records the decision and the alternatives it beat.

## Constraints a reader acts on

<!-- coderabbit-ai-generated:v1 -->

<!-- coderabbit-ai-generated-section:v1:e7d8a91cbf2b6929ef3778dfb310830bfa55abaf5591115bc78bb4fbe0e5021f -->

- The package declares no features and names no workspace package. `vt100` is a development dependency, so the tests can read a frame the way a person does.
- The leaf is built on Windows, Linux and macOS, so Zig at the version `crates/console_ffi/zig-version` pins is needed to build the workspace on all three; `sprawling doctor` lists it with the other tools. `build.rs` refuses a `zig` that answers another version, and inside this repository it also refuses a pin that differs from the one in `crates/desktop/ffi/zig-version`, because both leaves are built with one Zig. A package built from crates.io has no sibling and skips that second check.
- `zig/part.zig` must spell the enums `src/part.rs` defines. `build.rs` refuses to build when it does not, and prints the text it should hold.
- The leaf is compiled with `-O ReleaseSafe`, so every bounds and overflow check stays in the shipped binary and a failed one traps; `-fPIC` is added for every non-Windows target. The build refuses a target other than x86_64 or aarch64, and one that is not Windows, Linux or macOS.
- `build.rs` reruns only when `zig/`, `src/part.rs` or one of the pins changes, never when `PATH` does: the leaf is built by the pinned compiler, and an editor whose `PATH` differs from CI's would otherwise rebuild it on every call.
- A record cannot drive the terminal. Text passed through the character renderer replaces C0, C1, DEL and invalid UTF-8 with U+FFFD (D5); layout may interpret newlines and whitespace before that step. OSC 8 link URIs filter those controls and invalid UTF-8 before output, colour comes from the terminal's own palette (D3), and nothing on the screen moves on its own (D4).

<!-- /coderabbit-ai-generated:v1 -->

## Who uses it

One workspace package depends on it: `sprawling`, where `crates/sprawling/src/console/screen.rs` draws every frame through `console_ffi::draw`. `cargo tree --workspace -i sprawling-console-ffi -e normal --depth 1` prints the current list, which is that one package. Its line in the `depmap` block of [`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 3 allows it no workspace package at all.

## The specification

[`Spec.lean`](Spec.lean) holds the whole specification, with no parts under a `spec/` directory. Section 1 names the units and section 8 the interface, and the model proves what fitting text into a column comes to: a fitted text never runs past the columns left for it, and one that fits is written whole, so the terminal's last column is never written. The decisions D1 to D9 follow section 8: the leaf and its single `unsafe`, the words defined once, the palette, a screen with nothing moving on its own, control characters, the columns every line stands on, the composer, where the live area sits, and the banner's one hint.

A decision is a comment opening with `D<n>` directly above what it governs, cited elsewhere as `console_ffi D<n>`. The SPEC is written in Chinese with concept names in English, as the *Language* section of [`AGENTS.md`](../../AGENTS.md) requires, and section 11 of [`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md), *Specifications in Lean*, says how a SPEC in Lean is laid out.

## Building and checking

```bash
cargo nextest run -p sprawling-console-ffi    # each frame written into a terminal emulator and compared whole
lake build crates.console_ffi.Spec             # this package's Lean specification
just check-console                            # the leaf's own Zig tests
zig fmt --check crates/console_ffi/zig        # the leaf's formatting, which `just fmt-check` also runs
cargo xtask gates guard                       # this package's lint table against the workspace's
```

<!-- coderabbit-ai-generated:v1 -->

<!-- coderabbit-ai-generated-section:v1:d4db5e9888d7c2c2b606a77e5e28b357091ffde0c447f2017f1a247898c6bac4 -->

`just test` runs workspace Rust tests, `just models` builds all Lean specifications, and `just fmt-check` checks formatting. None runs `just check-console` (Zig tests) or `cargo xtask gates guard`; run those separately, or use `just check`, which includes them.

<!-- /coderabbit-ai-generated:v1 -->
