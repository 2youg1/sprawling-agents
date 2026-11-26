# Calyx

[简体中文](README.zh-CN.md)

**Read a long run and keep your hands on the keyboard.** Calyx is the interactive face of [Sprawling](../Sprawling/README.md) and [Mei](../Mei/README.md), not a general-purpose terminal emulator.

- **Read long runs without losing your place.** In Mei, the composer stays at the bottom while the transcript scrolls. The transcript reflows at the new width and keeps your reading position. Reasoning and Python output can fold away; replies can show Markdown tables, math, diagrams and image previews.
- **Drive it from the keyboard.** Type a slash prefix and use Tab for command completion in the city console. In Mei, `/e` sets effort for the next run, and Esc can stop the current run without closing Calyx.
- **Keep the interface separate from the harness.** Zig handles terminal input, editing and drawing. Its renderer takes a scene and returns a frame; the narrow Rust bridge connects it to the city. Mei stays a separate headless process, so scripts and ACP clients do not need the screen.

## Try it

Install [Magpie](https://usemagpie.ai), configure a model route and leave it running. With Bun, install Mei:

```sh
bun install --global @sprawling/mei@latest
```

Open a new terminal. If `calyx` is not found, run `bun pm bin --global` and add the reported directory to `PATH`. Then run:

```sh
calyx
```

Use a disposable workspace: [Mei runs generated Python with your account's permissions](../Mei/README.md). With a release archive or source build, also run `calyx` explicitly; the crates.io Mei install is headless. If you already run a Sprawling city, press Esc in the host terminal, type `/w`, and press Tab. Select `/web` to return to RefRain on the same city.

<a name="what-it-is"></a>
<a name="layout"></a>

## Technology and source layout

| Path | Role |
|---|---|
| `shell/` | Zig shell — sole implementation of the interactive face |
| `build.zig` | Zig build authority for Calyx (shell, leaf linkage, tests) |
| `ffi/` (`sprawling-console-ffi`) | Zig leaf + scene vocabulary; Rust safe `draw` for hosts/tests |
| `src/` | Rust city/wire/kernel host only — narrow ABI into the Zig shell |
| `Spec.lean` | Shell contract and decisions |

Package names stay `sprawling-calyx` and `sprawling-console-ffi`. The sole `calyx` binary is the Zig face from `build.zig`; `Calyx/build.rs` installs it into the cargo target directory beside `sprawling` so sibling/`CALYX_BIN` resolve works. It ships beside `mei` inside `@sprawling/mei` (npm shim: TTY without `-p`, `--acp` or `--resume` → `calyx` with `MEI_BIN` set; otherwise the headless core).

## Ownership

- **Zig** owns the shell end to end (`Calyx/Spec.lean` D7).
- **Rust** keeps only the city host: subscribe to `wire` records, run slash/room actions that must touch accounting/kernel/wire, and bridge those into the shell ABI.
- **Mei** stays a separate headless process. Scripted `mei -p` and piped stdin never need Calyx.

Shell logic must not regress into `Calyx/src`; the residue gate refuses that shape.

## Memory and ABI

Zig code at a C boundary uses caller-owned buffers and explicit lengths. Anything that allocates takes an `Allocator`. Temporary work prefers a fixed buffer over a hidden global allocator. The frame leaf stays pure: no clock, no files, no retained state between draws.

## Build and check

Zig is pinned at `ffi/zig-version` (same pin as `Sprawling/desktop/ffi/zig-version`).

```sh
cargo test -p sprawling-calyx --lib
cargo test -p sprawling-console-ffi --lib
cd Calyx && zig build test
```

`just check-console` runs the frame-leaf Zig tests (`Calyx/ffi/zig/leaf.zig`). Shell coverage is `cd Calyx && zig build test`. Format Zig with `zig fmt` on `Calyx/ffi/zig` and `Calyx/shell` (see the root `justfile`).

## City vs Mei

| Face | How it starts |
|---|---|
| Mei interactive | `calyx` (or the Mei npm shim on a TTY) |
| City console | `sprawling` serve path, through the Rust host into the same Zig shell |
| Mei headless | `mei -p`, `mei --resume`, or non-TTY stdin — Calyx not involved |

In the Sprawling console, type `/w` and press Tab to open command completion.

[![The slash menu after `/w` and Tab: `/web` under the cursor, `/wire` below it, and the keys that move through them](../docs/images/console-slash-menu.webp)](../docs/images/console-slash-menu.webp)

See [`Spec.lean`](Spec.lean) and [`ffi/Spec.lean`](ffi/Spec.lean).
