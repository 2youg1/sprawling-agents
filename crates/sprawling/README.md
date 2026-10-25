# sprawling

The assembly root: every concrete type, the one clock, the one spawn point, and the command line. This package is the city's only binary, and it has five parts. The command line is `bin::main`, with its verbs in one table, `src/main/verbs.rs`. The assembly point, `bin::assembly`, builds the run worker, connects each port to its production adapter, and holds `SystemClock`, the one place the wall clock is read. While a city is served, `bin::serving` runs the doors, the process log and the thread that folds the views. `bin::doctor` compares what this machine has with what a city needs. And `build.rs` embeds the browser client's bundle in the binary. The module map in [`architecture.toml`](../../architecture.toml) names this package's modules `bin::<module>`; Rust code that links it as a library names it `sprawling`, and so does `cargo -p`.

It depends on every other product crate, all of them listed on its line of the `depmap` block in [`ARCHITECTURE.md`](../../ARCHITECTURE.md) section 3, and no product crate depends on it. The one workspace package that does is `citysim`, for its two bench binaries, which time the binary's own startup and queries (`ARCHITECTURE.md` section 3).

## Building and running

```bash
just build-web                      # the client bundle, written to crates/sprawling/web-dist
cargo run -p sprawling -- --help    # every verb, from the table in src/main/verbs.rs
```

Without the bundle the binary still builds, so `cargo build -p sprawling` needs no JavaScript toolchain (`just check` does, because it runs `build-web`): the binary then carries a placeholder page, the build prints a warning, and the `budget` gate refuses a release binary whose client is missing. The default feature `sandbox` turns on `runtime`'s wasm engine, so `cargo install sprawling` builds the binary the release archives carry; `--no-default-features` builds one whose tool execution is refused, with a recovery that names the missing feature, and `just features` compiles it. How a person installs and uses the binary is [`docs/getting-started.md`](../../docs/getting-started.md); how it is driven from a script is [`docs/wire.md`](../../docs/wire.md).

## Specification

The SPEC is [`Spec.lean`](Spec.lean), with its parts under [`spec/`](spec/); `ARCHITECTURE.md` section 11, *Specifications in Lean*, describes that layout. Section 1 of the SPEC maps each part to the module it specifies, section 8 says which part holds each interface section, a decision is a comment that opens with `D<n>` in the part of its topic, and section 16 names the tests that hold what the proofs leave open. The parts under `spec/Accounting/` specify modules of the `accounting` crate, and decision D30 says why they live here for now. The SPEC is written in Chinese with concept names in English, as the *Language* section of [`AGENTS.md`](../../AGENTS.md) requires.

## Checking it

```bash
cargo nextest run -p sprawling --all-features   # the tests, as `just test` runs them
lake build crates.sprawling.Spec                # the SPEC's proofs; `just models` builds every crate's
just e2e                                        # dispatches against real endpoints; passes with a line when no key is set
just adversary                                  # builds this binary and attacks it over the wire from outside; never a gate
```

Some tests under `tests/` start the real binary; others drive a dispatch against scripted ports through the same door `wire::server` uses. `tests/from_adversary.rs` holds the counterexamples the Lean adversary found, written out as Rust tests. `just e2e` names the environment variables it reads when it runs, so run it once to learn them.
