# Calyx

Production terminal shell for [sprawling](https://github.com/2youg1/sprawling-agents) and [Mei](../Mei/). People type and read here; harness cores stay headless.

Chinese: [`README.zh-CN.md`](README.zh-CN.md).

## What it is

Calyx is the interactive face around the city console and the Mei process Face. It owns raw terminal mode, keys and paste, the line editor, slash menus, face lifecycle, screen orchestration (including OSC 7501 program status), and spawning the headless `mei` binary. The frame renderer is a pure Zig leaf: a scene in, one terminal frame out.

WebUI (`RefRain/`) is a separate consumer. Calyx does not implement the model loop, declare tools, or inject AGENTS.

## Layout

| Path | Role |
|---|---|
| `shell/` | Zig shell — sole implementation of the interactive face |
| `build.zig` | Zig build authority for Calyx (shell, leaf linkage, tests) |
| `ffi/` (`sprawling-console-ffi`) | Zig leaf + scene vocabulary; Rust safe `draw` for hosts/tests |
| `src/` | Rust city/wire/kernel host only — narrow ABI into the Zig shell |
| `Spec.lean` | Shell contract and decisions |

Package names stay `sprawling-calyx` and `sprawling-console-ffi`. The sole `calyx` binary is the Zig face from `build.zig`; `Calyx/build.rs` installs it into the cargo target directory beside `sprawling` so sibling/`CALYX_BIN` resolve works. It ships beside `mei` inside `@sprawling/mei` (npm shim: TTY without `-p`/`--resume` → `calyx` with `MEI_BIN` set; otherwise the headless core).

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

See [`Spec.lean`](Spec.lean) and [`ffi/Spec.lean`](ffi/Spec.lean).
