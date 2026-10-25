# sprawling-console-ffi

The console's renderer. Rust writes a scene — the lines of the transcript, the live area, the composer and where its cursor stands — and a Zig leaf, `crates/console_ffi/zig/leaf.zig`, turns it into the bytes of one terminal frame. What the console shows is decided in `crates/sprawling` (`crates/sprawling/spec/Console.lean` §8-11); this package decides how it looks. Rendering is a pure function: the same scene at the same width gives the same bytes, and the leaf reads no clock, opens no file and keeps no state.

## Where `unsafe` lives

Rust lends the leaf the scene and an output buffer and reads back the frame's length and the row the cursor ended on. That one call is the package's only `unsafe` block, inside the safe function `console_ffi::draw` (defined in `crates/console_ffi/src/leaf.rs`), with a `SAFETY:` line that states the precondition the call depends on. For that reason this package writes out a lint table of its own: the workspace's table with `unsafe_code` at `deny` instead of `forbid`, which `cargo xtask guard` holds to the workspace's key by key. `crates/console_ffi/Spec.lean` D1 records the decision and the alternatives it beat.

The words of a scene are defined once, in `crates/console_ffi/src/part.rs`. `crates/console_ffi/build.rs` reads that file and refuses to build when `crates/console_ffi/zig/part.zig` does not spell the same enums, printing the text it should hold.

## Who uses it

One workspace package depends on it: `sprawling`, whose console draws every frame through it. It depends on no workspace package.

## Building

The leaf is built on every platform. `build.rs` compiles it with the Zig version that `crates/console_ffi/zig-version` pins and links it statically; it refuses any other Zig, and inside this repository it also refuses a pin that differs from `crates/desktop/ffi/zig-version`.

## The specification

`crates/console_ffi/Spec.lean` holds the whole specification and has no parts under a `spec/` directory. It proves one property of the layout, that text written into a line never runs past the columns left in it, and records the decisions about colour, columns and control characters. `ARCHITECTURE.md` section 11, *Specifications in Lean*, says how a SPEC is laid out.

## Checking

```bash
cargo nextest run -p sprawling-console-ffi   # frames drawn into a terminal emulator and compared whole
lake build crates.console_ffi.Spec           # this package's Lean specification
just check-console                           # the leaf's own Zig tests
zig fmt --check crates/console_ffi/zig       # the leaf's formatting, also run by `just fmt-check`
```
