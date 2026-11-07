# sprawling-child

The one place the city configures a child process, and the one place that decides which environment keys a child never inherits. `child::command(program)` returns a `std::process::Command` ready for arguments, stdio and `spawn`: off the city's console on Windows (`CREATE_NO_WINDOW`), in a process group of its own on Unix (`process_group(0)`), and with every one of the city's secret keys removed from its environment.

What a caller receives is a configuration, not a running process. The arguments, the three standard streams and the `spawn` call stay with the call site. What the package decides on the call site's behalf is that a child of the city cannot read the screen a pairing code is printed on, the keys a person types into the CLI, or a secret key in the environment; each of those is a property of how a process is created rather than of what it does.

Package `sprawling-child`, library `child`. The empty row for `child` in the `depmap` block of [`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) §3 is the point of it: it names nothing, so every unit that starts a process may name it.

## How it is shaped

Two modules, one decision each:

- `child::command` (adapter) — the constructor. `NO_WINDOW` (`0x0800_0000`, Windows' `CREATE_NO_WINDOW`) is exposed for a call site that needs flags of its own, because `creation_flags` replaces the flag word rather than adding to it: a call site that sets a priority class writes `child::NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS`, and one that forgets loses the detached console silently.
- `child::environment` (decision) — which keys hold the city's secrets, and the rule that takes them out of a child's environment. Two names: `SECRET_PREFIX` (`SPRAWLING_SECRET_`) and `PAIRING_TOKEN` (`SPRAWLING_PAIRING_TOKEN`). Both comparisons are ASCII case-insensitive on Windows, where an environment variable's name has no case, and byte-wise on Unix. `environment::scrubbed` is the rule; `command` calls it with this process's own keys.

The prefix has one home. `gateway::credential::vault` reads `child::SECRET_PREFIX`, so the shape of the environment variable a person exports as a key is decided here and nowhere else.

The crate declares no trait, implements no seam, and runs no thread. A second constructor would be a second policy about consoles and secrets, which is the defect this crate exists to prevent.

## When it fails

`command` returns no `Result` and refuses nothing. It reads environment keys and never their values, so a key whose bytes are not Unicode is still compared and removed, and an empty environment is a legal input. There is no error type to handle and no code to match.

A process that will not start is `spawn`'s own `io::Error`, and it belongs to the call site. What this crate adds is the constraint that a call site which refuses a program says so where it calls `spawn`, next to the arguments it was given.

## Constraints a reader acts on

- No dependencies. The flags are one constant and both extension traits are the standard library's; a platform binding for one constant would put a platform dependency under every spawn point in the workspace.
- `clippy.toml` refuses `std::process::Command::new` across the workspace, and `command` is the only caller that does not need a reason. Every other spawn point carries `#[expect(clippy::disallowed_methods, reason = "…")]` naming which class it belongs to, from D4's list or from the exception §3 records:

  - the supervisor's own child, which stays on the city's console on purpose;
  - a command `gauge` measures, which runs in the person's own terminal and must receive Ctrl+C;
  - the development packages (`xtask`, `citysim`, `benchstat`), whose processes belong to a developer;
  - a build script, which runs inside cargo rather than inside the city;
  - a test fixture process, which is held to exactly the flags its test states.

- A key the caller sets with `env` after `command` is passed on. That is a value the caller chose to hand over, and it is how `agent_protocols` gives an MCP server the names a `[[mcp]]` table declared and the vault redeemed.
- D3: keys are removed by name instead of emptying the environment and re-adding a whitelist, because each call site needs a different environment, and a whitelist would be a second configuration surface per call site.
- `command` is `#[must_use]`: a `Command` configured and dropped has run nothing, and the compiler says so.

## Who uses it

`cargo tree --workspace -i sprawling-child -e normal --depth 1` prints `sprawling`, `sprawling-agent-protocols`, `sprawling-desktop`, `sprawling-gateway`, `sprawling-remote-access` and `sprawling-runtime`. `sprawling-gateway` names it for the one constant above; the others name it for `command`.

## The specification

`crates/child/Spec.lean` is the whole SPEC and has no parts under a `spec/` directory. §1 splits the two units, §2 states the two acceptance criteria, §3 records the one process that is not started through it and the things that are not city children at all, §8 gives the four public items and the flags rule, §11 enumerates the boundary cases of the key rule, and §16 names the test. D1 to D3 are the three decisions on the platform flag, the process group and the key rule; D4 is the ban and its classes.

The package has no state machine, so the SPEC proves nothing and holds no theorem: the Rust types and `child::environment::tests` hold it. `ARCHITECTURE.md` §11, *Specifications in Lean*, says how a SPEC is laid out.

## Building and checking

```bash
cargo nextest run -p sprawling-child   # the environment rule's tests
lake build crates.child.Spec           # this package's specification
```

`environment::tests::every_secret_key_is_removed_and_nothing_else` is the one test: it hands `scrubbed` a set of inherited keys and checks that each secret key is gone, that nothing else moved, and that a differently cased spelling of a secret key is gone on Windows.

Whether every other spawn point in the workspace goes through `command` is judged by `cargo clippy --workspace --all-targets --all-features -- -D warnings`, which `just clippy` runs. That check reads the `#[expect]` lines, so a new spawn point is a red build until it either calls `command` or carries one of them with a reason; that the reason names the right class is a matter for review, and the line's presence is the lint's.
