# Calyx

Production terminal shell for [sprawling](https://github.com/2youg1/sprawling-agents) and [Mei](../Mei/). People type here; harness cores stay headless.

| Piece | Role |
|---|---|
| `Calyx/ffi` (`sprawling-console-ffi`) | Zig leaf: scene → one terminal frame |
| `Calyx/src/console` (`sprawling-calyx`) | City and Mei backends |
| `calyx` binary | Mei interactive face; resolves sibling `mei` or `MEI_BIN` |

Mei does not embed Calyx in-process. Scripted `mei -p` and piped stdin use only the headless core. Interactive Mei is the `calyx` binary (also shipped inside `@sprawling/mei` and routed by the npm shim on a TTY without `-p`). City console remains `sprawling mei` / `sprawling` serve. Calyx is not a separate npm root package.

```sh
cargo test -p sprawling-calyx --lib
cargo test -p sprawling-console-ffi --lib
```

See [`Spec.lean`](Spec.lean).
