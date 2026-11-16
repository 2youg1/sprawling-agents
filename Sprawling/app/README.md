# sprawling

The city's one binary, and the assembly root: every concrete type, the one wall clock, the one `Hands` value a worker is given, and the command line. What the city decides about its own work belongs to the domain crates — `kernel` holds every decision, `accounting` the one writer, `runtime` one run — and this package decides what this machine has, what a command line means, and how a request reaches the writer. The package is also a library, so a test or the simulator enters the assembly point without going through a process and a socket ([`src/lib.rs`](src/lib.rs)).

No product crate depends on it: dependencies point inward and never back, and this package is the top ([`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 3).

## How it is shaped

Every module names one of the seven shapes in [`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 9, and [`architecture.toml`](../../architecture.toml) records which for each of them, so the table below names the module and the one thing it holds.

| Module | What it owns |
|---|---|
| `bin::main` | the command line. `verbs` is the one table of every verb, its positionals, its flags, its help line and whether running it changes anything; `grammar` reads argv into one invocation or the word it could not read; `router` hands off; `exit` is the closed set of exit codes; `refusal` writes a rejection. The verbs are grouped by subject in `city` (`up`, `init`, `serve`, `resume`), `data` (`adopt`, `enrol`, `export`, `fork`, `install`, `replay`, `restore`, `status`), `calling` and `dispatch` (one frame or one task over the wire), `check`, `view`, `whose`, `playback`, `gauge`, `desktop`, and the `privacy` verbs in `bin::privacy::cli` |
| `bin::assembly` | the assembly root, the only module that knows every concrete type; `bin::doctor` and `bin::serving` never name it back, which the `directions` block of [`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 3 checks. `production` makes the one `Hands` value and holds `SystemClock`; `listening` takes the port and opens the writer before returning `Listening`; `attending` runs the writer thread; `chain_watch` audits the whole history once per open; `dropping`, `remote_door`, `skill_audit` and `privacy` serve the rest of what only the binary can reach |
| `bin::serving` | what a served city is made of before its writer exists, and what it reaches outside through: `door` (the key this serve demands and the vault behind it), `key_file`, `browsers` (the public keys paired at this machine's door), `folding` (the thread the views fold on), `journal` (the one diagnostic sink), `output_ring`, `placement` (the seat a hot thread takes), `standing` (the priority it stands at), `serve` (the value one caller fills in) |
| `bin::doctor` | what this machine has against what a city needs, and the verdict per tier. It is the one authority on that: no other module in this binary reads the search path or an environment variable naming a program |
| `bin::console` | what a served city says to the terminal it runs in and what a line typed there means; `lifecycle` decides which face shows, `ui` draws it, `cli` reads a line |
| `bin::outside` | the remote door wired into this binary: the keeper with its ledger lines and device table, a loopback listener on a port of its own, the conduit that judges each frame by its class, and `/remote` on the console |
| `bin::supervising` | whether a served city that just ended is left down, raised again, or handed to the person |
| `bin::monitor` | whether anybody watches the performance page, and the last 300 samples they read |
| `bin::privacy` | the host privacy page and the one order its writes take: `plan` decides, `coordinator` runs lock, read, prepared, write, readback, rollback and receipt, `journal` keeps the record, `windows` holds the platform adapters |
| `bin::browser_bidi`, `bin::browser_tool` | a WebDriver BiDi engine and its socket, and the `browser` tool whose screenshots land in the content store |
| `bin::firstrun`, `bin::beside_binary` | the first screen a person sees, and the one move of a city an earlier release kept beside the binary |
| `bin::audience`, `bin::keying`, `bin::release`, `bin::revealing`, `bin::environment_broadcast` | who reads stdout, decided once; where a serve's key comes from, as a pure policy; install identity and what a registry offers; an address shown in the file manager; the announcement that the user environment changed (Windows only) |
| `bin::install`, `bin::wire_client` | the binary's own, declared by `main.rs` and not by the library: put this executable where a shell finds it, and talk to a served city from a terminal |
| `build.rs` | the one embed point: gzip the client bundle into `OUT_DIR` and generate `client_embed.rs`, plus the dependency list and the toolchain pins the doctor reports |

Most of these are adapters. The decisions are `bin::doctor`, `bin::console`, `bin::supervising`, `bin::keying` and `bin::privacy::plan`; the state machines are `bin::monitor`, `bin::serving::placement` and `bin::serving::standing`; `bin::main::verbs` and `bin::main::exit` are data, and `bin::audience` is a value.

A served city answers on one listener: the page and its assets through `wire::bundle_routes`, `/ws` for the wire, `/pair` and `/session` for a browser pairing, the privacy page through `bin::assembly::privacy` ahead of the views, with the paired browsers behind `wire::LocalDoor`. The remote door is a listener on a loopback port of its own, opened and closed with the door, and it answers `/remote/pair` and `/remote/session` plus the page the pairing code's address opens.

`bin::assembly` is the injection site: the clock and every handle a worker reaches this machine through pass through it, so a callee takes them as parameters instead of reaching for them. This crate draws entropy in one place, `bin::serving::door`, and `bin::keying` is pure for that reason. A thread starts in the module that serves it; rule 3 of [`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 10 lists every thread that starts outside a library crate.

The seams in [`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 4 that this package implements are `accounting::clock`, whose production adapter is `assembly::production::SystemClock`, and `accounting::machine`, whose production adapter is `bin::doctor::ThisMachine`. The second adapters of `accounting::models`, `accounting::connectors` and `accounting::machine` are the scripted ones under `tests/`. The production adapter of `accounting::views::snapshot::start` is `accounting::views::Views`, folded on the thread `bin::serving::folding` starts.

## What a caller meets when it fails

Every failure is an `AxError`: a failure line, a `recovery:` line, and a `nearby:` line when there are names close to the one that missed. `bin::main::refusal::written` decides the wording in one place: the two lines for a person, or one JSON line in the shape the wire's `Refusal` already has. An agent should read the exit code, because that is the part a script can act on without parsing:

| Code | `Exit` | The fact it asserts |
|---|---|---|
| 0 | `Done` | the verb did what it was asked |
| 1 | `Refused` | the city or this machine refused; the `AxError` lines are on stderr |
| 2 | `Line` | this command line does not parse, including a `call` frame the wire cannot carry |
| 3 | `Quiet` | `call` and `dispatch` sent the frame and heard nothing inside the quiet window, or the city answered and the frame that was waited for did not come |
| 4 | `NoCity` | nothing answered at `--at`: no connection, or no `Welcome` |

The table is `bin::main::exit`, and the model proves the five codes pairwise different (`crates/sprawling/Spec.lean` §8-103). These requests are refused rather than attempted:

- A command line `grammar` cannot read exits 2 and names the nearest verb.
- `serve` on a directory that holds no history, because `up` is the verb that raises a city and `serve` serves one that exists.
- A second `serve` of a city already being served: `E_CONFIG_INVALID` when the port is taken, `E_LEDGER_HELD` when the writer lock is held, and neither writes a ledger line or leaves a port bound (§8-88).
- Tool execution in a build without the `sandbox` feature: `AbsentSandbox` answers `E_TOOL_UNAVAILABLE` with a recovery that names the `wasm` feature to install a build with.
- Opening the remote door, replacing the city key, or confirming a page's request in a serve that has no console: `E_TOOL_UNAVAILABLE`, because nobody could read the code the console prints.
- An `AddAgent` that reaches the run worker instead of the listener: answered that consent is taken at the city's own listener.
- A refusal as one JSON line: `call` takes `--json` and no other verb does, which §8-103 records as the rest of that table still to be written.

## Constraints a reader acts on

**Features.** `sandbox` is a default feature and turns on `runtime/wasm`, so `cargo install sprawling` builds the binary a release archive carries. `--no-default-features` builds one whose tool execution is refused, and `just features` compiles that build so it cannot rot.

**The clock.** `assembly::production::SystemClock` is the single sanctioned sampling point; `clippy.toml` disallows `std::time::Instant::now` and `std::time::SystemTime::now` everywhere else, and each call site inside it carries an `#[expect]` naming that reason. A callee takes `TimeMs` as a parameter or reads the clock it was handed.

**Child processes.** Every child this binary starts is built by `child::command`, which detaches it from the city's console, gives it its own process group and removes the city's secrets from its environment; `clippy.toml` refuses `std::process::Command::new` elsewhere. Decision D73 in the SPEC lists the three exceptions and why each is one.

**No foreign call.** The crate's surface is safe: the processor topology, the power-throttling opt-out and a job's CPU and memory limits arrive from `desktop_ffi`, and each terminal frame from `console_ffi`. Those two are the crates whose lint tables lift `unsafe_code`.

**What it may name.** The `sprawling:` row of the `depmap` block in [`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 3 is the ceiling: `kernel`, `child`, `storage`, `gateway`, `runtime`, `collab`, `city`, `browser`, `agent_protocols`, `wire`, `accounting`, `desktop`, `desktop_ffi`, `console_ffi`, `remote_access`. `documents` is not on that row and is not named here; it arrives through `wire` and `accounting`. Inside the crate, `bin::doctor` and `bin::serving` never name `crate::assembly`, which the `directions` block of the same section holds.

**The allocator.** The static Linux build links `tikv-jemallocator` because musl's `malloc` serialises every thread on one lock; every other target keeps the allocator its platform ships (`crates/sprawling/spec/Serving/Memory.lean` D102).

**Undo is not a verb.** Taking a setting back means sending the command that sets it again, which is what `tests/undoing.rs` exercises.

## Building and running

The compile embeds the client bundle, so build the bundle first:

```bash
just build-web                      # cd client && bun install --frozen-lockfile && bun run build
cargo run -p sprawling -- --help    # the overview, built from the table in src/main/verbs.rs
cargo run -p sprawling -- serve --help
```

`just build-web` writes the bundle to `crates/sprawling/web-dist`, and `BUNDLE_DIR` in `build.rs` is the one declaration of that location. `build.rs` gzips whatever is there into `OUT_DIR`, at a fixed level and with a zeroed mtime, so the same source bytes embed identically on every build. Without the bundle the binary still builds and prints a warning: it carries a placeholder page, `CLIENT_COMPLETE` is false, and `sprawling status` says so. A build of this crate therefore needs no JavaScript toolchain, and `cargo xtask budget` refuses a release binary whose file table holds no client. The `.crate` archive carries `web-dist/**` through the manifest's `include`, because the directory is gitignored build output.

`--version` answers before any verb runs, and `--help` before any verb acts, so a verb that changes something cannot be set off by a request for its help. How a person installs and uses the binary is [`docs/getting-started.md`](../../docs/getting-started.md); how a script drives it is [`docs/wire.md`](../../docs/wire.md); how a running city is operated is [`docs/operating.md`](../../docs/operating.md).

## Who uses it

`citysim` is the one workspace package that depends on it, and it is a development-only tool (`publish = false`): its two bench binaries `bench` and `bench_startup` time the product's own startup and queries, and read `sprawling::monitor::spread`, `sprawling::monitor::tree`, `sprawling::doctor::usable_python_wasm` and `sprawling::assembly::init_city` to do it ([`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 3). No product crate depends on it.

## Specification

The SPEC is [`Spec.lean`](Spec.lean), with its parts under [`spec/`](spec/); [`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 11, *Specifications in Lean*, describes that layout. Section 1 maps each part to the module it specifies; section 3 states the assumptions and what is still ambiguous, and sends each interface's unfinished part to the section that owns it; section 8 holds the interface sections and the table that sends each §8-*n* tag to the part it lives in; section 12 sends each `D<n>` decision to its part; section 16 says which tests pin each section, and that `lake build Spec` proves every model section 2 lists.

A decision is a comment opening with `D<n>`, in the part of its topic, and another document cites it as `sprawling D<n>`. The parts under `spec/Accounting/` specify modules of the `accounting` crate, and D30 says why they live here. The SPEC is written in Chinese with concept names in English, as the *Language* section of [`AGENTS.md`](../../AGENTS.md) requires. Adding a verb means adding a section to the part that owns it and registering the module in [`architecture.toml`](../../architecture.toml) (§17).

## Checking it

```bash
cargo nextest run -p sprawling --all-features   # the tests, as `just test` runs the workspace
lake build crates.sprawling.Spec                # this SPEC and its parts; `just models` builds every crate's
cargo xtask gates modmap depmap spec            # module map, dependency edges and pub traits, one specification per crate
cargo xtask budget                              # sizes inside their budget, and the client inside the release binary
just e2e                                        # dispatch against real endpoints; passes with a line when no key is set
just adversary                                  # attacks this binary over the wire from outside; never a gate
```

The integration tests are under `tests/`. Some start the real binary through `CARGO_BIN_EXE_sprawling` (`remote_door.rs`, `desktop.rs`, `privacy_disposable.rs`); others drive a dispatch against scripted ports through the door `wire::server` hands every frame to (`assembly_door.rs`, `e2e.rs`, and `tests/acceptance/`). `tests/from_adversary.rs` keeps the traces the Lean adversary found, written out as Rust tests and compared against `crates/adversary/`. `just e2e` names the environment variables it reads when it runs, so run it once to learn them.
