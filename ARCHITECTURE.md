# ARCHITECTURE — sprawling

> **For someone about to change this code**, and for anyone who wants to know what it is made of and why it has this shape rather than another one.
>
> It answers: what runs, what the stack is and what each choice costs, how the twelve units are wired, what happens end to end when you dispatch one piece of work, what is on disk, what crosses the wire, which parts you can replace, and how the whole thing is verified.
>
> It does not teach the vocabulary ([`docs/glossary.md`](docs/glossary.md)), install anything ([`docs/getting-started.md`](docs/getting-started.md)), or list the rules a change must satisfy ([`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md)).
>
> **Two tables here are machine authorities**: the fenced `depmap` block in §3 and the module map in §12. `cargo xtask depmap` and `cargo xtask modmap` parse them, so an edge or a file that disagrees with them turns CI red. Their column shapes are fixed; the prose around them is not.

## 1 What runs

One process, one page, and the page is embedded in the binary at build time. **Two clients are in the tree at once**, which is the seam in `crates/channels` being exercised rather than argued: `client/` is TypeScript built by bun — npm and node appear nowhere — and `crates/web` is the earlier Dioxus client compiled to WebAssembly, which needs no JavaScript toolchain at all. They coexist until card 6.11 removes the wasm one; until then `just dist` embeds `crates/web`, because `build.rs` judges a bundle complete by `web.js` and `web_bg.wasm`. A third client written against the same wire in any language is a supported thing to build. The gate that once forbade JavaScript source in this tree was removed because it excluded architectures rather than defects.

```
                      one machine
┌──────────────────────────────────────────────────────────────┐
│  sprawling (one binary)                                      │
│                                                              │
│   bin::assembly ── the only omniscient point: it holds every │
│        │           concrete type, samples the clock, hands   │
│        │           out seeds, and starts every task          │
│        │                                                     │
│        ├── runtime ── turns, tools, sandbox, watchdog        │
│        ├── collab  ── signals, drafts, pull requests         │
│        ├── city    ── space, residents, archive, schedule    │
│        ├── eval    ── suites, probes, scoring                │
│        ├── browser ── WebDriver BiDi sessions                │
│        ├── protocol── MCP outbound, ACP inbound              │
│        ├── memory  ── Ledger on disk, CAS, projections, git  │
│        ├── gateway ── model routing, dialects, credentials   │
│        └── channels ─ WebSocket server, Command/Query/Event  │
│                 │                                            │
│                 │  ws://127.0.0.1:8787/ws                    │
│                 ▼                                            │
│          web (wasm, served from inside the binary)           │
└──────────────────────────────────────────────────────────────┘
        │                                    │
        ▼                                    ▼
   the city directory                 model providers, MCP servers
   (a tree on this disk)              (only over one gateway endpoint)
```

## 2 The stack, and what each choice costs

The pinned versions live in `Cargo.toml`; this table says why each is there. Where a choice has a cost, the cost is stated rather than implied.

| Concern | Choice | Why it, and what it costs |
|---|---|---|
| Language | Rust, edition 2024, toolchain pinned in `rust-toolchain.toml`, MSRV 1.97 | The invariants this design cares about are expressible as types, and `#![forbid(unsafe_code)]` holds workspace-wide. Cost: compile times, and a wasm client that has to be built before the binary that embeds it. |
| Async runtime | `tokio` 1, only in `channels` and the binary | The turn loop is synchronous on purpose — a decision that awaits is a decision that interleaves. Async stops at the process boundary. Cost: one blocking HTTP call per model request, paid inside a worker rather than a reactor. |
| HTTP server | `axum` 0.8 with its `ws` feature | It carries the WebSocket implementation itself, so the protocol has one version authority rather than two. |
| HTTP client | `reqwest` 0.13, blocking, `rustls`, no default features | One client for the whole workspace: providers and HTTP-reached MCP servers. Two clients would mean two TLS stacks in one binary. |
| Client | `dioxus` 0.7.10, `default-features = false`, `minimal` + `web`, built **without** `dx` | Rust to `wasm32-unknown-unknown` plus a pinned `wasm-bindgen` CLI. Dropping default features removes the `asset!` macro and devtools, both of which need the CLI we deliberately do not use. Cost: no hot reload; the CLI version must equal the crate version or the build breaks quietly. |
| History | JSONL segments, appended, chain-verified | A history a person can read with `tail` and a machine can verify byte by byte. Cost: the Ledger's throughput is the city's throughput (§11). |
| Cold views | `redb` 4.2 | Embedded, transactional, crash-safe. The projection is derived, so its file is disposable and never a second authority. |
| Content store | BLAKE3 (`blake3` 1.8) | One hash for the whole library: content addressing and `IdemKey` derivation. Identical content is stored once. |
| Restoration | `git2` 0.21, vendored libgit2 | Git is the restoration authority for tracked files, so a discarded file points at a checkpoint commit. Also one worktree per reviewing run. Cost: a C library in the tree, vendored so there is no system dependency. |
| Sandbox | `wasmtime` 48 + `wasmtime-wasi`, wasip1 only | Fuel-metered execution with **no socket host implementation** — the Python arm's mechanical proof that it cannot reach the network. Cost: an optional feature; a build without it refuses tool execution in three parts rather than pretending. |
| Credentials | `keyring` 3 (platform credential service), `secrecy` 0.10, `zeroize` 1.9 | Plaintext lives in the operating system's own vault, never in a file we wrote. `sha2` 0.10 is present for one external protocol fact: PKCE mandates SHA-256. |
| Entropy | `getrandom` 0.3 | OS entropy for the PKCE verifier and the login state. It is *not* the seeded RNG the simulator uses, and must never become it. |
| Serialisation | `serde` 1, `serde_json` 1, `toml` 0.8 | JSON on the wire and in the Ledger because the receiver may be a browser and a person still has to read it. TOML for configuration a person edits. |
| Errors | `thiserror` 2 | One error shape, `AxError`, defined in `kernel::error` and mapped at every crate boundary. |
| Release profile | `lto = "fat"`, one codegen unit, symbols stripped, `panic = "abort"` | Crash-only delivery: there is no unwinding path to maintain, because there is nothing to catch. |
| Dependency count | 497 packages in `Cargo.lock` | Listed by `sprawling status --deps`, licence-checked one by one by `cargo deny` against `deny.toml`. |

**Verification tools**, kept out of the shipped binary: `proptest` (properties before examples), `insta` (golden output), `trybuild` (proof that something cannot be expressed), `kani` (bounded proof, Linux CI), `cargo-mutants` (do the tests bite), `cargo-fuzz` (parsers against hostile bytes).

## 3 Twelve units and the dependency law

Crates are not a reuse mechanism here. They exist so that **the dependency rules are executed by the compiler**: the wall `pub(crate)` builds is drawn at the crate boundary, so twelve crates are twelve walls that actually close.

```
sprawling (bin: init/serve/resume/replay/fork/adopt/export/restore/status)
   ├─ runtime ──→ kernel, memory, gateway   turns, tools, sandbox, watchdog, fork
   ├─ collab  ──→ kernel, memory            inbox, drafts, workshop, fan-in, pull requests
   ├─ city    ──→ kernel                    space, residents, archive, library, schedule
   ├─ eval    ──→ kernel, memory            suites, probes, scoring, metabolism
   ├─ browser ──→ kernel                    WebDriver BiDi sessions, page snapshots
   ├─ protocol──→ kernel                    MCP outbound, ACP inbound
   ├─ memory  ──→ kernel                    Ledger, CAS, projections, attribution, git
   ├─ gateway ──→ kernel                    routing, dialects, market, cost, credentials
   ├─ channels──→ kernel                    Command/Query/Event, WebSocket server, auth
   └─ web     ──→ channels                  the one client, in a browser (wasm)
kernel: no internal dependencies.
```

The block below is what the machine reads. Actual edges must be a **subset** of it, so a hidden edge is a red build rather than a discovery:

```depmap
kernel:
memory: kernel
gateway: kernel
runtime: kernel, memory, gateway
collab: kernel, memory
city: kernel
eval: kernel, memory
browser: kernel
protocol: kernel
channels: kernel
web: channels
sprawling: kernel, memory, gateway, runtime, collab, city, eval, browser, protocol, channels
```

**Three rules.** Dependencies point inward, and never back. A seam declares its trait in the inner layer and implements it in the outer one, so `kernel` can define what a Ledger *is* without knowing where it is written. And splitting into crates buys compiler-enforced layering, not reuse — nothing here is published.

**Three kinds of edge**, and telling them apart is what makes the repository readable:

| Edge | When it exists | Example |
|---|---|---|
| Dependency | compile time | `runtime → kernel` |
| Assembly | run time, only in `bin::assembly` | the upload sink in `channels::server` receiving `memory::cas` |
| Event | anywhere a `kernel::Ledger` handle is held | writing `tool_result` after a tool runs |

`runtime` has the widest fan-out — three crates at once. It may **use** their interfaces and nothing more; the moment a runtime module starts passing concrete types between `memory` and `gateway`, that edge moves up into the assembly layer.

`xtask` and `citysim` are workspace members outside the product graph. **citysim drives the turn loop a second time**: `runtime::run::drive` with simulated adapters — a scripted model, scripted tools, an in-memory Ledger — which is how one seed reproduces a run. It stops below `bin::assembly`, whose `RunWorker` builds its model adapter out of the endpoint book rather than receiving one; the dispatch policy above that line is held by that module's own tests. `sprawling` carries a lib target so the policy is at least *reachable* — an integration test enters by the same door `channels::server` uses — and inverting the model seam is what a seeded scenario would still need.

## 4 Seams

A seam is a trait declared in the inner layer and implemented outside it. **One adapter is a hypothetical seam; two make it real** — so every seam ships with a second implementation, and `cargo xtask depmap` refuses a `pub trait` declared anywhere but the files below.

| Seam | Declared in | Production adapter | Second adapter |
|---|---|---|---|
| `kernel::ledger` | crates/kernel/src/ledger.rs | memory: jsonl segments with tail recovery | citysim: in-memory Ledger |
| `kernel::tool` | crates/kernel/src/tool.rs | runtime tools, collab tools, browser, protocol | citysim: scripted tools |
| `kernel::model` | crates/kernel/src/model.rs | gateway: native and endpoint | citysim: scripted model |
| `runtime::sandbox` | crates/runtime/src/sandbox.rs | wasmtime with fuel metering | pass-through and fault doubles |
| `browser::port` | crates/browser/src/port.rs | WebDriver BiDi session layer | recording and replay adapter |
| `protocol::mcp` | crates/protocol/src/mcp/outbound.rs | stdio child process, or HTTP | `ScriptedOutbound` for offline replay |

**Two inner seams** stay `pub(crate)` because nothing outside their crate needs them: `memory`'s `Vfs` (real filesystem / deterministic power-loss model) and `gateway`'s `Vault` (platform credential service / in-session store).

**Deliberately not seams**: `gateway::dialect` is a pure function and needs no trait; the internals of `city`, `collab` and `eval` have one implementation each and are driven from outside by citysim; `git2` and `redb` are used directly, because an interface with one implementation is decoration.

## 5 One dispatch, end to end

This is the path everything else supports. Following it once explains more than any diagram of boxes.

1. **The page sends a Command.** A person fills in the control surface — address, what to produce, what counts as done — and `web::socket` sends `Command::Dispatch` over the WebSocket. The frame carries no ceiling of any kind: nobody can price a piece of work before it runs, and the one brake is `Halt`.
2. **`channels::server` decides whether to accept it.** Two pure judgements — may this address be bound, may this peer be accepted — with the socket code that surrounds them making no judgement at all.
3. **`bin::assembly` turns it into work.** This is the only place that samples the clock, so the timestamp enters as a parameter from here on. A worker takes the dispatch, and answers every refusal it can owe before it writes anything: the reserved subtree, a halted scope, rules that will not load, and a tag with no model behind it are all decided by `agree_to_work`, which reads and writes nothing. **Opening the room is the first thing this city puts on disk for a dispatch**, so work nobody could take leaves no room behind for a person to find.
4. **The city writes `run_started` before anything happens.** Every effect becomes an event first; that ordering is the design's load-bearing rule, not a logging preference.
5. **`runtime::prefix` assembles the frozen prefix** in four segments — city, building, resident, run — from `city::spine_files`, `city::policy` and `city::resident`. Assembling it is itself an event, and the result is frozen for the whole run.
6. **`runtime::catalog` decides what the model may see**: the three built-in tools, the collaboration tools this building admits, the skills its reading room allows, and any MCP tools discovered from the building's `CONFIG.toml`. `city::neighbourhood` is scanned in the same breath, so the run also knows which addresses it can reach and who stands at them — without it, `signal` takes an address the model has to have been told.
7. **`runtime::turn` enters its typestate**: Assembling → Calling → Applying → Settling, with four cancellation-safe points. An interruption inside a phase cannot be spelled.
8. **`gateway` makes the call.** `gateway::router` picks the endpoint attached to this tag; `gateway::dialect` translates the canonical Anthropic-shaped conversation into the provider's dialect; `gateway::credential` redeems a `secret:realm/name` reference into a header at the last moment; `gateway::admission` holds the provider's concurrency limit.
9. **The reply is scanned before it is recorded.** `runtime::redact` puts model output through the same secret scan as everything else, so a key a model repeated does not become permanent.
10. **Tools run behind gates.** `kernel::gate` answers with an exhaustive verdict — allowed, refused in three parts, or escalated to a person. `memory::checkpoint` puts a git fence before the wave and scans the worktree after it, so anything that disappeared becomes a `file_discarded` event carrying the way back.
11. **The result comes back shaped.** `runtime::pipeline` builds the result envelope — clock stamp, network reminder, any steer a person sent — and `runtime::compaction` shortens what is too long, always reporting how much it dropped.
12. **Everything lands in the Ledger, and the views follow.** `memory::hot`, `memory::projection` and `memory::attribution` fold the same event stream into what the pages ask for. The server pushes each event; `web::app` folds it into a `Snapshot`. The same fold, on both sides of the wire.

13. **A signal reaches whoever it names, working or not.** After the run freezes, each signal it sent is recorded and then delivered. A steer-kind signal slips under the door of a run that is already going, landing at that run's next safe point with `@` and the sender's address in front of it; anyone else who was spoken to is *knocked* — `bin::assembly` starts a run for them, whose brief names the resident who spoke. Only the person's own entrance can render as `user`, which is what makes an answer go to the right place. A knock addresses a resident, never a frozen run: history is read, not woken.

When the process dies mid-call, `sprawling resume` verifies the chain, closes tool calls whose outcome was lost as *unknown* rather than as failed, and reports what waits for a person.

## 6 On disk

A city is one directory. Copy it and it is the same city; delete it and nothing outside it changes.

```
<city>/
├─ .sprawling/                 the city's own reserved subtree
│  ├─ ledger/                  the only history — jsonl segments, appended, chain-verified
│  ├─ cas/                     content-addressed store, BLAKE3, one copy per content
│  ├─ views/                   redb: cold projections, disposable, rebuilt from the ledger
│  ├─ worktrees/               one git worktree per reviewing run, objects shared
│  ├─ staging/                 uploads land here read-only, never in a worktree
│  ├─ library/                 skills more than one building admits
│  └─ CONFIG.toml              city layer of the three-layer configuration
└─ <building>/                 one building, one line of business
   ├─ .sprawling/              the building's reserved subtree: what governs it (F2.09, F2.10)
   │  ├─ BUILDING.md           the building's rules: confidential, review, write domains
   │  ├─ CONFIG.toml           building layer, including its MCP servers
   │  └─ skills/               skills only this building admits
   ├─ Roadmap.md               the plan tree, and the denominator of every progress reading
   ├─ Memo.md                  decisions and corrections
   ├─ Handoff.md               what the next session needs
   └─ <room>/                  one session's workplace, named by the person who started it
      ├─ URBANITE.md           who this resident is and how it works
      └─ JOB.md                the task for this session
```

**One rule, applied at every scope: what governs a scope lives in that scope's `.sprawling/`, and no write domain reaches it.** `is_reserved` answers true for an address with `.sprawling` in any segment, so the check is one predicate in `kernel::address` rather than a list of protected file names. An agent therefore cannot edit its own accounting, its own configuration, its own building's rules, or the history of what it did.

A run's write domain is what its building's `BUILDING.md` declares, and **the whole building when it declares nothing** — which is the shipped template. The room is where a session works, not the boundary that contains it.

**`Roadmap.md` is a tree, and the `plan` tool is what writes it.** The index column is a path — `2.3.1` hangs under `2.3` — so one file states a multi-level plan without a second file to say how the levels relate. `Weight` is a ratio among the rows sharing a parent and `Needs` names what must finish first, which is what makes a ready set computable. A resident divides its own branch and cannot reach past it: `kernel::share` has no constructor, so a share exists only by dividing another one, and the total is the whole plan whatever the plan turns into. Only leaves are counted; a branch's work is its children.

## 7 The wire

One WebSocket, three kinds of frame, and a schema hash that both ends check on connect: a page from a different build refuses rather than misreads. `WIRE_V` is 15.

| Frame | Count | What it is |
|---|---|---|
| `Command` | 24 | something a person wants done: dispatch, steer, cancel, approve, halt, raise a building, attach an endpoint, set a goal the city works towards, write a document that governs the city |
| `Query` | 17 | something a page wants to know: the city, one run, approvals, cost, the ledger, archive, discards, inboxes, which run wrote a commit, who answers and what was answered for the person, and one file's patch text |
| `Delta` | — | what a model is saying while it is still saying it: no sequence number, never written down, and a client that missed one has lost nothing |
| `Event` | the Ledger's own kinds | what happened, pushed as it happens |

Two properties are worth stating because they are enforced by types rather than by review. A `Command` carrying a credential **cannot be serialised**: `Sealed<T>` has no `Serialize`, and the `PutSecret` payload of the remote command type is an uninhabited type, so entering a key over the network is not a request that can be spelled. And every `Query` must be answered exhaustively — the answer match has no catch-all, so adding a query without answering it does not compile.

## 8 Parts you can replace

This repository bundles nobody's key, pays for nothing, and proxies nothing. Everything that reaches outside is therefore an adapter you can swap, and this section says where each one lives.

### Provider intelligence — followed from codex and pi

Signing in to a provider means knowing four things: authorization endpoint, token endpoint, client id, scopes. Those are facts, and they change without warning, so they are followed from two actively maintained projects rather than watched by hand: [`openai/codex`](https://github.com/openai/codex) (Apache-2.0) for OpenAI, [`earendil-works/pi`](https://github.com/earendil-works/pi) (MIT) for Anthropic and the rest. **What is followed is intelligence, not code** — see [`docs/third-party.md`](docs/third-party.md) for the obligations and how to re-check. That file's §3 also lists the thirty-one crates this workspace names and the licence each one declares; the full resolved graph is 413 packages and belongs to `cargo deny` and the CycloneDX bill of materials, not to a table in a document.

| To do this | Change this |
|---|---|
| add or correct a subscription provider | `gateway::oauth_profiles` — a table with data and zero branches |
| change how a login is begun, finished or renewed | `gateway::credential` |
| use an API key instead | the settings page: base URL, dialect, key |
| speak a third dialect | `gateway::dialect`, a pure two-way translation with the canonical shape in the middle |
| run a local model | `gateway::native` — local inference never goes through the outbound gateway |

**What you cannot move out**: credential custody. Plaintext reaches the platform credential service and nothing else, configuration holds a `secret:realm/name` reference, and that is part of what the product promises rather than an implementation detail.

### Outside applications — MCP, and Composio as one server among many

Mail, GitHub, Figma, Discord: writing an integration for each is a weekly chore unrelated to the problem here, so the whole class is outsourced over **MCP**. [Composio](https://composio.dev) is the first choice and is reached the same way any other server is — this code never knows what Composio is.

| To do this | Change this |
|---|---|
| give a building tools from a server | its `CONFIG.toml`: a `command` starts a child process, a `url` reaches a hosted server |
| point at a different provider of the same tools | the same URL field. Nothing else changes |
| add a transport | `bin::mcp_stdio` and `bin::mcp_http` are the two adapters behind `protocol::mcp`'s `Outbound` seam |
| drive this city from an editor | `protocol::acp` accepts an outside request as an ordinary dispatch |

A confidential building constructs none of them: data may enter and may not leave.

### The rest

| Part | Seam or surface | Note |
|---|---|---|
| execution sandbox | `runtime::sandbox` | implement the trait, pass its conformance suite; the shipped adapter is wasmtime with fuel |
| the client | `channels::wire` | the wire is the whole API; a second client writes against it |
| the browser driver | `browser::port` | frames in, replies out; the shipped adapter speaks WebDriver BiDi |
| where views are stored | `memory::projection` | delete the store and it rebuilds from the Ledger, byte-identical |

## 9 Seven shapes

Every module instantiates exactly one of these. The classification earns its place by what it catches: a module that cannot name its shape usually holds two things that want to be separate files.

| # | Shape | The test for it |
|---|---|---|
| 1 | decision | no I/O, no clock, no global state; returns an exhaustive enum, never a bool |
| 2 | value | invariants enforced at one construction point; private fields; no setters |
| 3 | port | a trait on a seam, delivered with its conformance suite |
| 4 | adapter | thin, no policy: swapping in a second implementation changes no policy |
| 5 | typestate | phase changes enforced by types; no method returns a previous phase |
| 6 | data | data only, no branches. Editing it is editing behaviour |
| 7 | projection | folds the event stream into a view; deleting it and rebuilding gives the same bytes |

**The Humble Object is the recurring move**: the hard-to-test end is stripped to nothing and the thick end stays pure. `web::app`, `runtime::watchdog`, `gateway::endpoint`, `memory::projection` and the sandbox adapters are all instances of it.

### Making illegal states unrepresentable

Each of these is a type, not a slogan, and each has a compile-failure counterexample in the test suite — because "unrepresentable" is itself a claim that needs testing.

**These expectations are byte comparisons against a compiler's output, so the machine is part of them.** Installing the `rust-src` component makes rustc render a source snippet inside a `note:` that the committed `.stderr` files do not carry, and every counterexample using one goes red without a line of this repository changing. `cargo public-api` pulls that component in, so `just api-baseline` can turn `just check` red on the next run; remove it (`rustup component remove rust-src`) rather than blessing the longer output, which would only move the failure to CI.

- `EventRef` has no public constructor and no serde ⇒ **a forged event reference cannot be spelled**.
- `Completion::Done` always carries `Evidence`, and `Completion` has no serde ⇒ **a deserialised "finished" cannot be spelled**.
- A `Delegate` value has no `delegate` method ⇒ **a grand-delegate cannot be spelled**; delegation is one level deep.
- `Discard`'s constructor requires a `Restoration` ⇒ **a deletion with no way back cannot be spelled**.
- `Sealed<T>` has no `Serialize` ⇒ **entering a credential remotely cannot be spelled**.
- `UnplannedProgress` has no `ratio` method ⇒ **a percentage with no denominator cannot be drawn**; there is nothing to call.
- `Share` has no constructor and no arithmetic; the only producer is `Share::split`, which consumes what it divides ⇒ **weight cannot be minted**, so a branch cannot be given more than its parent had and the total is always the whole plan.
- `Held` has one private field and two consuming methods ⇒ **a claimed plan node cannot be put down silently**: it is finished with evidence or stopped with a cause, and a run that simply ends spends it on `FrozeWithoutEvidence`.
- `Pursuit::declare` takes the depth-zero position and a `Delegate` cannot produce one ⇒ **a sub-agent cannot set the city working until the work runs out**.

## 10 Determinism and hardening

The whole city runs as real code, single-threaded, seed-driven, on a virtual clock. That is not a testing convenience — it is the property that makes a failure reproducible from one seed, and these seven rules are its admission conditions.

| # | Rule | Held by |
|---|---|---|
| 1 | Decision paths iterate `BTreeMap`; never a hash order | review, plus the citysim determinism scenarios |
| 2 | Time arrives as a parameter; the one sampling point is `bin::assembly` | `clippy.toml` disallowed methods |
| 3 | One spawn point | review; the two exceptions are runtime's concurrent wave, with structured cancellation, and the driving lanes in `bin::serving::pool`, each of which lives exactly as long as the run it drives |
| 4 | Seeded RNG handed out from one place | assembly derives per session |
| 5 | Execute in parallel, account in series, ordered by `seq` | the Ledger port owns `seq` and `prev` |
| 6 | Ledger payloads hold integers; timestamps are integer milliseconds; field order is declaration order | cross-OS byte fixtures |
| 7 | `IdemKey` derives from `(run, seq, normalised action)` — never from a clock or a random number | property tests |

Rules 1 and 6 together give a checkable property: **the same event sequence replays byte-for-byte identically on any machine.**

**Hardening** is compile-time, workspace-wide, and identical in tests and production except where a test module relaxes it locally: no `unwrap`, `expect`, `panic!`, `todo!`, `unreachable!`, bare indexing or slicing; arithmetic is checked; narrowing goes through `TryFrom`; `as` casts are denied; `unsafe_code` is forbidden outright. Money and quantities are integer newtypes (`UsdMicros`, `Tokens`, `ByteLen`, `Seq`), and floats stay out of every decision path.

**The most fragile point in the design is one paragraph long.** A database handle, one `Instant::now()`, or a bare spawn inside `kernel` disables replay, formal verification and deterministic simulation at the same time. The gates hold that line so the property survives a builder who has never read this file.

## 11 How this is verified, and what it costs

Eleven layers, each catching what the layer above cannot. They deliberately do not overlap: overlapping verification reads as more coverage than it is.

| Layer | Catches | Today |
|---|---|---|
| V0 unrepresentable | a whole class of error moved out of what can be written | 15 compile-failure counterexamples |
| V1 types and lints | null, overflow, silent truncation, hidden panics | workspace lints, `-D warnings`, `--all-features` |
| V2 unit and property | a function wrong across a class of inputs | 1,085 tests, properties before examples |
| V3 conformance | a second adapter behaving unlike the first | one suite per port |
| V4 fuzz | parsers meeting hostile bytes | three targets: address, locator, truncated ledger tail |
| V5 formal | termination, absence of overflow, monotonicity | 2 of 10 kani harnesses, Linux CI — those with an unbounded domain and a solvable one |
| V6 deterministic simulation | components each correct and wrong together | citysim, six scenario files, failures reproduced from a seed |
| V7 mutation | tests that do not bite | `cargo-mutants`, by `just mutants` |
| V8 cross-version, cross-OS fixtures | byte drift after an upgrade or a platform change | golden ledgers in `fixtures/` |
| V9 end to end | the thing a person actually wants to do | the real client in a real browser against a real server, on a developer machine |
| V10 adversarial | a promise the door makes that holds on the traces we wrote and not on the ones we did not | `adversary/`, out of tree, in Haskell, driving the shipped binary over the wire |

**V10 is not a gate, and the difference is load-bearing.** Section 8 says the wire is the whole API and that a second client writes against it; `adversary/` exercises that permission by writing a third one outside the workspace, in another language, to attack rather than to use. It is reached by `just adversary` and by a schedule, never by `just check` — on a machine with no Haskell toolchain, `just check` behaves byte for byte as it does where the directory is absent, and `just adversary` prints one line and succeeds. What it buys that V2 cannot is quantification over traces: V2 proves that the paths we thought of hold, and V10 asks whether the door's stable error codes survive any prefix, one halt, and any suffix. It has already found three things a specific trace would not have — an exit code that means "no refusal arrived in time" where its own documentation promises "the city refused", a dispatch that writes a job file to disk before the guard that refuses it runs, and an `IdemKey` the wire makes every state-changing command carry that `kernel::gate::dedup` checks and nothing calls. All three are recorded in `adversary/adversary-SPEC.md` section 4; the second is fixed here (card V3.51), the other two are open.

**Four gaps, named rather than hidden.** CI has no browser driver, so V9 is a command a developer runs rather than a gate. The isometric city compares display lists rather than bitmaps: the preconditions for bitmap comparison are paid for — placement is a pure function of the id, painter order is total, projection and its inverse are exact — but there is no rasteriser. And V6 stops below `bin::assembly` (§3): a seeded scenario reproduces a run, not a dispatch, because `RunWorker` builds its model adapter instead of receiving one. What holds the dispatch policy is that module's own tests, plus the integration tests the lib target makes possible. And eight of the ten kani harnesses are written but not proved, for two different reasons. Seven take concrete input, so each states what the `#[test]` beside it already states — twice over a wider domain — and those same seven build the `Vec`, `String` or `BTreeSet` whose loops CBMC cannot bound: they burned six hours, then forty-five minutes under a bounded unwind, and returned nothing either time. The eighth, `secret::entropy_is_total_on_short_inputs`, has a real domain but an unsolvable shape: some 2,560 symbolic non-linear multiplications, one per slot of a 256-slot count table times ten squarings inside `log2_q10`. CI proves the two that have both an unbounded domain and a tractable one, in under a minute each (card-11.7 deleted the third, `budget::admit_spend`, along with the ceiling it decided); closing either gap means rewriting a harness — symbolic without a heap collection, or over one slot rather than the table — rather than waiting longer (P4.05, P4.06).

### Four times a gate changed the design

Evidence that the mechanism pays for itself. In none of them was the gate loosened.

1. **The secret gate caught an `.expose()` on a boundary.** `ServeConfig` now takes a digest only, so `channels` cannot obtain the pairing token in plaintext at all — which also removed a length side channel.
2. **The secret gate caught a token's display form.** `PairingToken` now holds only a hash. Sealing a value and unsealing it on the next line is theatre; not holding it is the property that was wanted.
3. **The colour gate found a contradiction between two documents.** One set the brightest step at L=0.930 while a token table placed a hover variant at 0.945. The reading that makes both legal is that the bound delimits the greyscale information surface, and interactive variants sit above it by design.
4. **The public-surface gate caught "just one more `pub use` line" three times.** Re-exporting one item is a public-surface change, and to a builder it feels like it is not.

### The performance register

Sizes are gated because a byte count does not depend on how busy the machine was. Wall-clock figures are measured, reported with the machine that produced them, and never gated: a slow runner is not a defect, and a gate that says it is teaches people to ignore gates. The full register is `xtask/budgets.toml`; `cargo xtask budget` prints it.

| Metric | Budget | Measured | Gated |
|---|---|---|---|
| Client bundle, gzipped | ≤2 MB | 558,419 B — 3.8× headroom | yes |
| The installed binary | ≤128 MB | 8,620,032 B, client included | yes |
| Resident memory, one session | ≤30 MB | 4.3 MB idle, 7.8 MB running a real run | no: the counter means something different on each platform |
| Ledger append plus fsync | p50 ≤5 ms, p99 ≤20 ms | 0.97 ms / 1.61 ms on one NVMe machine | no |
| Projection rebuild | ≥50,000 records/s | about 493,000 records/s on the same machine | no |
| Prefix assembly | ≤1 ms | 0.022 ms for 16.5 KB over four slots | no |
| Resident segment, catalog included | none stated | 815 B on one real dispatch, with thirteen tools admitted | no: what a building admits is the building's, and a reading room with more in it is not a defect |
| Runs driving at once | 4 lanes | one thread per run, and one accounting thread taking every write | no: it is a wall this city sets, not a measurement |
| Kernel mutation score | ≥90% | by `just mutants` | by that command, not by `just check` |

The two size rows are also rendered as the badges in `README.md`, from this same reading — `cargo xtask badge --write`, which `just dist` ends with. Nobody types a size into a document.

**One honest trade.** With network and model time removed, the throughput ceiling of a city is the throughput ceiling of its Ledger. That is the price of "the Ledger is the only history", stated in the open. The first wall is one this city sets itself: a building working towards a goal drives four runs at a time, because a lane past the provider's own admission ceiling would only park a thread there. Then come the walls outside: provider-side rate limits (a few dozen concurrent calls), Ledger fsync, worktree disk, file-descriptor limits, then the blocking pool. RAM is not among them. **Runs are driven in parallel and accounted for in series** — every line a lane writes crosses to the one thread that owns the Ledger and waits there — so concurrency buys model time back and buys nothing from the Ledger.

## 12 Module map

The machine's data face, parsed by `cargo xtask modmap`: a `.rs` file under `crates/*/src` that is not in this table turns CI red, and so does a row whose file is missing. `lib.rs` and pure index files are exempt because they hold no logic — that too is checked.

Columns are fixed: **Module | File | What it owns | Shape** (§9) **| Since** (the construction stage that introduced it: S0–S5 skeleton, P1–P4 product, R1 repair, F1 front end, P5–P7 documents, measurement and delivery) **| Status** (`planned`, `building`, `built`, `frozen`) **| Spec** (`<crate>-SPEC.md#8-N`: the section of that crate's SPEC where this module's interface is settled).

**The Spec column is checked, not decorative.** `cargo xtask specalign` resolves every anchor and turns CI red when the section is not there, because a column of links to nothing costs a reader more than no column at all. A submodule points at its parent's section unless a section names its own full path — the SPEC divides by interface and this table divides by file, and the two were never one to one. Section numbers repeat inside several SPECs, so the anchor is checked for existence and not for uniqueness; `xtask-SPEC.md` §8-10 records what would let that tighten.

**The number in each subheading is the number of rows under it**, and `cargo xtask modmap` counts them, because every count a person maintained by hand here had already gone stale. The `desktop` heading is the one exception the machine cannot judge: its files sit outside `crates/`, where the parser does not look.

### kernel (74) — every decision in the city, and nothing that touches a disk

| Module | File | What it owns | Shape | Since | Status | Spec |
|---|---|---|---|---|---|---|
| kernel::address | crates/kernel/src/address.rs | canonical relative paths, write-domain primitive, reserved prefix | value | S1 | built | kernel-SPEC.md#8-2 |
| kernel::locator | crates/kernel/src/locator.rs | the one grammar for referring to content: `cas:` and `file:`, fail-closed | value | S1 | built | kernel-SPEC.md#8-3 |
| kernel::locator::tests | crates/kernel/src/locator/tests.rs | the grammar's round-trip and fail-closed cases | value | S1 | built | kernel-SPEC.md#8-3 |
| kernel::ledger (port) | crates/kernel/src/ledger.rs | the only write entrance to history; owns `seq` and `prev` | port | S1 | built | kernel-SPEC.md#8-9 |
| kernel::event | crates/kernel/src/event.rs | EventRecord, the closed EventKind set, and the unforgeable EventRef | value | S1 | built | kernel-SPEC.md#8-4 |
| kernel::event::identity | crates/kernel/src/event/identity.rs | run, sequence, and time | value | S1 | built | kernel-SPEC.md#8-4 |
| kernel::event::kind | crates/kernel/src/event/kind.rs | the closed kind set and its window classes | value | S1 | built | kernel-SPEC.md#8-4 |
| kernel::event::payload | crates/kernel/src/event/payload.rs | payloads, drafts, records, and refs | value | S1 | built | kernel-SPEC.md#8-4 |
| kernel::error | crates/kernel/src/error.rs | AxError and the closed AxCode set, each with its carrier event | value | S1 | built | kernel-SPEC.md#8-1 |
| kernel::version | crates/kernel/src/version.rs | optimistic concurrency: a write carries the version it read | value | S1 | built | kernel-SPEC.md#8-5 |
| kernel::idem | crates/kernel/src/idem.rs | the deduplication key for outward actions, derived deterministically | value | S1 | built | kernel-SPEC.md#8-6 |
| kernel::consts_external | crates/kernel/src/consts_external.rs | constants that follow the outside world, each with its source | data | S1 | built | kernel-SPEC.md#8-7 |
| kernel::consts_policy | crates/kernel/src/consts_policy.rs | constants that are our choice; changing one needs evidence | data | S1 | built | kernel-SPEC.md#8-8 |
| kernel::gate | crates/kernel/src/gate.rs | the five doors, idempotent dedup, and refusal in three parts | decision | S2 | built | kernel-SPEC.md#8-27 |
| kernel::gate::domain | crates/kernel/src/gate/domain.rs | the Domain door: writes land inside the write domain | decision | S2 | built | kernel-SPEC.md#8-27 |
| kernel::gate::egress | crates/kernel/src/gate/egress.rs | the Egress door and its allowlist | decision | S2 | built | kernel-SPEC.md#8-27 |
| kernel::gate::commitment | crates/kernel/src/gate/commitment.rs | the Commitment door: no decision pre-blocks the run | decision | S2 | built | kernel-SPEC.md#8-27 |
| kernel::gate::govern | crates/kernel/src/gate/govern.rs | the Discard, Delegation and Govern doors | decision | S2 | built | kernel-SPEC.md#8-27 |
| kernel::gate::dedup | crates/kernel/src/gate/dedup.rs | idempotent dedup, judged before any side effect | decision | S2 | built | kernel-SPEC.md#8-27 |
| kernel::gate::item | crates/kernel/src/gate/item.rs | the one Escalate item mint, shared by the doors | decision | S2 | built | kernel-SPEC.md#8-27 |
| kernel::gate::undoable | crates/kernel/src/gate/undoable.rs | which connector tools reach effects nothing here can take back, and the person that puts in front of | decision | V4 | built | kernel-SPEC.md#8-49 |
| kernel::taint | crates/kernel/src/taint.rs | outside content is data: union propagation, no unwrapping surface | value | S2 | built | kernel-SPEC.md#8-10 |
| kernel::write_domain | crates/kernel/src/write_domain.rs | which prefixes a resident may write, and edit-war detection | decision | S2 | built | kernel-SPEC.md#8-11 |
| kernel::budget | crates/kernel/src/budget.rs | money and tokens as integers, and what a run turned out to cost | value | S2 | built | kernel-SPEC.md#8-12 |
| kernel::backpressure | crates/kernel/src/backpressure.rs | the city-wide shedding posture: admit or shed, with a reason | decision | S2 | built | kernel-SPEC.md#8-13 |
| kernel::stall | crates/kernel/src/stall.rs | the sole criterion for "this run is going nowhere" | decision | S2 | built | kernel-SPEC.md#8-14 |
| kernel::goal | crates/kernel/src/goal.rs | two goals wanting the same resource | decision | S2 | built | kernel-SPEC.md#8-15 |
| kernel::spine | crates/kernel/src/spine.rs | the Roadmap table's grammar: six columns, a dotted index, and the one editing entrance | decision | S2 | built | kernel-SPEC.md#8-19 |
| kernel::spine::row | crates/kernel/src/spine/row.rs | the row types: status, cells, rows, shapes, children | decision | S2 | built | kernel-SPEC.md#8-19 |
| kernel::spine::grammar | crates/kernel/src/spine/grammar.rs | locating the table and reading its rows | decision | S2 | built | kernel-SPEC.md#8-19 |
| kernel::spine::rewrite | crates/kernel/src/spine/rewrite.rs | status changes and child insertions | decision | S2 | built | kernel-SPEC.md#8-19 |
| kernel::spine::rewrite::tests | crates/kernel/src/spine/rewrite/tests.rs | the rewrite's fixtures and byte-stability | decision | S2 | built | kernel-SPEC.md#8-19 |
| kernel::spine::memo | crates/kernel/src/spine/memo.rs | the six memo fields and their write moments | decision | S2 | built | kernel-SPEC.md#8-19 |
| kernel::share | crates/kernel/src/share.rs | how much of the plan one node is; a share exists only by dividing another, so weight cannot be minted | value | V3 | built | kernel-SPEC.md#8-32 |
| kernel::node_id | crates/kernel/src/node_id.rs | a plan node's address: the dotted path that says where it hangs | value | V3 | built | kernel-SPEC.md#8-48 |
| kernel::plan | crates/kernel/src/plan.rs | the plan as a tree: what hangs where, what each is worth, what may be started, and the two exits of a held node | decision | V3 | built | kernel-SPEC.md#8-33 |
| kernel::plan::node | crates/kernel/src/plan/node.rs | the node types: StopCause, Held, PlanExit, PlanNode | decision | V3 | built | kernel-SPEC.md#8-33 |
| kernel::plan::tree | crates/kernel/src/plan/tree.rs | placement, division, queries, claims, and progress | decision | V3 | built | kernel-SPEC.md#8-33 |
| kernel::plan::tree::tests | crates/kernel/src/plan/tree/tests.rs | the tree's test fixtures and refusals | decision | V3 | built | kernel-SPEC.md#8-33 |
| kernel::plan::share | crates/kernel/src/plan/share.rs | giving a level's pot to its rows by weight | decision | V3 | built | kernel-SPEC.md#8-33 |
| kernel::plan::blocking | crates/kernel/src/plan/blocking.rs | what a node waits for, and the circles it waits in | decision | V3 | built | kernel-SPEC.md#8-33 |
| kernel::blockage | crates/kernel/src/blockage.rs | red, and how far it reaches: one cause named rather than every symptom listed | decision | V3 | built | kernel-SPEC.md#8-34 |
| kernel::pursuit | crates/kernel/src/pursuit.rs | a goal the city works towards, and the one condition under which it stops | decision | V3 | built | kernel-SPEC.md#8-35 |
| kernel::completion | crates/kernel/src/completion.rs | done requires evidence; progress has two states and no third | value | S2 | built | kernel-SPEC.md#8-20 |
| kernel::registry | crates/kernel/src/registry.rs | the three books: artifact, asset, skill | value | S2 | built | kernel-SPEC.md#8-18 |
| kernel::approval | crates/kernel/src/approval.rs | what waits for a person, its cluster key, and a policy that expires | value | S2 | built | kernel-SPEC.md#8-21 |
| kernel::approval::tests | crates/kernel/src/approval/tests.rs | the three rules a caller depends on: policy matching, idle expiry, and who may answer | value | S2 | built | kernel-SPEC.md#8-21 |
| kernel::delegation | crates/kernel/src/delegation.rs | two kinds of delegate, one level deep, no grand-delegate | value | S2 | built | kernel-SPEC.md#8-17 |
| kernel::repair | crates/kernel/src/repair.rs | leases for when the environment itself is broken | decision | S2 | built | kernel-SPEC.md#8-16 |
| kernel::config | crates/kernel/src/config.rs | three-layer resolution, and the frozen/live split with no shared field | decision | S2 | built | kernel-SPEC.md#8-22 |
| kernel::config::tests | crates/kernel/src/config/tests.rs | the ladder rules a caller depends on: lower layers win, whole-list override, and the frozen/live field disjointness | decision | S2 | built | kernel-SPEC.md#8-22 |
| kernel::tool (port) | crates/kernel/src/tool.rs | what a tool is, in eight fields | port | S2 | built | kernel-SPEC.md#8-23 |
| kernel::tool::tests | crates/kernel/src/tool/tests.rs | the grammars a name and a label are held to, and the eight fields on the wire | port | S2 | built | kernel-SPEC.md#8-23 |
| kernel::model (port) | crates/kernel/src/model.rs | what a model call is, carrying the building's policy with it | port | S2 | built | kernel-SPEC.md#8-24 |
| kernel::model::wire | crates/kernel/src/model/wire.rs | the canonical conversation vocabulary a request and a response are made of | port | S2 | built | kernel-SPEC.md#8-24 |
| kernel::model::image | crates/kernel/src/model/image.rs | what a picture is on the canonical conversation: a `cas:` locator, its media type, and integer dimensions | value | V4 | built | kernel-SPEC.md#8-24 |
| kernel::model::seam | crates/kernel/src/model/seam.rs | what one call carries each way, and content-to-payload conversion | port | S2 | built | kernel-SPEC.md#8-24 |
| kernel::model::conformance | crates/kernel/src/model/conformance.rs | the assertion suite every model implementation answers | port | S2 | built | kernel-SPEC.md#8-24 |
| kernel::model::tests | crates/kernel/src/model/tests.rs | the model seam fixtures | port | S2 | built | kernel-SPEC.md#8-24 |
| kernel::secret | crates/kernel/src/secret.rs | secret-shape judgement, the `secret:` grammar, and `Sealed<T>` | decision | S2 | built | kernel-SPEC.md#8-25 |
| kernel::secret::span | crates/kernel/src/secret/span.rs | references and spans: the shapes that name secrets | decision | S2 | built | kernel-SPEC.md#8-25 |
| kernel::secret::scan | crates/kernel/src/secret/scan.rs | shape-table-first, entropy-second, float-free | decision | S2 | built | kernel-SPEC.md#8-25 |
| kernel::secret::sealed | crates/kernel/src/secret/sealed.rs | plaintext that cannot reach any sink | decision | S2 | built | kernel-SPEC.md#8-25 |
| kernel::discard | crates/kernel/src/discard.rs | deletion as an effect class; a Discard without a Restoration cannot exist | decision | S2 | built | kernel-SPEC.md#8-26 |
| kernel::discard::request | crates/kernel/src/discard/request.rs | restoration routes and planned/unplanned shapes | decision | S2 | built | kernel-SPEC.md#8-26 |
| kernel::discard::verdict | crates/kernel/src/discard/verdict.rs | the decision table | decision | S2 | built | kernel-SPEC.md#8-26 |
| kernel::discard::forecast | crates/kernel/src/discard/forecast.rs | reading a command whole before it runs | decision | S2 | built | kernel-SPEC.md#8-26 |
| kernel::error::code | crates/kernel/src/error/code.rs | the closed code set and its carrier events | decision | S1 | built | kernel-SPEC.md#8-1 |
| kernel::error::refusal | crates/kernel/src/error/refusal.rs | the three mandatory parts | decision | S1 | built | kernel-SPEC.md#8-1 |
| kernel::error::shape | crates/kernel/src/error/shape.rs | the one error shape of the whole city | decision | S1 | built | kernel-SPEC.md#8-1 |
| kernel::change | crates/kernel/src/change.rs | what moved between two checkpoints; a binary file cannot be spelled as one that moved nothing | value | R2 | built | kernel-SPEC.md#8-30 |
| kernel::highlight | crates/kernel/src/highlight.rs | a document read as ordered, disjoint spans; it says where things are and never rewrites the text | decision | R2 | built | kernel-SPEC.md#8-31 |
| kernel::highlight::tests | crates/kernel/src/highlight/tests.rs | the lexical rules a reader depends on: precedence, fences, and spans that slice without overlapping | decision | R2 | built | kernel-SPEC.md#8-31 |
| kernel::schema | crates/kernel/src/schema.rs | the JSON Schema of the five values whose serde is hand-written, so the client generated from the wire reads their strings the way the parser does | adapter | V4 | built | kernel-SPEC.md#8-45 |

### memory (45) — persistence, and every view derived from it

| Module | File | What it owns | Shape | Since | Status | Spec |
|---|---|---|---|---|---|---|
| memory::jsonl | crates/memory/src/jsonl.rs | the durable Ledger: segments, chain verification, tail recovery, group commit | adapter | S1 | built | memory-SPEC.md#8-1 |
| memory::jsonl::ledger | crates/memory/src/jsonl/ledger.rs | the ledger types and segment grammar | adapter | S1 | built | memory-SPEC.md#8-1 |
| memory::jsonl::open | crates/memory/src/jsonl/open.rs | opening, version probes, tail recovery | adapter | S1 | built | memory-SPEC.md#8-1 |
| memory::jsonl::open::tests | crates/memory/src/jsonl/open/tests.rs | the opening fixtures | adapter | S1 | built | memory-SPEC.md#8-1 |
| memory::jsonl::append | crates/memory/src/jsonl/append.rs | waves, reads, and the kernel Ledger face | adapter | S1 | built | memory-SPEC.md#8-1 |
| memory::vfs | crates/memory/src/vfs.rs | the one face this crate touches a filesystem through; inner seam, two adapters | port | V3 | built | memory-SPEC.md#8-15 |
| memory::real_fs | crates/memory/src/real_fs.rs | std::fs, holding the handle it is appending through | adapter | V3 | built | memory-SPEC.md#8-16 |
| memory::error | crates/memory/src/error.rs | what persistence says when it refuses, and the one door out to `AxError` | value | V3 | built | memory-SPEC.md#8-14 |
| memory::cas | crates/memory/src/cas.rs | content-addressed storage under BLAKE3, written through a temporary file | adapter | S1 | built | memory-SPEC.md#8-3 |
| memory::fault_fs | crates/memory/src/fault_fs.rs | the second filesystem adapter: a deterministic power-loss model | adapter | S1 | built | memory-SPEC.md#8-2 |
| memory::fault_fs::plan | crates/memory/src/fault_fs/plan.rs | which write dies, and how | adapter | S1 | built | memory-SPEC.md#8-2 |
| memory::fault_fs::fs | crates/memory/src/fault_fs/fs.rs | power cuts on demand | adapter | S1 | built | memory-SPEC.md#8-2 |
| memory::fault_fs::fs::tests | crates/memory/src/fault_fs/fs/tests.rs | the power-cut matrix | adapter | S1 | built | memory-SPEC.md#8-2 |
| memory::index | crates/memory/src/index.rs | seq to byte offset; disposable, rebuilt when damaged | projection | S3 | built | memory-SPEC.md#8-4 |
| memory::index::ledger | crates/memory/src/index/ledger.rs | the side index map | projection | S3 | built | memory-SPEC.md#8-4 |
| memory::index::ledger::tests | crates/memory/src/index/ledger/tests.rs | the index fixtures | projection | S3 | built | memory-SPEC.md#8-4 |
| memory::index::reader | crates/memory/src/index/reader.rs | seeking lines without scanning | projection | S3 | built | memory-SPEC.md#8-4 |
| memory::index::cache | crates/memory/src/index/cache.rs | stamps, caches, rebuilds | projection | S3 | built | memory-SPEC.md#8-4 |
| memory::hot | crates/memory/src/hot.rs | the in-memory view the interface reads without touching disk | projection | S3 | built | memory-SPEC.md#8-5 |
| memory::projection | crates/memory/src/projection.rs | the cold view: questions too big for memory, and recovery after restart | projection | S3 | built | memory-SPEC.md#8-6 |
| memory::projection::tables | crates/memory/src/projection/tables.rs | rows, folds, table grammar | projection | S3 | built | memory-SPEC.md#8-6 |
| memory::projection::view | crates/memory/src/projection/view.rs | open, apply, read | projection | S3 | built | memory-SPEC.md#8-6 |
| memory::projection::view::tests | crates/memory/src/projection/view/tests.rs | the view fixtures | projection | S3 | built | memory-SPEC.md#8-6 |
| memory::attribution | crates/memory/src/attribution.rs | where the money went, in five independent cuts that reconcile | projection | S3 | built | memory-SPEC.md#8-7 |
| memory::attribution::report | crates/memory/src/attribution/report.rs | reports and buckets | projection | S3 | built | memory-SPEC.md#8-7 |
| memory::attribution::report::tests | crates/memory/src/attribution/report/tests.rs | the attribution fixtures | projection | S3 | built | memory-SPEC.md#8-7 |
| memory::attribution::split | crates/memory/src/attribution/split.rs | dividing a bill by weight | projection | S3 | built | memory-SPEC.md#8-7 |
| memory::checkpoint | crates/memory/src/checkpoint.rs | git fences around a tool wave, and what disappeared between them | adapter | S3 | built | memory-SPEC.md#8-17 |
| memory::checkpoint::fence | crates/memory/src/checkpoint/fence.rs | base, wave pre/post | adapter | S3 | built | memory-SPEC.md#8-17 |
| memory::checkpoint::fence::tests | crates/memory/src/checkpoint/fence/tests.rs | the fence fixtures | adapter | V4 | built | memory-SPEC.md#8-17 |
| memory::checkpoint::scan | crates/memory/src/checkpoint/scan.rs | staged secrets and scoped commits | adapter | S3 | built | memory-SPEC.md#8-17 |
| memory::checkpoint::provenance | crates/memory/src/checkpoint/provenance.rs | which session a commit came out of, as five git trailers | value | V4 | built | memory-SPEC.md#8-17 |
| memory::changes | crates/memory/src/changes.rs | what moved between two checkpoints, as paths and counts and never as patch text | adapter | R2 | built | memory-SPEC.md#8-13 |
| memory::hunks | crates/memory/src/hunks.rs | one file's patch text between two checkpoints, with every line a credential shape matched withheld and named | adapter | V4 | built | memory-SPEC.md#8-19 |
| memory::worktree | crates/memory/src/worktree.rs | one node, one working tree, objects shared and files not | adapter | P2 | built | memory-SPEC.md#8-9 |
| memory::worktree::name | crates/memory/src/worktree/name.rs | one segment, no escape | adapter | P2 | built | memory-SPEC.md#8-9 |
| memory::worktree::lease | crates/memory/src/worktree/lease.rs | a held tree and its measure | adapter | P2 | built | memory-SPEC.md#8-9 |
| memory::worktree::trees | crates/memory/src/worktree/trees.rs | claim, merge, release | adapter | P2 | built | memory-SPEC.md#8-9 |
| memory::worktree::trees::tests | crates/memory/src/worktree/trees/tests.rs | the worktree fixtures | adapter | P2 | built | memory-SPEC.md#8-9 |
| memory::queue | crates/memory/src/queue.rs | one queue implementation serving three lanes | value | S3 | built | memory-SPEC.md#8-10 |
| memory::digest_cache | crates/memory/src/digest_cache.rs | the same bytes summarised once in their lifetime | projection | S3 | built | memory-SPEC.md#8-11 |
| memory::bundle | crates/memory/src/bundle.rs | export and restore; the manifest is the completeness test | adapter | P1 | built | memory-SPEC.md#8-12 |
| memory::bundle::manifest | crates/memory/src/bundle/manifest.rs | manifests and layout constants | adapter | P1 | built | memory-SPEC.md#8-12 |
| memory::bundle::export | crates/memory/src/bundle/export.rs | export, manifest, restore | adapter | P1 | built | memory-SPEC.md#8-12 |
| memory::bundle::files | crates/memory/src/bundle/files.rs | walking, counting, copying | adapter | P1 | built | memory-SPEC.md#8-12 |

### gateway (34) — everything between a decision to call a model and the bytes on the wire

| Module | File | What it owns | Shape | Since | Status | Spec |
|---|---|---|---|---|---|---|
| gateway::router | crates/gateway/src/router.rs | the book of attached endpoints and the model chosen per tag | projection | P1 | built | gateway-SPEC.md#8-9 |
| gateway::router::attached | crates/gateway/src/router/attached.rs | registration records | projection | P1 | built | gateway-SPEC.md#8-9 |
| gateway::router::book | crates/gateway/src/router/book.rs | choices the city made | projection | P1 | built | gateway-SPEC.md#8-9 |
| gateway::router::payload | crates/gateway/src/router/payload.rs | references on the wire | projection | P1 | built | gateway-SPEC.md#8-9 |
| gateway::dialect | crates/gateway/src/dialect.rs | which dialect answers a question, and the closed set of two | decision | S3 | built | gateway-SPEC.md#8-1 |
| gateway::dialect::request | crates/gateway/src/dialect/request.rs | one ChatRequest, each wire | decision | S3 | built | gateway-SPEC.md#8-1 |
| gateway::dialect::response | crates/gateway/src/dialect/response.rs | frames back to ChatResponse | decision | S3 | built | gateway-SPEC.md#8-1 |
| gateway::dialect::images | crates/gateway/src/dialect/images.rs | the bytes behind the pictures one request refers to, and their base64 form | value | V4 | built | gateway-SPEC.md#8-1 |
| gateway::anthropic | crates/gateway/src/anthropic.rs | the Anthropic Messages wire, in both directions | decision | V3 | built | gateway-SPEC.md#8-1 |
| gateway::anthropic::stream | crates/gateway/src/anthropic/stream.rs | what one Anthropic stream settles into | decision | V4 | built | gateway-SPEC.md#8-1 |
| gateway::openai | crates/gateway/src/openai.rs | the OpenAI Chat Completions wire, and every loss the translation takes | decision | V3 | built | gateway-SPEC.md#8-1 |
| gateway::openai::stream | crates/gateway/src/openai/stream.rs | what one OpenAI stream settles into | decision | V4 | built | gateway-SPEC.md#8-1 |
| gateway::mismatch | crates/gateway/src/mismatch.rs | reading a provider's JSON, and the one word for a shape we did not ask for | decision | V3 | built | gateway-SPEC.md#8-1 |
| gateway::native | crates/gateway/src/native.rs | local inference, which never leaves the machine | adapter | S3 | built | gateway-SPEC.md#8-3 |
| gateway::endpoint | crates/gateway/src/endpoint.rs | the external provider: a self-written wire format over one HTTP client | adapter | S3 | built | gateway-SPEC.md#8-2 |
| gateway::endpoint::config | crates/gateway/src/endpoint/config.rs | auth, overrides, construction | adapter | S3 | built | gateway-SPEC.md#8-2 |
| gateway::endpoint::auth | crates/gateway/src/endpoint/auth.rs | which header a credential travels in, chosen by the compatible format | decision | V4 | built | gateway-SPEC.md#8-2 |
| gateway::endpoint::call | crates/gateway/src/endpoint/call.rs | one request, streamed or settled | adapter | S3 | built | gateway-SPEC.md#8-2 |
| gateway::endpoint::redemption | crates/gateway/src/endpoint/redemption.rs | what one endpoint redeems at the wire: the credential and the pictures | adapter | V4 | built | gateway-SPEC.md#8-2 |
| gateway::endpoint::model | crates/gateway/src/endpoint/model.rs | the Model face | adapter | S3 | built | gateway-SPEC.md#8-2 |
| gateway::endpoint::adapter | crates/gateway/src/endpoint/adapter.rs | which adapter a chosen model gets, and why | adapter | S3 | built | gateway-SPEC.md#8-2 |
| gateway::oauth_profiles | crates/gateway/src/oauth_profiles.rs | subscription-login intelligence: data only, zero branches | data | S3 | built | gateway-SPEC.md#8-5 |
| gateway::admission | crates/gateway/src/admission.rs | the provider's concurrency limit and a deterministic minimum interval | decision | S3 | built | gateway-SPEC.md#8-6 |
| gateway::fallback | crates/gateway/src/fallback.rs | what a tag does when its endpoint will not answer, and the payload the retreat itself is | value | V4 | built | gateway-SPEC.md#8-11 |
| gateway::market | crates/gateway/src/market.rs | the model catalogue snapshot, pinned so a price cannot move under a run | value | S3 | built | gateway-SPEC.md#8-7 |
| gateway::cost | crates/gateway/src/cost.rs | per-call settlement, with the provider's own figure preferred | decision | S3 | built | gateway-SPEC.md#8-8 |
| gateway::credential | crates/gateway/src/credential.rs | custody: capture, replace with a reference, redeem at the wire, renew before expiry | adapter | S3 | built | gateway-SPEC.md#8-4 |
| gateway::credential::vault | crates/gateway/src/credential/vault.rs | vaults, backends, persistence | adapter | S3 | built | gateway-SPEC.md#8-4 |
| gateway::credential::custodian | crates/gateway/src/credential/custodian.rs | capture, resolve, rotate | adapter | S3 | built | gateway-SPEC.md#8-4 |
| gateway::credential::oauth | crates/gateway/src/credential/oauth.rs | PKCE, redeem, refresh | adapter | S3 | built | gateway-SPEC.md#8-4 |
| gateway::credential::oauth::codec | crates/gateway/src/credential/oauth/codec.rs | base64url, percent-encoding, randomness | adapter | S3 | built | gateway-SPEC.md#8-4 |
| gateway::credential::oauth::flow | crates/gateway/src/credential/oauth/flow.rs | begin, redeem, refresh | adapter | S3 | built | gateway-SPEC.md#8-4 |
| gateway::credential::oauth::flow::tests | crates/gateway/src/credential/oauth/flow/tests.rs | the oauth fixtures | adapter | S3 | built | gateway-SPEC.md#8-4 |
| gateway::credential::oauth::types | crates/gateway/src/credential/oauth/types.rs | pending, request, tokens | adapter | S3 | built | gateway-SPEC.md#8-4 |

### runtime (58) — one run, from dispatch to freeze

| Module | File | What it owns | Shape | Since | Status | Spec |
|---|---|---|---|---|---|---|
| runtime::turn | crates/runtime/src/turn.rs | the turn typestate: four phases, four cancellation-safe points | typestate | S2 | built | runtime-SPEC.md#8-3 |
| runtime::turn::boundary | crates/runtime/src/turn/boundary.rs | what the executor supplies at a phase change, and what a phase change answers | typestate | S2 | built | runtime-SPEC.md#8-3 |
| runtime::turn::report | crates/runtime/src/turn/report.rs | what a completed turn hands the run loop, and the call shape it was given | typestate | S2 | built | runtime-SPEC.md#8-3 |
| runtime::turn::wave | crates/runtime/src/turn/wave.rs | boundary 3: the tool wave, executed in call order | typestate | S2 | built | runtime-SPEC.md#8-3 |
| runtime::turn::tests::helpers | crates/runtime/src/turn/tests/helpers.rs | the ledger, model and prefix fixtures the turn tests share | typestate | S2 | built | runtime-SPEC.md#8-3 |
| runtime::turn::tests::phases | crates/runtime/src/turn/tests/phases.rs | the four boundaries driven against a real ledger chain | typestate | S2 | built | runtime-SPEC.md#8-3 |
| runtime::turn::tests::window | crates/runtime/src/turn/tests/window.rs | what the opening lines and a steer put in the window | typestate | S2 | built | runtime-SPEC.md#8-3 |
| runtime::bench | crates/runtime/src/bench.rs | which door one tool call goes through, in which order, and what a refusal becomes | decision | V3 | built | runtime-SPEC.md#8-19 |
| runtime::bench::admit | crates/runtime/src/bench/admit.rs | which gate one call's declared Effect names, and what that gate's verdict means to the bench | decision | V3 | built | runtime-SPEC.md#8-19 |
| runtime::bench::tests | crates/runtime/src/bench/tests.rs | the bench's ordering, door and dedup fixtures | decision | V3 | built | runtime-SPEC.md#8-19 |
| runtime::window | crates/runtime/src/window.rs | the run's conversation history: the volatile half of a request | value | V3 | built | runtime-SPEC.md#8-3 |
| runtime::prefix | crates/runtime/src/prefix.rs | frozen prefix assembly in four segments, each hashed | decision | S2 | built | runtime-SPEC.md#8-4 |
| runtime::prefix::tests | crates/runtime/src/prefix/tests.rs | what the frozen prefix guarantees: slot order, determinism, dedup, truncation markers, cache breakpoints | decision | S2 | built | runtime-SPEC.md#8-4 |
| runtime::handoff | crates/runtime/src/handoff.rs | freezing and resuming: the five-section artifact and its one construction point | value | S2 | built | runtime-SPEC.md#8-5 |
| runtime::transcript | crates/runtime/src/transcript.rs | what one run actually saw, scanned, pinned and written beside its room as `<run-id>.jsonl` | value | V4 | built | runtime-SPEC.md#8-32 |
| runtime::reminder | crates/runtime/src/reminder.rs | how full the window is, from the provider's own count: two thresholds, each sounding once per run | decision | V4 | built | runtime-SPEC.md#8-34 |
| runtime::fork | crates/runtime/src/fork.rs | a new run whose in-window history is a byte-identical prefix of another | decision | S1 | built | runtime-SPEC.md#8-2 |
| runtime::compaction | crates/runtime/src/compaction.rs | when to shorten something and what to keep; never larger than its input | decision | P3 | built | runtime-SPEC.md#8-14 |
| runtime::redact | crates/runtime/src/redact.rs | what a model said, scanned on its way into history | decision | P3 | built | runtime-SPEC.md#8-14 |
| runtime::sieve | crates/runtime/src/sieve.rs | what survives a command's output, by which command produced it: seven stages in a fixed order, each accepted only when it shrinks, and a tee in front so nothing cut is unreachable | decision | V4 | built | runtime-SPEC.md#8-27 |
| runtime::sieve::key | crates/runtime/src/sieve/key.rs | which command produced an output: arm, program, arguments, ordered so it can key a `BTreeMap` | value | V4 | built | runtime-SPEC.md#8-27 |
| runtime::sieve::record | crates/runtime/src/sieve/record.rs | what the sieve answers with: the text the model sees, the way back, and the account of every stage | value | V4 | built | runtime-SPEC.md#8-27 |
| runtime::sieve::filter | crates/runtime/src/sieve/filter.rs | the filter table: its TOML shape, the three built-in reducers, whole-value override across the ladder, and which filter a command key hits | data | V4 | built | runtime-SPEC.md#8-27 |
| runtime::sieve::scan | crates/runtime/src/sieve/scan.rs | which lines matter and how much, by four hand-written scans; and the spans no stage may touch | decision | V4 | built | runtime-SPEC.md#8-27 |
| runtime::sieve::stages | crates/runtime/src/sieve/stages.rs | the line-level passes: ANSI stripped, blank runs folded, templates counted, long lines cut, the middle kept by priority | decision | V4 | built | runtime-SPEC.md#8-27 |
| runtime::sieve::diff | crates/runtime/src/sieve/diff.rs | what this run already saw from the same command, and only what changed since | decision | V4 | built | runtime-SPEC.md#8-27 |
| runtime::sieve::tests | crates/runtime/src/sieve/tests.rs | one golden per built-in filter, the two properties, and the stage account | decision | V4 | built | runtime-SPEC.md#8-27 |
| runtime::replay | crates/runtime/src/replay.rs | offline replay: re-verify without re-executing | decision | S1 | built | runtime-SPEC.md#8-1 |
| runtime::replay::tests | crates/runtime/src/replay/tests.rs | what offline verification refuses: future versions, unknown kinds, drifted prefix sources, dangling calls | decision | S1 | built | runtime-SPEC.md#8-1 |
| runtime::digest | crates/runtime/src/digest.rs | what a long document looks like from outside, summarised once | decision | P1 | built | runtime-SPEC.md#8-16 |
| runtime::digest::tests | crates/runtime/src/digest/tests.rs | what a digest promises a reader: heading trees that skip code fences, prose that stays suspect, one digest per content hash, and a breaker that reopens | decision | P1 | built | runtime-SPEC.md#8-16 |
| runtime::pipeline | crates/runtime/src/pipeline.rs | the result envelope and the order in which a result is shrunk | decision | S3 | built | runtime-SPEC.md#8-7 |
| runtime::pipeline::exec | crates/runtime/src/pipeline/exec.rs | one `exec` result as the model reads it: stdout and stderr sieved under their command key, the way back beside them, every other field untouched | decision | V4 | built | runtime-SPEC.md#8-7 |
| runtime::pipeline::tests | crates/runtime/src/pipeline/tests.rs | the shrink order and the three attachments, through the pipeline's own door | decision | V4 | built | runtime-SPEC.md#8-7 |
| runtime::offload | crates/runtime/src/offload.rs | the shared shrink primitive: lossy but restorable, four invariants | decision | S3 | built | runtime-SPEC.md#8-8 |
| runtime::watchdog | crates/runtime/src/watchdog.rs | disposal in order: correct, then stall, then freeze | decision | S3 | built | runtime-SPEC.md#8-9 |
| runtime::backlog | crates/runtime/src/backlog.rs | the work still running while a run goes on: background children, the ten-second window that decides which of them go there, and the one place they are stopped | adapter | V4 | built | runtime-SPEC.md#8-28 |
| runtime::backlog::member | crates/runtime/src/backlog/member.rs | what one member is made of: a child process with its output on disk, or a run that a halt marks and that stops itself | adapter | V4 | built | runtime-SPEC.md#8-28 |
| runtime::sandbox (port) | crates/runtime/src/sandbox.rs | the execution boundary: capabilities in, outcome out | port | S3 | built | runtime-SPEC.md#8-13 |
| runtime::sandbox::tests | crates/runtime/src/sandbox/tests.rs | what the two stand-ins promise a caller: stdin echoed back with the job recorded, and a fault script delivered in order then spent | port | S3 | built | runtime-SPEC.md#8-13 |
| runtime::catalog | crates/runtime/src/catalog.rs | progressive disclosure: which tools and skills a run is told about | decision | S3 | built | runtime-SPEC.md#8-11 |
| runtime::mode | crates/runtime/src/mode.rs | the modes a run may sit in, and what each admits | decision | S3 | built | runtime-SPEC.md#8-12 |
| runtime::clock | crates/runtime/src/clock.rs | formatting an injected instant; it never samples one | value | S3 | built | runtime-SPEC.md#8-10 |
| runtime::tools::exec | crates/runtime/src/tools/exec.rs | the exec tool: three arms, each with its own failure story | adapter | S3 | built | runtime-SPEC.md#8-26 |
| runtime::tools::exec::tests | crates/runtime/src/tools/exec/tests.rs | what each arm promises a caller: a missing component refuses by name, sandbox exits arrive as themselves, an unknown arm is never guessed | adapter | S3 | built | runtime-SPEC.md#8-14 |
| runtime::tools::chosen_path | crates/runtime/src/tools/chosen_path.rs | the one judgement a model-chosen path gets: it parses as an address, and it does not reach a reserved subtree | decision | V4 | built | runtime-SPEC.md#8-30 |
| runtime::tools::read | crates/runtime/src/tools/read.rs | the read tool: a path the reserved subtree closes, or a name the reading room opens, by the line interval the caller asked for | adapter | P6 | built | runtime-SPEC.md#8-29 |
| runtime::tools::read::tests | crates/runtime/src/tools/read/tests.rs | what a read promises a caller: the two doors, the reserved refusal, and a truthful total beside a truncated interval | adapter | V4 | built | runtime-SPEC.md#8-14 |
| runtime::tools::search | crates/runtime/src/tools/search.rs | the search tool: one substring over a bounded subtree, with context lines and no pattern engine | adapter | V4 | built | runtime-SPEC.md#8-30 |
| runtime::tools::search::tests | crates/runtime/src/tools/search/tests.rs | what a search promises a caller: a hit with its context and its line number, a reserved prefix refused, a binary file passed over | adapter | V4 | built | runtime-SPEC.md#8-14 |
| runtime::tools::edit | crates/runtime/src/tools/edit.rs | the edit tool: optimistic concurrency against the version the caller read | adapter | S3 | built | runtime-SPEC.md#8-22 |
| runtime::tools::edit::tests | crates/runtime/src/tools/edit/tests.rs | the edit tool's fixtures: versions, refusals, the create arm | adapter | S3 | built | runtime-SPEC.md#8-14 |
| runtime::tools::status | crates/runtime/src/tools/status.rs | the model's view of its own situation, in thirteen fields | adapter | S3 | built | runtime-SPEC.md#8-14 |
| runtime::tools::status::tests | crates/runtime/src/tools/status/tests.rs | the frozen order, the children line, and the backlog line, through the tool's own door | adapter | V4 | built | runtime-SPEC.md#8-14 |
| runtime::tools::succeed | crates/runtime/src/tools/succeed.rs | the face succession shows a model: ask to be replaced at the same address and depth, and the desk that holds the ask | adapter | V4 | built | runtime-SPEC.md#8-33 |
| runtime::run | crates/runtime/src/run.rs | the run driver: dispatch, turns, freeze — one authority for the loop | typestate | P1 | built | runtime-SPEC.md#8-15 |
| runtime::run::lifecycle | crates/runtime/src/run/lifecycle.rs | what an active run does: the dispatch pair, one turn, and the freeze that is its only exit | typestate | P1 | built | runtime-SPEC.md#8-15 |
| runtime::diagnostics | crates/runtime/src/diagnostics.rs | the diagnostic log: write-only, five levels, anchored to a Ledger position | adapter | P1 | built | runtime-SPEC.md#8-17 |

### collab (28) — several residents in one building, without stepping on each other

| Module | File | What it owns | Shape | Since | Status | Spec |
|---|---|---|---|---|---|---|
| collab::inbox | crates/collab/src/inbox.rs | signals between residents: at-least-once, deduplicated before any effect | decision | P2 | built | collab-SPEC.md#8-1 |
| collab::inbox::signal_id | crates/collab/src/inbox/signal_id.rs | a signal id: what a duplicate delivery is recognised by | decision | P2 | built | collab-SPEC.md#8-1 |
| collab::inbox::signal_kind | crates/collab/src/inbox/signal_kind.rs | what kind of communication a signal is, and its wire name | decision | P2 | built | collab-SPEC.md#8-1 |
| collab::inbox::tests | crates/collab/src/inbox/tests.rs | deduplication, lane order, bandwidth and the two records, through the production door | decision | P2 | built | collab-SPEC.md#8-1 |
| collab::draft | crates/collab/src/draft.rs | what happens when the room moved while you were writing | typestate | P2 | built | collab-SPEC.md#8-3 |
| collab::draft::tests | crates/collab/src/draft/tests.rs | holds, hold tokens, the four ways back and escalation, through the production door | typestate | P2 | built | collab-SPEC.md#8-3 |
| collab::steer | crates/collab/src/steer.rs | speaking into a run that is already working, at its next safe point | decision | P2 | built | collab-SPEC.md#8-2 |
| collab::workshop | crates/collab/src/workshop.rs | one creation split into nodes, and the order they run in | decision | P2 | built | collab-SPEC.md#8-4 |
| collab::workshop::tests | crates/collab/src/workshop/tests.rs | deterministic scheduling, the fan-out, and the three graphs construction refuses | decision | P2 | built | collab-SPEC.md#8-4 |
| collab::fanin | crates/collab/src/fanin.rs | where the branches come back together, verified before they merge | decision | P2 | built | collab-SPEC.md#8-5 |
| collab::pr | crates/collab/src/pr.rs | an implementer cannot verify their own work — a compile error, not a rule | typestate | P2 | built | collab-SPEC.md#8-6 |
| collab::arbiter | crates/collab/src/arbiter.rs | who decides when two goals collide, and how far up it goes | decision | P2 | built | collab-SPEC.md#8-7 |
| collab::signal_tool | crates/collab/src/signal_tool.rs | the face the inbox shows a model: send and pull | adapter | P3 | built | collab-SPEC.md#8-8 |
| collab::signal_tool::tests | crates/collab/src/signal_tool/tests.rs | what sending queues, what a pull leaves behind, and that the lent inbox comes back | adapter | P3 | built | collab-SPEC.md#8-8 |
| collab::delegate_tool | crates/collab/src/delegate_tool.rs | the face delegation shows a model: one level down, and the desk that remembers what was asked | adapter | P1 | built | collab-SPEC.md#8-7 |
| collab::handback | crates/collab/src/handback.rs | what a run is told about work it handed down, and who is allowed to say it finished | decision | P1 | built | collab-SPEC.md#8-7 |
| collab::goal_tool | crates/collab/src/goal_tool.rs | the face goal detection and arbitration show a model | adapter | P3 | built | collab-SPEC.md#8-9 |
| collab::pr_tool | crates/collab/src/pr_tool.rs | the face pull requests show a model: open, list, check | adapter | P3 | built | collab-SPEC.md#8-10 |
| collab::pr_tool::request | crates/collab/src/pr_tool/request.rs | the request a resident offers, and the record a rebuild reads back | adapter | P3 | built | collab-SPEC.md#8-10 |
| collab::pr_tool::tests | crates/collab/src/pr_tool/tests.rs | the tests of the pull request desk and its tool | adapter | P3 | built | collab-SPEC.md#8-10 |
| collab::archive_tool | crates/collab/src/archive_tool.rs | writing something down so the next run need not be told twice | adapter | P4 | built | collab-SPEC.md#8-20 |
| collab::claim_effect | crates/collab/src/claim_effect.rs | what a claim on a plan node left behind, and whether the file still agrees with it | value | V3 | built | collab-SPEC.md#8-21 |
| collab::claim_tool | crates/collab/src/claim_tool.rs | the face `Roadmap.md` shows a model: one claimed row at a time | adapter | P4 | built | collab-SPEC.md#8-12 |
| collab::claim_tool::tool | crates/collab/src/claim_tool/tool.rs | the six actions, their schema, and the arguments they are spelled with | adapter | P4 | built | collab-SPEC.md#8-12 |
| collab::claim_tool::tests | crates/collab/src/claim_tool/tests.rs | the plan-desk fixtures | adapter | P4 | built | collab-SPEC.md#8-12 |
| collab::workshop_tool | crates/collab/src/workshop_tool.rs | the face a workshop shows a model: lay out, ask the join, judge it | adapter | P1 | built | collab-SPEC.md#8-16 |
| collab::workshop_tool::tests | crates/collab/src/workshop_tool/tests.rs | what the workshop tool refuses: a cycle, a second graph, a verdict from nobody who read | adapter | P1 | built | collab-SPEC.md#8-16 |
| collab::triage | crates/collab/src/triage.rs | where something from outside lands, and whether it starts work | decision | P3 | built | collab-SPEC.md#8-11 |

### city (27) — space, identity, and the documents a building keeps

| Module | File | What it owns | Shape | Since | Status | Spec |
|---|---|---|---|---|---|---|
| city::building | crates/city/src/building.rs | which building governs an address, and how a new one comes into being | decision | P2 | built | city-SPEC.md#8-3 |
| city::building::tests | crates/city/src/building/tests.rs | creation, adoption and refusal, read back through the city's own parser | decision | P2 | built | city-SPEC.md#8-3 |
| city::resident | crates/city/src/resident.rs | standing identity: an address plus the file that says who lives there | value | P1 | built | city-SPEC.md#8-4 |
| city::spine_files | crates/city/src/spine_files.rs | the documents a building keeps its long work in, and the job file a run reads | adapter | P2 | built | city-SPEC.md#8-5 |
| city::spine_files::hall | crates/city/src/spine_files/hall.rs | the two identity files City Hall's residents are read from, in the city's own reserved subtree | adapter | V4 | built | city-SPEC.md#8-5 |
| city::governed | crates/city/src/governed.rs | the three documents that govern a city, where each one lives, and the one point they are written through | adapter | V4 | built | city-SPEC.md#8-24 |
| city::spine_files::tests | crates/city/src/spine_files/tests.rs | the laid-out documents, the job file, and the two briefs a session can carry | adapter | P2 | built | city-SPEC.md#8-5 |
| city::archive | crates/city/src/archive.rs | what a building remembers between runs, indexed by computing it | projection | P3 | built | city-SPEC.md#8-9 |
| city::library | crates/city/src/library.rs | the city's stock of settled work, and the reading room each building admits | decision | P3 | built | city-SPEC.md#8-8 |
| city::config_layers | crates/city/src/config_layers.rs | the three configuration files a run is governed by | decision | P2 | built | city-SPEC.md#8-4 |
| city::config_layers::write | crates/city/src/config_layers/write.rs | writing a chosen value back into the layer that governs it | decision | P2 | built | city-SPEC.md#8-4 |
| city::config_layers::tests | crates/city/src/config_layers/tests.rs | the ladder, the refusals, and the write-back round trip | decision | P2 | built | city-SPEC.md#8-4 |
| city::policy | crates/city/src/policy.rs | `BUILDING.md` evaluated into rules a machine can hold | decision | P1 | built | city-SPEC.md#8-2 |
| city::policy::evaluate | crates/city/src/policy/evaluate.rs | reading a `BUILDING.md` into the rules a machine holds | decision | V4 | built | city-SPEC.md#8-2 |
| city::policy::reach | crates/city/src/policy/reach.rs | what a building's residents may write inside their prefixes: everything, or documents | decision | V4 | built | city-SPEC.md#8-2 |
| city::policy::tests | crates/city/src/policy/tests.rs | the confidential three, the refusals, and the write domain a file declares | decision | P1 | built | city-SPEC.md#8-2 |
| city::rules_tool | crates/city/src/rules_tool.rs | the face a building's rules show a model: read them, propose the whole of them | adapter | P2 | built | city-SPEC.md#8-2 |
| city::schedule | crates/city/src/schedule.rs | work that starts by itself, counted in whole minutes | decision | P2 | built | city-SPEC.md#8-6 |
| city::schedule::tests | crates/city/src/schedule/tests.rs | windows return each entry at most once | decision | P2 | built | city-SPEC.md#8-6 |
| city::room | crates/city/src/room.rs | which room a named session works in, and how a new one comes into being | decision | F2 | built | city-SPEC.md#8-13 |
| city::watch | crates/city/src/watch.rs | what the city is listening to, and which building answers | value | P4 | built | city-SPEC.md#8-7 |
| city::wizard | crates/city/src/wizard.rs | starting a city, and moving a resident inside one | decision | P4 | built | city-SPEC.md#8-10 |
| city::neighbourhood | crates/city/src/neighbourhood.rs | which addresses a run can reach and who stands at them, in less detail the further away they are | decision | P3 | built | city-SPEC.md#8-15 |
| city::neighbours_tool | crates/city/src/neighbours_tool.rs | the face the neighbourhood shows a model: this building's addresses, or the city's buildings by name | adapter | P3 | built | city-SPEC.md#8-15 |
| city::gitignore | crates/city/src/gitignore.rs | what a building promises is tracked, what one session was thinking is not | data | V4 | built | city-SPEC.md#8-21 |
| city::vocation | crates/city/src/vocation.rs | whether the residents at an address build or plan | decision | V4 | built | city-SPEC.md#8-22 |
| city::city_tool | crates/city/src/city_tool.rs | the face the city itself shows a model: raise, adopt, list | adapter | V4 | built | city-SPEC.md#8-23 |

### eval (10) — whether a change made the city better

| Module | File | What it owns | Shape | Since | Status | Spec |
|---|---|---|---|---|---|---|
| eval::suite | crates/eval/src/suite.rs | real tasks split into what a change may learn from and what judges it | decision | P3 | built | eval-SPEC.md#8-1 |
| eval::probe | crates/eval/src/probe.rs | the same questions asked twice, versioned so two versions never compare | value | P3 | built | eval-SPEC.md#8-2 |
| eval::score | crates/eval/src/score.rs | what a settled asset is worth, in integer thousandths | decision | P3 | built | eval-SPEC.md#8-3 |
| eval::nesting | crates/eval/src/nesting.rs | which nested format a model edits with fewest mistakes, and how it fails when it fails | decision | V3 | built | eval-SPEC.md#8-5 |
| eval::nesting::reading | crates/eval/src/nesting/reading.rs | reading a document of any of the three shapes into the same leaves | decision | V3 | built | eval-SPEC.md#8-5 |
| eval::nesting::tests | crates/eval/src/nesting/tests.rs | the graded fixtures: worst fault wins, and a tie broken by damage | decision | V3 | built | eval-SPEC.md#8-5 |
| eval::metabolism | crates/eval/src/metabolism.rs | clearing out: warn first, retire second, delete never | decision | P3 | built | eval-SPEC.md#8-4 |
| eval::ablation | crates/eval/src/ablation.rs | what a resident stops being able to do when one passage of the city document is removed | decision | V3 | built | eval-SPEC.md#8-7 |
| eval::ablation::capabilities | crates/eval/src/ablation/capabilities.rs | the corpus: each thing a resident must be able to do, and the phrase in the document that grants it | data | V3 | built | eval-SPEC.md#8-7 |
| eval::ablation::tests | crates/eval/src/ablation/tests.rs | the graded fixtures, and the on-demand run against the real City.md | decision | V3 | built | eval-SPEC.md#8-7 |

### channels (17) — the process boundary

| Module | File | What it owns | Shape | Since | Status | Spec |
|---|---|---|---|---|---|---|
| channels::wire | crates/channels/src/wire.rs | the envelope both sides speak: the frames, the version and its hash | value | S4 | built | channels-SPEC.md#8-1 |
| channels::wire::query | crates/channels/src/wire/query.rs | everything a page may ask, and the name table the schema hash is built from | value | V4 | built | channels-SPEC.md#8-1 |
| channels::command | crates/channels/src/command.rs | everything a client may ask the city to do, and the one frame it cannot spell | value | V3 | built | channels-SPEC.md#8-1 |
| channels::command::kind | crates/channels/src/command/kind.rs | names, steps, the wire enum | value | V3 | built | channels-SPEC.md#8-1 |
| channels::command::wire | crates/channels/src/command/wire.rs | no-secret commands and back | value | V3 | built | channels-SPEC.md#8-1 |
| channels::answer | crates/channels/src/answer.rs | what a Query comes back as: one shape per view, and the closed set of them | value | V3 | built | channels-SPEC.md#8-1 |
| channels::answer::building | crates/channels/src/answer/building.rs | what one building says about itself: its plan rows, what is stuck, its goal, its documents and its archive | value | V4 | built | channels-SPEC.md#8-1 |
| channels::carried_name | crates/channels/src/carried_name.rs | names this crate does not own, validated at one construction point | value | V3 | built | channels-SPEC.md#8-1 |
| channels::reception | crates/channels/src/reception.rs | may we bind, may this peer enrol, may we greet it, and what its frame means now | decision | V3 | built | channels-SPEC.md#8-2 |
| channels::assets | crates/channels/src/assets.rs | the client the browser downloads, and which bytes answer which path | adapter | V3 | built | channels-SPEC.md#8-2 |
| channels::server | crates/channels/src/server.rs | the listening end; the judgements are pure and the socket makes none | adapter | S4 | built | channels-SPEC.md#8-2 |
| channels::server::config | crates/channels/src/server/config.rs | routes and bodies | adapter | S4 | built | channels-SPEC.md#8-2 |
| channels::server::reply | crates/channels/src/server/reply.rs | deliveries and refusals | adapter | S4 | built | channels-SPEC.md#8-2 |
| channels::server::socket | crates/channels/src/server/socket.rs | sessions, assets, uploads | adapter | S4 | built | channels-SPEC.md#8-2 |
| channels::control | crates/channels/src/control.rs | the five verbs a person has, and which of them owe a handoff | decision | S4 | built | channels-SPEC.md#8-4 |
| channels::auth | crates/channels/src/auth.rs | pairing tokens: minting, the one readable form, constant-time comparison | value | S4 | built | channels-SPEC.md#8-3 |
| channels::aggregate | crates/channels/src/aggregate.rs | watching several cities from one interface, queries and events only | decision | S4 | built | channels-SPEC.md#8-5 |

### web (114) — the only client, compiled to WebAssembly

| Module | File | What it owns | Shape | Since | Status | Spec |
|---|---|---|---|---|---|---|
| web::app | crates/web/src/app.rs | what the client believes, folded forward from events; holds no business state | projection | S4 | built | web-SPEC.md#8-1 |
| web::app::snapshot | crates/web/src/app/snapshot.rs | what the client believes, folded forward from events | projection | S4 | built | web-SPEC.md#8-1 |
| web::app::reading | crates/web/src/app/reading.rs | what a page reads from what the client believes | projection | S4 | built | web-SPEC.md#8-1 |
| web::app::fold | crates/web/src/app/fold.rs | one event forward: the fold that advances what the client believes | projection | S4 | built | web-SPEC.md#8-1 |
| web::app::rows | crates/web/src/app/rows.rs | one run as a row, and what the model calls consumed | value | S4 | built | web-SPEC.md#8-1 |
| web::app::tests | crates/web/src/app/tests.rs | the fold, exercised through its production door | projection | S4 | built | web-SPEC.md#8-1 |
| web::readout | crates/web/src/readout.rs | what a page says about a snapshot, in the reader's own language | decision | V3 | built | web-SPEC.md#8-62 |
| web::asking | crates/web/src/asking.rs | what this client keeps, what it asks for again, and what it missed | decision | V3 | built | web-SPEC.md#8-62 |
| web::shell | crates/web/src/shell.rs | which region shows what, and the client that mounts it | projection | V3 | built | web-SPEC.md#8-62 |
| web::shell::root | crates/web/src/shell/root.rs | three regions, and nothing that decides anything | projection | V3 | built | web-SPEC.md#8-62 |
| web::shell::client | crates/web/src/shell/client.rs | the live snapshot and mounting it | projection | V3 | built | web-SPEC.md#8-62 |
| web::shell::nav | crates/web/src/shell/nav.rs | everything the palette can reach | decision | V3 | built | web-SPEC.md#8-62 |
| web::mount | crates/web/src/mount.rs | the four things only a browser has: address bar, keyboard, socket, frame | adapter | V3 | built | web-SPEC.md#8-62 |
| web::mount::wiring | crates/web/src/mount/wiring.rs | every signal one page holds | value | V3 | built | web-SPEC.md#8-62 |
| web::mount::address | crates/web/src/mount/address.rs | the one reader of the address bar | adapter | V3 | built | web-SPEC.md#8-62 |
| web::mount::keys | crates/web/src/mount/keys.rs | the one place a keystroke reaches this client | adapter | V3 | built | web-SPEC.md#8-62 |
| web::mount::outbound | crates/web/src/mount/outbound.rs | the one way a component reaches the server | adapter | V3 | built | web-SPEC.md#8-62 |
| web::mount::shell | crates/web/src/mount/shell.rs | socket, starting the client, and its theme | adapter | V3 | built | web-SPEC.md#8-62 |
| web::mount::frame | crates/web/src/mount/frame.rs | what one painted frame may move | value | V3 | built | web-SPEC.md#8-62 |
| web::board | crates/web/src/board.rs | the plan tree laid out by state; five columns, no state of its own, nothing here can move a node | projection | V3 | built | web-SPEC.md#8-58 |
| web::command | crates/web/src/command.rs | every command frame this client sends, built in one place | value | V3 | built | web-SPEC.md#8-60 |
| web::command::tests | crates/web/src/command/tests.rs | what a dispatch from the building form names: a room per piece of work, never the building's root | value | V3 | built | web-SPEC.md#8-60 |
| web::pursuit | crates/web/src/pursuit.rs | what a building is working towards on its own, and who answers for it | adapter | V3 | built | web-SPEC.md#8-60 |
| web::socket | crates/web/src/socket.rs | the only place in this crate that talks to the server | adapter | S4 | built | web-SPEC.md#8-2 |
| web::socket::link | crates/web/src/socket/link.rs | the link: state, events, actions, and the backoff ladder | decision | S4 | built | web-SPEC.md#8-2 |
| web::socket::frames | crates/web/src/socket/frames.rs | reading frames: text in, link events out | decision | S4 | built | web-SPEC.md#8-2 |
| web::socket::enrol | crates/web/src/socket/enrol.rs | the one credential that never becomes a command | decision | S4 | built | web-SPEC.md#8-2 |
| web::socket::tests | crates/web/src/socket/tests.rs | the ladder and the handshake, through the production door | decision | S4 | built | web-SPEC.md#8-2 |
| web::pace | crates/web/src/pace.rs | how often this page may change, and what a burst of frames folds into | decision | R2 | built | web-SPEC.md#8-41 |
| web::keys | crates/web/src/keys.rs | what a keystroke means, and the one sequence that cannot strand a reader | decision | R2 | built | web-SPEC.md#8-45 |
| web::palette | crates/web/src/palette.rs | one box that reaches every page, building and session, and how a query ranks them | decision | R2 | built | web-SPEC.md#8-45 |
| web::turn | crates/web/src/turn.rs | a session's events folded into the rounds a person reads: what was said, what it cost, what each call came to, and what a door said about it | decision | R2 | built | web-SPEC.md#8-47 |
| web::turn::reading | crates/web/src/turn/reading.rs | what was said, what it cost, what each call came to | value | R2 | built | web-SPEC.md#8-47 |
| web::turn::rounds | crates/web/src/turn/rounds.rs | turns folded from the session events | decision | R2 | built | web-SPEC.md#8-47 |
| web::turn::tests | crates/web/src/turn/tests.rs | the fold, exercised through its production door | decision | R2 | built | web-SPEC.md#8-47 |
| web::turn::rounds_tests | crates/web/src/turn/rounds_tests.rs | the rounds through the production door | decision | R2 | built | web-SPEC.md#8-47 |
| web::turn::reading_tests | crates/web/src/turn/reading_tests.rs | the reading through the production door | value | R2 | built | web-SPEC.md#8-47 |
| web::city_view | crates/web/src/city_view.rs | the city page: the picture, the controls around it, and what a click means | projection | S4 | built | web-SPEC.md#8-13 |
| web::city_view::page | crates/web/src/city_view/page.rs | the picture, the controls around it, and what a click means | projection | S4 | built | web-SPEC.md#8-13 |
| web::city_view::text | crates/web/src/city_view/text.rs | the sentence that names the way into one building's own pages | projection | S4 | built | web-SPEC.md#8-13 |
| web::isometry | crates/web/src/isometry.rs | where a point on the ground lands on the screen, and the window around what was drawn | decision | V3 | built | web-SPEC.md#8-61 |
| web::isometry::tests | crates/web/src/isometry/tests.rs | the window holds what was drawn, zoom crops, and one shape is spelled one way | decision | V3 | built | web-SPEC.md#8-61 |
| web::skyline | crates/web/src/skyline.rs | what a city of buildings looks like: height from assets, a lit band from the plan | decision | V3 | built | web-SPEC.md#8-61 |
| web::skyline::prisms | crates/web/src/skyline/prisms.rs | what a city of buildings looks like | decision | V3 | built | web-SPEC.md#8-61 |
| web::skyline::faces | crates/web/src/skyline/faces.rs | prisms become geometry, and geometry becomes a list | decision | V3 | built | web-SPEC.md#8-61 |
| web::skyline::tests | crates/web/src/skyline/tests.rs | the same city draws the same shapes, through the production door | decision | V3 | built | web-SPEC.md#8-61 |
| web::progress | crates/web/src/progress.rs | the one place a progress bar is drawn, for all three of its callers | decision | S4 | built | web-SPEC.md#8-5 |
| web::dashboard | crates/web/src/dashboard.rs | cost in five cuts, with shares against the authoritative total | decision | S4 | built | web-SPEC.md#8-6 |
| web::dashboard::tests | crates/web/src/dashboard/tests.rs | the cost page orders facts the same way every time | decision | S4 | built | web-SPEC.md#8-6 |
| web::live | crates/web/src/live.rs | watching one session as it happens, in a window that says what it dropped | decision | S4 | built | web-SPEC.md#8-6 |
| web::live::feed | crates/web/src/live/feed.rs | the bounded window: what the live view keeps | value | S4 | built | web-SPEC.md#8-6 |
| web::live::describe | crates/web/src/live/describe.rs | one event, one short line | decision | S4 | built | web-SPEC.md#8-6 |
| web::live::commands | crates/web/src/live/commands.rs | the commands a watcher can send | decision | S4 | built | web-SPEC.md#8-6 |
| web::live::page | crates/web/src/live/page.rs | watching one session as it happens | decision | S4 | built | web-SPEC.md#8-6 |
| web::live::rounds | crates/web/src/live/rounds.rs | one row per turn, what it did inside it | decision | S4 | built | web-SPEC.md#8-6 |
| web::live::stream | crates/web/src/live/stream.rs | raw order one click down, and the empty states | decision | S4 | built | web-SPEC.md#8-6 |
| web::live::composer | crates/web/src/live/composer.rs | the steer box and the interventions | decision | S4 | built | web-SPEC.md#8-6 |
| web::live::tests | crates/web/src/live/tests.rs | the window and the wording, through the production door | decision | S4 | built | web-SPEC.md#8-6 |
| web::approval | crates/web/src/approval.rs | two lists that share one shape: what waits for a person, and what was discarded | decision | S4 | built | web-SPEC.md#8-5 |
| web::approval::inbox | crates/web/src/approval/inbox.rs | things waiting for a person, grouped by cluster key | decision | S4 | built | web-SPEC.md#8-5 |
| web::approval::bin | crates/web/src/approval/bin.rs | what was discarded and how it comes back | decision | S4 | built | web-SPEC.md#8-5 |
| web::approval::tests | crates/web/src/approval/tests.rs | the inbox and the bin, through the production door | decision | S4 | built | web-SPEC.md#8-5 |
| web::ledger_view | crates/web/src/ledger_view.rs | browsing the one history; a filter always says how much it hid | decision | S4 | built | web-SPEC.md#8-6 |
| web::ledger_view::tests | crates/web/src/ledger_view/tests.rs | the filter, the paging limit, and an export that names itself a view | decision | S4 | built | web-SPEC.md#8-6 |
| web::alert | crates/web/src/alert.rs | the only module that may interrupt a person, and only once per fact | decision | S4 | built | web-SPEC.md#8-6 |
| web::alert::judge | crates/web/src/alert/judge.rs | what needs a person, and only once per fact | decision | S4 | built | web-SPEC.md#8-6 |
| web::alert::notify | crates/web/src/alert/notify.rs | the interruption that reaches another tab | adapter | S4 | built | web-SPEC.md#8-6 |
| web::alert::tests | crates/web/src/alert/tests.rs | one fact, one interruption, through the production door | decision | S4 | built | web-SPEC.md#8-6 |
| web::lang | crates/web/src/lang.rs | every word this client says, in the two languages it says them in | data | F2 | built | web-SPEC.md#8-39 |
| web::theme | crates/web/src/theme.rs | the single-hue language: the only place that produces a colour | data | S4 | built | web-SPEC.md#8-4 |
| web::building_view | crates/web/src/building_view.rs | one building, what it has written down, and what waits in each room | decision | R1 | built | web-SPEC.md#8-36 |
| web::building_view::leaf | crates/web/src/building_view/leaf.rs | which face of a building opens first | decision | R1 | built | web-SPEC.md#8-36 |
| web::building_view::room | crates/web/src/building_view/room.rs | waiting signals in one room | decision | R1 | built | web-SPEC.md#8-36 |
| web::building_view::text | crates/web/src/building_view/text.rs | pieces and their classes | decision | R1 | built | web-SPEC.md#8-36 |
| web::building_view::faces | crates/web/src/building_view/faces.rs | what each open leaf shows | decision | R1 | built | web-SPEC.md#8-36 |
| web::building_view::page | crates/web/src/building_view/page.rs | one building, what it has written down | decision | R1 | built | web-SPEC.md#8-36 |
| web::building_view::tests | crates/web/src/building_view/tests.rs | leaves and queues, through the production door | decision | R1 | built | web-SPEC.md#8-36 |
| web::reach | crates/web/src/reach.rs | what a building's runs may reach, and the one form that sets it | decision | P3 | built | web-SPEC.md#8-53 |
| web::drop | crates/web/src/drop.rs | what a drag means at each of the four places it can land, and what it is refused for | decision | P0 | built | web-SPEC.md#8-49 |
| web::vitals | crates/web/src/vitals.rs | the few numbers no other surface states, and the four it refuses to state | decision | F1 | built | web-SPEC.md#8-25 |
| web::archive_search | crates/web/src/archive_search.rs | what this city wrote down: the shelves and the record, never merged | decision | F1 | built | web-SPEC.md#8-24 |
| web::settings | crates/web/src/settings.rs | turning a URL and a key into a model a run can be given | decision | P1 | built | web-SPEC.md#8-12 |
| web::settings::forms | crates/web/src/settings/forms.rs | the two forms and what a complete choice is | decision | P1 | built | web-SPEC.md#8-12 |
| web::settings::tables | crates/web/src/settings/tables.rs | what the server answered, read as rows | decision | P1 | built | web-SPEC.md#8-12 |
| web::settings::attach | crates/web/src/settings/attach.rs | the attach-provider form | decision | P1 | built | web-SPEC.md#8-12 |
| web::settings::login | crates/web/src/settings/login.rs | the subscription login | decision | P1 | built | web-SPEC.md#8-12 |
| web::settings::choose | crates/web/src/settings/choose.rs | the model-choice form | decision | P1 | built | web-SPEC.md#8-12 |
| web::settings::listing | crates/web/src/settings/listing.rs | what each model is for, and what is attached | decision | P1 | built | web-SPEC.md#8-12 |
| web::settings::page | crates/web/src/settings/page.rs | the settings page shell | decision | P1 | built | web-SPEC.md#8-12 |
| web::settings::tests | crates/web/src/settings/tests.rs | the forms, read here; the frames they turn into are command’s | decision | P1 | built | web-SPEC.md#8-12 |
| web::route | crates/web/src/route.rs | the one translation between a View and the address bar, both ways | decision | F2 | built | web-SPEC.md#8-14 |
| web::route::view | crates/web/src/route/view.rs | which page the content region shows, and its lens | decision | F2 | built | web-SPEC.md#8-14 |
| web::route::fragments | crates/web/src/route/fragments.rs | one spelling written, every old spelling read | decision | F2 | built | web-SPEC.md#8-14 |
| web::route::places | crates/web/src/route/places.rs | nav entries, and what is showing | decision | F2 | built | web-SPEC.md#8-14 |
| web::route::tests | crates/web/src/route/tests.rs | fragments both ways, through the production door | decision | F2 | built | web-SPEC.md#8-14 |
| web::panel | crates/web/src/panel.rs | the one version of a centre panel: conclusion, scope, body, and where the numbers came from | decision | F2 | built | web-SPEC.md#8-29 |
| web::phase | crates/web/src/phase.rs | what a session is doing, in the one vocabulary every surface reads from | data | V3 | built | web-SPEC.md#8-53 |
| web::sessions | crates/web/src/sessions.rs | the first screen: the box that starts work, and the table its rows land in | decision | V3 | built | web-SPEC.md#8-67 |
| web::sessions::plan | crates/web/src/sessions/plan.rs | guess, choose, and what each field means | decision | V3 | built | web-SPEC.md#8-67 |
| web::sessions::listing | crates/web/src/sessions/listing.rs | seats, listing, and readings | decision | V3 | built | web-SPEC.md#8-67 |
| web::sessions::composer | crates/web/src/sessions/composer.rs | the ladder and the box that sends work | decision | V3 | built | web-SPEC.md#8-67 |
| web::sessions::tables | crates/web/src/sessions/tables.rs | what is moving, what ended, which buildings are busy | decision | V3 | built | web-SPEC.md#8-67 |
| web::sessions::page | crates/web/src/sessions/page.rs | the first screen | decision | V3 | built | web-SPEC.md#8-67 |
| web::sessions::tests | crates/web/src/sessions/tests.rs | guess versus decision, through the production door | decision | V3 | built | web-SPEC.md#8-67 |
| web::session | crates/web/src/session.rs | one session: the four questions a person arrives with, and five readings of what it did | decision | V3 | built | web-SPEC.md#8-35 |
| web::session::facts | crates/web/src/session/facts.rs | the four questions a person arrives with | decision | V3 | built | web-SPEC.md#8-35 |
| web::session::tabs | crates/web/src/session/tabs.rs | five readings of what a session did | decision | V3 | built | web-SPEC.md#8-35 |
| web::session::links | crates/web/src/session/links.rs | building of an address, and old live links | decision | V3 | built | web-SPEC.md#8-35 |
| web::session::page | crates/web/src/session/page.rs | one session: questions and readings | decision | V3 | built | web-SPEC.md#8-35 |
| web::session::tests | crates/web/src/session/tests.rs | the head through the production door | decision | V3 | built | web-SPEC.md#8-35 |
| web::prompt | crates/web/src/prompt.rs | what a run was given: the four frozen blocks of its prompt, and whether an admitted skill's bytes moved since the city last looked | projection | V3 | built | web-SPEC.md#8-67 |
| web::prompt::tests | crates/web/src/prompt/tests.rs | what one run was given, through the production door | projection | V3 | built | web-SPEC.md#8-67 |
| web::waiting | crates/web/src/waiting.rs | everything that cannot move until a person answers, in one place | decision | V3 | built | web-SPEC.md#8-64 |
| web::record | crates/web/src/record.rs | one history, in three lenses, at one address | decision | V3 | built | web-SPEC.md#8-67 |

### browser (12), protocol (5), bin (119)

| Module | File | What it owns | Shape | Since | Status | Spec |
|---|---|---|---|---|---|---|
| browser::port (port) | crates/browser/src/port.rs | the browser seam: frames out, replies in; a refusal is an answer | port | P4 | built | browser-SPEC.md#8-1 |
| browser::session | crates/browser/src/session.rs | which frames a conversation with a browser is made of | adapter | P4 | built | browser-SPEC.md#8-2 |
| browser::snapshot | crates/browser/src/snapshot.rs | what a model may see of a page: accessibility tree, never raw DOM | decision | P4 | built | browser-SPEC.md#8-3 |
| browser::act | crates/browser/src/act.rs | turning an intention into frames, with reference and generation both checked | decision | P4 | built | browser-SPEC.md#8-4 |
| browser::devloop | crates/browser/src/devloop.rs | change something, look at it, decide: four outcomes and always an end | decision | P4 | built | browser-SPEC.md#8-5 |
| browser::profile | crates/browser/src/profile.rs | where a browser keeps what it remembers, and who that belongs to | decision | P4 | built | browser-SPEC.md#8-6 |
| browser::verb | crates/browser/src/verb.rs | the eight actions the browser tool offers, each read from arguments and turned into frames | decision | V4 | built | browser-SPEC.md#19-2 |
| browser::verb::tests | crates/browser/src/verb/tests.rs | what the eight actions are read from, and which frames each one puts on the wire | decision | V4 | built | browser-SPEC.md#19-2 |
| browser::shot | crates/browser/src/shot.rs | what a screenshot was asked for, and what came back: bytes, format, and two integer sides | value | V4 | built | browser-SPEC.md#19-3 |
| browser::shot::tests | crates/browser/src/shot/tests.rs | the two fractions written out of integers, and a size read from the bytes | value | V4 | built | browser-SPEC.md#19-3 |
| browser::diff | crates/browser/src/diff.rs | what changed between two screenshots: a ten-thousandth, and the boxes it changed in | decision | V4 | built | browser-SPEC.md#19-4 |
| browser::diff::tests | crates/browser/src/diff/tests.rs | two screenshots, and what a caller is told about the difference | decision | V4 | built | browser-SPEC.md#19-4 |
| protocol::mcp | crates/protocol/src/mcp.rs | reaching an MCP server, and the seam its transports sit behind | adapter | P4 | built | protocol-SPEC.md#8-1 |
| protocol::mcp::handshake | crates/protocol/src/mcp/handshake.rs | initialize, initialized, ready | adapter | P4 | built | protocol-SPEC.md#8-1 |
| protocol::mcp::tools | crates/protocol/src/mcp/tools.rs | listing, naming, calling | adapter | P4 | built | protocol-SPEC.md#8-1 |
| protocol::mcp::outbound | crates/protocol/src/mcp/outbound.rs | one line per request | adapter | P4 | built | protocol-SPEC.md#8-1 |
| protocol::acp | crates/protocol/src/acp.rs | the other direction: an outside editor driving this city | decision | P4 | built | protocol-SPEC.md#8-2 |
| bin::main | crates/sprawling/src/main.rs | the command line, each subcommand refused honestly until it exists | adapter | S0 | built | sprawling-SPEC.md#8-30 |
| bin::main::router | crates/sprawling/src/main/router.rs | one verb in, one subcommand out, plus the flags every verb reads | adapter | S0 | built | sprawling-SPEC.md#8-30 |
| bin::main::city | crates/sprawling/src/main/city.rs | the verbs that raise and serve a city | adapter | S0 | built | sprawling-SPEC.md#8-30 |
| bin::main::data | crates/sprawling/src/main/data.rs | the verbs that move bytes and ask about history | adapter | S0 | built | sprawling-SPEC.md#8-30 |
| bin::main::whose | crates/sprawling/src/main/whose.rs | the verb that asks a commit which run wrote it | adapter | V4 | built | sprawling-SPEC.md#8-30 |
| bin::main::tests | crates/sprawling/src/main/tests.rs | flags are never paths, and no ledger is never verified | adapter | S0 | built | sprawling-SPEC.md#8-30 |
| bin::assembly | crates/sprawling/src/assembly.rs | the assembly point: the worker every module below writes methods for, the one clock sample, and the one door a command enters by | adapter | S0 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::lifetime | crates/sprawling/src/assembly/lifetime.rs | a worker opened over a history, and the city closed in the record | adapter | S0 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::fixture | crates/sprawling/src/assembly/fixture.rs | the fake provider and the worker every assembly test starts from | adapter | S0 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::recording | crates/sprawling/src/assembly/recording.rs | the lines this worker appends: one for the city, one for a run, and the diagnostic note beside them | adapter | V4 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::genesis | crates/sprawling/src/assembly/genesis.rs | forming a city in a directory, taking a folder in as a building, and what a restart finds | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::genesis::tests | crates/sprawling/src/assembly/genesis/tests.rs | genesis happens once, adoption leaves the work alone, and a restart closes what a crash left open | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::naming | crates/sprawling/src/assembly/naming.rs | the wire's words and the kernel's, translated one way each | decision | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::folds | crates/sprawling/src/assembly/folds.rs | everything a worker inherits from a history it did not write, in one verified pass | projection | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::folds::collaboration | crates/sprawling/src/assembly/folds/collaboration.rs | what is waiting in each room, and what ground is already claimed | projection | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::folds::tests | crates/sprawling/src/assembly/folds/tests.rs | the folds tests route | projection | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::folds::tests::standing | crates/sprawling/src/assembly/folds/tests/standing.rs | what one verified pass rebuilds: queues, endpoint book, halted scopes | projection | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::folds::tests::history | crates/sprawling/src/assembly/folds/tests/history.rs | what a page reads back: the city, one session, and where a slice resumes | projection | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::building_page | crates/sprawling/src/assembly/building_page.rs | one building, as the files in it say it is | projection | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::credentials | crates/sprawling/src/assembly/credentials.rs | what this city can sign in as, and what it may call | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::credentials::signing | crates/sprawling/src/assembly/credentials/signing.rs | a subscription login in two steps, the renewal before use, and the vault | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::credentials::endpoints | crates/sprawling/src/assembly/credentials/endpoints.rs | an endpoint probed before it is attached, and a model chosen for a tag | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::credentials::tests | crates/sprawling/src/assembly/credentials/tests.rs | logins, enrolments and the endpoint a dispatch needs | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::mcp | crates/sprawling/src/assembly/mcp.rs | reaching the MCP servers a building's configuration names | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::dispatching | crates/sprawling/src/assembly/dispatching.rs | one dispatch, from what a person asked to the run that froze | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::dispatching::agreeing | crates/sprawling/src/assembly/dispatching/agreeing.rs | every refusal a dispatch can owe before it costs anything | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::dispatching::running | crates/sprawling/src/assembly/dispatching/running.rs | one dispatch run to its freeze, and the handback it leaves | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::dispatching::session | crates/sprawling/src/assembly/dispatching/session.rs | which room a dispatch works in: the session a person named, or the name the digest model gives the work | adapter | V4 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::dispatching::tests | crates/sprawling/src/assembly/dispatching/tests.rs | steers, refusals and handbacks as the rooms see them | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::waking | crates/sprawling/src/assembly/waking.rs | the two ways a resident who is not working is set going | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::waking::tests | crates/sprawling/src/assembly/waking/tests.rs | arrivals and knocks, as the rooms see them | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::workbench | crates/sprawling/src/assembly/workbench.rs | where a run stands, and the bench it is given to work at | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::workbench::standing | crates/sprawling/src/assembly/workbench/standing.rs | where one run stands, settled once the city has agreed to take the work | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::workbench::desks | crates/sprawling/src/assembly/workbench/desks.rs | the desks one dispatch lends out and takes back | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::workbench::tools | crates/sprawling/src/assembly/workbench/tools.rs | one registration feeding the catalogue and the bench, and the status tool | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::workbench::servers | crates/sprawling/src/assembly/workbench/servers.rs | the external tools a building's configuration names | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::workbench::engine | crates/sprawling/src/assembly/workbench/engine.rs | the execution engine this build carries, and the shell a layer may ask for | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::workbench::tests | crates/sprawling/src/assembly/workbench/tests.rs | rules that do not parse, neighbours, signals, and the engine arms | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::freezing | crates/sprawling/src/assembly/freezing.rs | what a run is frozen with: its plan, and the handoff that resumes it | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::freezing::tests | crates/sprawling/src/assembly/freezing/tests.rs | the freezing tests route | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::freezing::tests::dispatches | crates/sprawling/src/assembly/freezing/tests/dispatches.rs | what one dispatch freezes and leaves: the handoff, the job bytes, the prompt, the lineage | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::freezing::tests::ceilings | crates/sprawling/src/assembly/freezing/tests/ceilings.rs | the ceiling and the effort a run is frozen under | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::driving | crates/sprawling/src/assembly/driving.rs | what one drive is handed, what it leaves, and the handles it takes from the worker | adapter | V3 | built | sprawling-SPEC.md#8-43 |
| bin::assembly::driving::lane | crates/sprawling/src/assembly/driving/lane.rs | one drive as a lane runs it: the ledger it writes through, the three hooks, and who may interrupt it | adapter | V4 | built | sprawling-SPEC.md#8-46 |
| bin::assembly::driving::tests | crates/sprawling/src/assembly/driving/tests.rs | the driving fixtures route | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::driving::tests::turns | crates/sprawling/src/assembly/driving/tests/turns.rs | turn loops, cancels and steers | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::driving::tests::confidential | crates/sprawling/src/assembly/driving/tests/confidential.rs | confidential stops before a remote call | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::driving::tests::ledger | crates/sprawling/src/assembly/driving/tests/ledger.rs | lines before the city they announce | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::driving::tests::sieving | crates/sprawling/src/assembly/driving/tests/sieving.rs | a command's output over the floor reaches the model sieved, and the way back resolves | adapter | V4 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::settling | crates/sprawling/src/assembly/settling.rs | what a drive left, on the ledger before it is made true | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::settling::desks | crates/sprawling/src/assembly/settling/desks.rs | the four desks settled in history order | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::settling::landing | crates/sprawling/src/assembly/settling/landing.rs | one landing made true, and what a drive ended with | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::settling::tests | crates/sprawling/src/assembly/settling/tests.rs | the settling fixtures route | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::settling::tests::ending | crates/sprawling/src/assembly/settling/tests/ending.rs | endings and hand-downs | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::settling::tests::landing | crates/sprawling/src/assembly/settling/tests/landing.rs | half-settled landings leave nothing torn | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::settling::tests::succession | crates/sprawling/src/assembly/settling/tests/succession.rs | a resident replacing itself: same tools, conserved depth, a lineage of four, and the room's own handoff | adapter | V4 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::settling::tests::halting | crates/sprawling/src/assembly/settling/tests/halting.rs | a halt on a scope reaches the run a resident handed down inside it | adapter | V4 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::reviewing | crates/sprawling/src/assembly/reviewing.rs | what a run offers a building it may not write in, and what merging it costs | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::probing | crates/sprawling/src/assembly/probing.rs | the handoff probe asked of a predecessor and of its successor, and the line that records what survived | adapter | V4 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::reviewing::tests | crates/sprawling/src/assembly/reviewing/tests.rs | the index of the review tests | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::reviewing::tests::landing | crates/sprawling/src/assembly/reviewing/tests/landing.rs | work reaching a review building only after a second resident checks it | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::reviewing::tests::refusal | crates/sprawling/src/assembly/reviewing/tests/refusal.rs | a merge whose announcing line the history refused leaves the building where it was | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::plans | crates/sprawling/src/assembly/plans.rs | one building's plan: who holds what, what is ready, and how far red reaches | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::plans::pursuing | crates/sprawling/src/assembly/plans/pursuing.rs | the whole ready set driven at once: which nodes are in somebody's hands, which lanes they are in, and where each one lands | adapter | V4 | built | sprawling-SPEC.md#8-46 |
| bin::assembly::plans::tests | crates/sprawling/src/assembly/plans/tests.rs | what a plan and a standing goal do to the city, by theme | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::plans::tests::rows | crates/sprawling/src/assembly/plans/tests/rows.rs | a plan's rows read, claimed, closed, and the claim whose line never landed | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::plans::tests::goals | crates/sprawling/src/assembly/plans/tests/goals.rs | a goal that collides with a claimed path, and a graph run room by room | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::commanding | crates/sprawling/src/assembly/commanding.rs | the verbs a person sends, and what each one does to the city | adapter | V3 | built | sprawling-SPEC.md#8-41 |
| bin::assembly::commanding::routing | crates/sprawling/src/assembly/commanding/routing.rs | one verb in, one dispatch or refusal out, plus the schedule tick | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::commanding::governing | crates/sprawling/src/assembly/commanding/governing.rs | halts, autonomy, approvals, forks: the standing answers | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::commanding::entrance | crates/sprawling/src/assembly/commanding/entrance.rs | the key every command carries, read before anything is done about it | decision | V4 | built | sprawling-SPEC.md#8-41 |
| bin::assembly::commanding::tests | crates/sprawling/src/assembly/commanding/tests.rs | the commanding fixtures route | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::commanding::tests::answering | crates/sprawling/src/assembly/commanding/tests/answering.rs | answers, refusals and the approval queue | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::commanding::tests::clockwork | crates/sprawling/src/assembly/commanding/tests/clockwork.rs | the schedule fires and repeats do not | adapter | V3 | built | sprawling-SPEC.md#8-39 |
| bin::assembly::commanding::tests::entrance | crates/sprawling/src/assembly/commanding/tests/entrance.rs | one ask under one key is one effect, across a restart too | decision | V4 | built | sprawling-SPEC.md#8-39 |
| bin::serving | crates/sprawling/src/serving.rs | how a city is stood up and served: the key at the door, the desk, the one writer thread | adapter | V3 | built | sprawling-SPEC.md#8-38 |
| bin::serving::door | crates/sprawling/src/serving/door.rs | the key at the door and the vault behind it | adapter | V3 | built | sprawling-SPEC.md#8-38 |
| bin::serving::desk | crates/sprawling/src/serving/desk.rs | where commands wait between the socket and the worker | adapter | V3 | built | sprawling-SPEC.md#8-38 |
| bin::serving::relay | crates/sprawling/src/serving/relay.rs | the third Ledger adapter: a write from a driving thread, carried to the accounting thread and waited for | adapter | V4 | built | sprawling-SPEC.md#8-42 |
| bin::serving::pool | crates/sprawling/src/serving/pool.rs | the driving lanes: one thread per run in flight, entered with a drive and left with what it left behind | adapter | V4 | built | sprawling-SPEC.md#8-46 |
| bin::serving::serve | crates/sprawling/src/serving/serve.rs | what one served city is made of | adapter | V3 | built | sprawling-SPEC.md#8-38 |
| bin::serving::worker | crates/sprawling/src/serving/worker.rs | the one writer thread and the city it serves | adapter | V3 | built | sprawling-SPEC.md#8-38 |
| bin::serving::tests | crates/sprawling/src/serving/tests.rs | the key this city mints is the key its own door accepts | adapter | V3 | built | sprawling-SPEC.md#8-38 |
| bin::browser_bidi::engine | crates/sprawling/src/browser_bidi/engine.rs | which engine this machine can hold a session with, and the command line that starts it | decision | V4 | built | sprawling-SPEC.md#8-45 |
| bin::browser_bidi::lazy | crates/sprawling/src/browser_bidi/lazy.rs | an engine nobody has started yet, and the port it will answer on | adapter | V4 | built | sprawling-SPEC.md#8-45 |
| bin::browser_bidi::socket | crates/sprawling/src/browser_bidi/socket.rs | one frame out, replies in, events read past | adapter | V4 | built | sprawling-SPEC.md#8-45 |
| bin::browser_tool | crates/sprawling/src/browser_tool.rs | the browser tool: eight actions over one session, and a screenshot that lands in the content store | adapter | V4 | built | sprawling-SPEC.md#8-45 |
| bin::browser_tool::tests | crates/sprawling/src/browser_tool/tests.rs | one conversation with a browser, replayed without one | adapter | V4 | built | sprawling-SPEC.md#8-45 |
| bin::mcp_stdio | crates/sprawling/src/mcp_stdio.rs | an MCP server as a child process, one line per message | adapter | R1 | built | sprawling-SPEC.md#8-4 |
| bin::mcp_http | crates/sprawling/src/mcp_http.rs | an MCP server over HTTP: one request, one message, no session | adapter | R1 | built | sprawling-SPEC.md#8-15 |
| bin::mcp_http::tests | crates/sprawling/src/mcp_http/tests.rs | what the HTTP transport, its session and its redeemed header are held to | adapter | R1 | built | sprawling-SPEC.md#8-15 |
| bin::firstrun | crates/sprawling/src/firstrun.rs | the first screen, where a city goes when nobody said, and handing a URL to the desktop | adapter | P7 | built | sprawling-SPEC.md#8-8 |
| bin::install | crates/sprawling/src/install.rs | putting this binary where a shell will find it, and taking it back out | adapter | P0 | built | sprawling-SPEC.md#8-9 |
| bin::install::search_path_windows | crates/sprawling/src/install/search_path_windows.rs | editing the user search path in the registry under its own type, and telling the desktop | adapter | P0 | built | sprawling-SPEC.md#8-9 |
| bin::install::search_path_elsewhere | crates/sprawling/src/install/search_path_elsewhere.rs | handing back the line a person adds themselves, where no search path is written | adapter | P0 | built | sprawling-SPEC.md#8-9 |
| bin::install::tests | crates/sprawling/src/install/tests.rs | installing twice is installing once, and uninstalling returns the original search path | adapter | P0 | built | sprawling-SPEC.md#8-9 |
| bin::doctor | crates/sprawling/src/doctor.rs | what this machine has against what the city needs, and the verdict for each tier | decision | V4 | built | sprawling-SPEC.md#8-40 |
| bin::doctor::table | crates/sprawling/src/doctor/table.rs | the two tiers item by item: how each one is detected, and how each one is installed here | data | V4 | built | sprawling-SPEC.md#8-40 |
| bin::doctor::screen | crates/sprawling/src/doctor/screen.rs | the report a person reads, and the one question asked per absent item | adapter | V4 | built | sprawling-SPEC.md#8-40 |
| bin::doctor::probe | crates/sprawling/src/doctor/probe.rs | this machine answering: a program on the search path, its version under a deadline, one consented install | adapter | V4 | built | sprawling-SPEC.md#8-40 |
| bin::doctor::tests | crates/sprawling/src/doctor/tests.rs | every row is detectable and either installable or manual, and consent is asked one item at a time | decision | V4 | built | sprawling-SPEC.md#8-40 |
| bin::wire_client | crates/sprawling/src/wire_client.rs | the second client of the wire: one frame out, every frame back, and enrolment from stdin | adapter | P3 | built | sprawling-SPEC.md#8-41 |
| bin::console | crates/sprawling/src/console.rs | what a served city says to the terminal it is running in, and what a line typed there means | decision | P1 | built | sprawling-SPEC.md#8-30 |
| bin::console::language | crates/sprawling/src/console/language.rs | the words a line may use and what each one asks for | decision | P1 | built | sprawling-SPEC.md#8-30 |
| bin::console::terminal | crates/sprawling/src/console/terminal.rs | the listener half no query can answer, and the loop that drives it | decision | P1 | built | sprawling-SPEC.md#8-30 |
| bin::console::tests::helpers | crates/sprawling/src/console/tests/helpers.rs | one scripted terminal both faces share | decision | P1 | built | sprawling-SPEC.md#8-30 |
| bin::console::tests::parsing | crates/sprawling/src/console/tests/parsing.rs | every typed spelling lands where the wire says | decision | P1 | built | sprawling-SPEC.md#8-30 |
| bin::console::tests::terminal | crates/sprawling/src/console/tests/terminal.rs | the listener half, as a person reads it | decision | P1 | built | sprawling-SPEC.md#8-30 |
| bin::keying | crates/sprawling/src/keying.rs | what guards the door this serve opens: nothing, what the operator configured, or one minted for this serve alone | decision | R2 | built | sprawling-SPEC.md#8-22 |
| bin::effect | crates/sprawling/src/effect.rs | what a run's desks left behind: the lines the history takes, and the change the city may not make before them | value | R2 | built | sprawling-SPEC.md#8-24 |
| bin::plan_view | crates/sprawling/src/plan_view.rs | every building's plan, parsed once and re-parsed only when a record says it may have moved | projection | V3 | built | sprawling-SPEC.md#8-34 |
| bin::plan_view::tests | crates/sprawling/src/plan_view/tests.rs | proof that the projection folds records rather than holding a copy of the plan | projection | V3 | built | sprawling-SPEC.md#8-34 |
| bin::views | crates/sprawling/src/views.rs | the fold every query is answered from, and the lines a page reads off it | projection | V3 | built | sprawling-SPEC.md#8-37 |
| bin::views::holding | crates/sprawling/src/views/holding.rs | what the views hold and how one record folds in | projection | V3 | built | sprawling-SPEC.md#8-37 |
| bin::views::answering | crates/sprawling/src/views/answering.rs | every question a page may ask, answered from the fold | projection | V3 | built | sprawling-SPEC.md#8-37 |
| bin::views::commits | crates/sprawling/src/views/commits.rs | which run wrote a commit, folded from the records that announced it | projection | V4 | built | sprawling-SPEC.md#8-41 |
| bin::views::lines | crates/sprawling/src/views/lines.rs | one record rendered as the lines a page reads | projection | V3 | built | sprawling-SPEC.md#8-37 |
| bin::views::tests | crates/sprawling/src/views/tests.rs | five views answer from the record, not from unavailable | projection | V3 | built | sprawling-SPEC.md#8-37 |
| bin::views::governance_tests | crates/sprawling/src/views/governance_tests.rs | who answers and what was answered for the person, and a patch of a commit this city never wrote | projection | V4 | built | sprawling-SPEC.md#8-37 |

### desktop (25) — out of tree: this Windows desktop, offered as an MCP server

Not a workspace member, and excluded in the root manifest on purpose: `unsafe_code` is `deny` here rather than `forbid`, so the Win32 boundary can relax it at the call sites that need it instead of opening a hole in the wall twelve crates stand behind. The seam it arrives through is the one section 8 already describes — an outside application is reached over MCP, not linked in. Card 7.1 settled the protocol, the tool table, the scope file and the refusals; card 7.2 replaced the Windows arm's body and nothing else, so the wire shape a city already speaks did not move. `cargo xtask modmap` does not read these rows, because it parses `crates/*/src`; they are here so that one table still lists every file.

**Every `unsafe` block under `desktop/src/platform/windows/` carries a `SAFETY:` comment stating the precondition that makes the call sound** — the invariant a reader could in principle find false, never a restatement of the call. That condition is the whole reason this package was allowed outside the wall, so it is written here as well as in `desktop-SPEC.md` §8-9. The split the rows below follow is the Humble Object (§9): five modules touch Win32 and hold almost no judgement, and five hold the judgement and touch nothing, which is what makes the easy-to-get-wrong parts testable on a machine with no desktop.

| Module | File | What it owns | Shape | Since | Status | Spec |
|---|---|---|---|---|---|---|
| desktop::main | desktop/src/main.rs | where this server is started from: one argument, one environment variable, one pair of pipes | adapter | V4 | built | desktop-SPEC.md#8-10 |
| desktop::refusal | desktop/src/refusal.rs | what this server says when it will not do something: a stable code and three parts | value | V4 | built | desktop-SPEC.md#8-1 |
| desktop::rpc | desktop/src/rpc.rs | one JSON-RPC 2.0 message per line, read and written | adapter | V4 | built | desktop-SPEC.md#8-2 |
| desktop::tools | desktop/src/tools.rs | the six tools, their schemas, and what each one does not do | data | V4 | built | desktop-SPEC.md#8-3 |
| desktop::scope | desktop/src/scope.rs | what DESKTOP.toml permits, and the three ways it closes | decision | V4 | built | desktop-SPEC.md#8-4 |
| desktop::scope::pattern | desktop/src/scope/pattern.rs | one allowlist line, and what it matches | value | V4 | built | desktop-SPEC.md#8-4 |
| desktop::session | desktop/src/session.rs | one connection: the handshake order, and one reply per request | adapter | V4 | built | desktop-SPEC.md#8-5 |
| desktop::session::tests | desktop/src/session/tests.rs | the handshake, the six names, and a refusal for everything this build cannot do | adapter | V4 | built | desktop-SPEC.md#8-5 |
| desktop::platform | desktop/src/platform.rs | which machine this build is standing on | adapter | V4 | built | desktop-SPEC.md#8-6 |
| desktop::platform::windows | desktop/src/platform/windows.rs | the Windows arm: the desk one connection holds, and which of the six carries out an admitted call | adapter | V4 | built | desktop-SPEC.md#8-6 |
| desktop::platform::windows::reading | desktop/src/platform/windows/reading.rs | what one call's arguments say, each field refused by name rather than defaulted | decision | V4 | built | desktop-SPEC.md#8-6 |
| desktop::platform::windows::fault | desktop/src/platform/windows/fault.rs | the one place a Win32 failure becomes a refusal a caller can act on | adapter | V4 | built | desktop-SPEC.md#8-6 |
| desktop::platform::windows::geometry | desktop/src/platform/windows/geometry.rs | rectangles, the two coordinate spaces, and what a percentage scales a size to | value | V4 | built | desktop-SPEC.md#8-6 |
| desktop::platform::windows::enumerate | desktop/src/platform/windows/enumerate.rs | EnumWindows: which top-level windows exist, and the title, process and bounds of each | adapter | V4 | built | desktop-SPEC.md#8-6 |
| desktop::platform::windows::target | desktop/src/platform/windows/target.rs | choosing exactly one window from what a call named, and refusing rather than picking | decision | V4 | built | desktop-SPEC.md#8-6 |
| desktop::platform::windows::views | desktop/src/platform/windows/views.rs | what a snapshot minted, which generation a window is on, and the action an older view cannot buy | decision | V4 | built | desktop-SPEC.md#8-6 |
| desktop::platform::windows::tree | desktop/src/platform/windows/tree.rs | the UI Automation tree of one window: role, name, ref and bounds | adapter | V4 | built | desktop-SPEC.md#8-6 |
| desktop::platform::windows::keys | desktop/src/platform/windows/keys.rs | the key names this server accepts, and the virtual-key code each one is | data | V4 | built | desktop-SPEC.md#8-6 |
| desktop::platform::windows::act | desktop/src/platform/windows/act.rs | SendInput: one action landing on one window, never a script of them | adapter | V4 | built | desktop-SPEC.md#8-6 |
| desktop::platform::windows::capture | desktop/src/platform/windows/capture.rs | PrintWindow: one window as pixels, and the all-black answer that is a failure rather than an image | adapter | V4 | built | desktop-SPEC.md#8-6 |
| desktop::platform::windows::encode | desktop/src/platform/windows/encode.rs | pixels scaled by a percentage and encoded as png, jpeg or webp | decision | V4 | built | desktop-SPEC.md#8-6 |
| desktop::platform::windows::record | desktop/src/platform/windows/record.rs | start and stop: an mp4 through ffmpeg, or the frame sequence this package's one thread writes | adapter | V4 | built | desktop-SPEC.md#8-6 |
| desktop::platform::windows::clipboard | desktop/src/platform/windows/clipboard.rs | this machine's clipboard, as text and as nothing else | adapter | V4 | built | desktop-SPEC.md#8-6 |
| desktop::platform::elsewhere | desktop/src/platform/elsewhere.rs | every machine that is not Windows, named in its own refusal | adapter | V4 | built | desktop-SPEC.md#8-6 |
| desktop::smoke | desktop/tests/smoke.rs | the real binary answering the exchange the city's own client sends | adapter | V4 | built | desktop-SPEC.md#8-10 |

## 13 Changing this document

- **Structure is add-only.** The topology in §3, the seam list in §4, the shape set in §9 and the module map's column contract change only with an explicit ruling, recorded in the commit that changes them. The contract has moved once: card-8.2 added the seventh column, `Spec`, at the end, where it leaves every other column's position alone.
- **A rejected alternative is recorded where the decision lives**, in the crate's SPEC, rather than in a separate register of regrets.
- **Adding a module row is the registration step**: the row lands in the same change as the file, before it is written.
- **Removing a module row requires a ruling** — `cargo xtask guard` refuses the commit otherwise, because a row that quietly disappears is a rule that quietly stops being enforced.
