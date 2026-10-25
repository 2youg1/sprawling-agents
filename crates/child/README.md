# sprawling-child

The one place the city builds a child process. `child::command(program)` returns a `std::process::Command` that is already off the city's console (`CREATE_NO_WINDOW` on Windows), in a process group of its own (`process_group(0)` on Unix), and stripped of the environment keys that carry the city's secrets: every key that begins `SPRAWLING_SECRET_`, and `SPRAWLING_PAIRING_TOKEN`. Those two names are defined in `child::environment` and nowhere else; `gateway::credential::vault` reads the prefix from there.

The package only configures. The caller still sets the arguments and the three standard streams and calls `spawn`.

`clippy.toml` refuses `std::process::Command::new` everywhere, so a spawn point that does not go through `command` is visible where it stands: it carries `#[expect(clippy::disallowed_methods, reason = "…")]` naming which exception it is. `crates/child/Spec.lean` D4 lists the exceptions.

## Who uses it

It depends on no crate, so every package that starts a process may depend on it. Six do: `sprawling`, `sprawling-agent-protocols`, `sprawling-desktop`, `sprawling-gateway`, `sprawling-remote-access` and `sprawling-runtime`. Their lines are in the `depmap` block of `ARCHITECTURE.md` section 3.

## The specification

`crates/child/Spec.lean` holds the whole specification and has no parts under a `spec/` directory. The package has no state machine, so the file describes the interface and its decisions and proves nothing; the Rust types and `child::environment::tests` hold it. `ARCHITECTURE.md` section 11, *Specifications in Lean*, says how a SPEC is laid out.

## Building and checking

```bash
cargo nextest run -p sprawling-child   # the environment rule's tests
lake build crates.child.Spec           # this package's specification
```

Whether every other spawn point goes through `command` is checked by `cargo clippy --workspace --all-targets --all-features -- -D warnings`, which `just clippy` runs.
