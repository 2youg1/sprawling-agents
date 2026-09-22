# ARCHITECTURE — sprawling

> **For someone about to change this code.** It answers what runs, how the
> units are wired, what happens end to end when one piece of work is
> dispatched, what is on disk, what crosses the wire, and how the whole thing
> is verified.
>
> It does not teach the vocabulary ([`docs/glossary.md`](docs/glossary.md)),
> install anything ([`docs/getting-started.md`](docs/getting-started.md)),
> list the rules a change must satisfy
> ([`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md)), or explain how to swap a
> provider ([`docs/operating.md`](docs/operating.md)).
>
> **What is written here is what is true across modules today.** A rule that
> governs one module lives in that module's SPEC and its rustdoc; the history
> of a decision — what was tried, what was rejected, what a gate once caught —
> lives beside the thing it constrains, so that removing the thing removes the
> record. This document is therefore as long as the system is complicated, not
> as long as the project is old, which is what makes *read this before you
> start* an instruction rather than a wish.
>
> **Three kinds of statement, and you can tell them apart.** A figure between
> `xtask:begin` and `xtask:end` markers is written from the code by
> `cargo xtask docnum` and cannot drift. A table marked **machine authority**
> is parsed by a gate, so a disagreement between it and the code is a red
> build. Everything else is prose a person maintains: true when written, and
> checked by review alone — when it contradicts the code, the code is right.

## 1 What runs

One process serves one page, and the page is inside the binary rather than
beside it: `crates/sprawling/build.rs` compresses `target/web-dist` and emits
an `include_bytes!` entry per file, so a server that started has no asset
directory left to lose. The page is `client/`, TypeScript bundled by bun.

**The wire is the seam, not the language.** A second client written against
`channels::wire` in any language is a supported thing to build; what the
shipped one happens to be written in is a replaceable fact.

```
                      one machine
┌──────────────────────────────────────────────────────────────┐
│  sprawling (one binary)                                      │
│                                                              │
│   bin::assembly ── the only omniscient point: it holds every │
│        │           concrete type, samples the clock, hands   │
│        │           out seeds, and starts every task          │
│        │                                                     │
│        ├── runtime ── turns, tools, sandbox, watchdog, fork  │
│        ├── collab  ── inbox, signals, claims, delegation,    │
│        │              workshop, fan-in, pull requests        │
│        ├── city    ── buildings, residents, rooms, archive,  │
│        │              library, schedule                      │
│        ├── eval    ── suites, probes, scoring, metabolism    │
│        ├── browser ── WebDriver BiDi sessions, snapshots     │
│        ├── protocol── MCP outbound, ACP inbound              │
│        ├── memory  ── Ledger, CAS, projections, git, Vfs     │
│        ├── gateway ── routing, dialects, market, cost,       │
│        │              credentials                            │
│        └── channels ─ WebSocket server, Command/Query/Event, │
│                 │     admission                              │
│                 ▼                                            │
│       client (TypeScript, served from inside the binary)     │
└──────────────────────────────────────────────────────────────┘
        │                                    │
        ▼                                    ▼
   the city directory                 model providers, MCP servers
   (a tree on this disk)              (only over one gateway endpoint)
```

Each unit's duty is stated here and nowhere else in this document; `kernel`
is absent from the figure because everything above depends on it and nothing
it holds is reached from outside the process.

The address the server binds is `kernel::consts_policy::DEFAULT_AT` unless a
caller overrides it. The value is not repeated here: the CLI defaults, the
first-run screen, the installer and `README.md` all read that one constant,
because the address a person is told and the address actually bound may not
be two spellings (sprawling-SPEC.md section 8-2b).

## 2 The stack, and what each choice costs

The pinned versions live in `Cargo.toml`, which is the authority; this
table says **why** each choice is there and what it costs. Version numbers
are deliberately absent — a reader who needs one reads the manifest, and a
number repeated here would be a second home for a fact Dependabot moves
every week.

| Concern | Choice | Why it, and what it costs |
|---|---|---|
| Language | Rust, edition 2024, toolchain pinned in `rust-toolchain.toml`, MSRV 1.97 | The invariants this design cares about are expressible as types, and `#![forbid(unsafe_code)]` holds workspace-wide. Cost: compile times, and a client that has to be built before the binary that embeds it. |
| Async runtime | `tokio`, only in `channels` and the binary | The turn loop is synchronous on purpose — a decision that awaits is a decision that interleaves. Async stops at the process boundary. Cost: one blocking HTTP call per model request, paid inside a worker rather than a reactor. |
| Inbound WebSocket | `axum` with its `ws` feature (`crates/channels/Cargo.toml`) | It carries the WebSocket implementation itself, so the served protocol has one version authority rather than two. |
| Outbound WebSocket | `tokio-tungstenite`, no default features | A client rather than a server: it drives the browser over WebDriver BiDi and is what an integration test speaks the wire with. TLS termination is deliberately not here — a face reachable beyond this machine refuses to serve without a credential, so certificates stay the proxy's (channels-SPEC section 8-41). |
| HTTP client | `reqwest`, blocking, `rustls`, no default features | One client for the whole workspace: providers and HTTP-reached MCP servers. Two clients would mean two TLS stacks in one binary. |
| Client | Solid and Effect, bundled by Vite, driven by bun | Two runtime dependencies and no framework runtime beyond them: Solid compiles its templates away, and Effect is used for one job, decoding the wire. Cost: a JavaScript toolchain has to be present to build the page the binary embeds. |
| History | JSONL segments, appended, chain-verified | A history a person can read with `tail` and a machine can verify byte by byte. Cost: the Ledger's throughput is the city's throughput (§11). |
| Content store | BLAKE3 | One hash for the whole library: content addressing and `IdemKey` derivation. Identical content is stored once. |
| Restoration | `git2`, vendored libgit2 | Git is the restoration authority for tracked files, so a discarded file points at a checkpoint commit. Also one worktree per reviewing run. Cost: a C library in the tree, vendored so there is no system dependency. |
| Sandbox | `wasmtime` + `wasmtime-wasi`, wasip1 only | Fuel-metered execution with **no socket host implementation** — the Python arm's mechanical proof that it cannot reach the network. Cost: an optional feature; a build without it refuses tool execution in three parts rather than pretending. |
| Credentials | `keyring` (platform credential service), `secrecy`, `zeroize` | Plaintext lives in the operating system's own vault, never in a file we wrote. `sha2` is present for one external protocol fact: PKCE mandates SHA-256. |
| Entropy | `getrandom` | OS entropy for the PKCE verifier and the login state. It is *not* the seeded RNG the simulator uses, and must never become it. |
| Serialisation | `serde`, `serde_json`, `toml` | JSON on the wire and in the Ledger because the receiver may be a browser and a person still has to read it. TOML for configuration a person edits. |
| Errors | `thiserror` | One error shape, `AxError`, defined in `kernel::error` and mapped at every crate boundary. |
| Release profile | `opt-level = "z"`, `lto = "fat"`, one codegen unit, symbols stripped, `panic = "abort"` | Crash-only delivery: there is no unwinding path to maintain, because there is nothing to catch. `"z"` rather than `3` on a measurement whose criterion was written before the readings existed — the manifest records both arms. |
| Dependency count | <!-- xtask:begin dependency_count -->402<!-- xtask:end --> packages in `Cargo.lock` | The one number in this table that is a fact about the whole graph rather than about one choice. Listed by `sprawling status --deps`, licence-checked one by one by `cargo deny` against `deny.toml`. |

**Verification tools**, kept out of the shipped binary: `proptest`
(properties before examples), `insta` (golden output), `trybuild` (proof
that something cannot be expressed), `kani` (bounded proof, Linux CI),
`cargo-mutants` (do the tests bite), `cargo-fuzz` (parsers against hostile
bytes).

## 3 The units and the dependency law

Crates are not a reuse mechanism here. They exist so that **the dependency
rules are executed by the compiler**: the wall `pub(crate)` builds is drawn
at the crate boundary, so each crate is a wall that actually closes.

The block below is the **machine authority** for those walls, read by
`cargo xtask depmap`. Actual edges must be a *subset* of it, so a hidden
edge is a red build rather than a discovery. It is also the unit list:
counting its rows is how you learn how many there are, which is why no
number appears in this sentence.

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
sprawling: kernel, memory, gateway, runtime, collab, city, eval, browser, protocol, channels
```

What each unit owns is stated once, in §1's figure. It is not repeated
here: the two lists drifted apart while both were maintained by hand, and
`collab` was carrying a duty in one that its source had never had.

**Three rules.** Dependencies point inward, and never back. A seam declares
its trait in the inner layer and implements it in the outer one, so
`kernel` can define what a Ledger *is* without knowing where it is written.
And splitting into crates buys compiler-enforced layering, not reuse —
nothing here is published.

**Three kinds of edge**, and telling them apart is what makes the
repository readable:

| Edge | When it exists | Example |
|---|---|---|
| Dependency | compile time | `runtime → kernel` |
| Assembly | run time, only in `bin::assembly` | the upload sink in `channels::server` receiving `memory::cas` |
| Event | anywhere a `kernel::Ledger` handle is held | writing `tool_result` after a tool runs |

`runtime` has the widest fan-out — three crates at once. It may **use**
their interfaces and nothing more; the moment a runtime module starts
passing concrete types between `memory` and `gateway`, that edge moves up
into the assembly layer.

`xtask` and `citysim` are workspace members outside the product graph.
**citysim drives the turn loop a second time**: `runtime::run::drive` with
simulated adapters — a scripted model, scripted tools, an in-memory Ledger
— which is how a script reproduces a run. It stops below `bin::assembly`,
whose `RunWorker` builds its model adapter out of the endpoint book rather
than receiving one; the dispatch policy above that line is held by that
module's own tests. `sprawling` carries a lib target so the policy is at
least *reachable* — an integration test enters by the same door
`channels::server` uses — and inverting the model seam is what a seeded
scenario would still need.

## 4 Seams

A seam is a trait declared in the inner layer and implemented outside it.
**One adapter is a hypothetical seam; two make it real** — so every seam
ships with a second implementation.

This table is a **machine authority**: `cargo xtask depmap` refuses a
`pub trait` declared anywhere but the files it names.

| Seam | Declared in | Production adapter | Second adapter |
|---|---|---|---|
| `kernel::ledger` | crates/kernel/src/ledger.rs | memory: jsonl segments with tail recovery | citysim: in-memory Ledger |
| `kernel::tool` | crates/kernel/src/tool.rs | runtime tools, collab tools, browser, protocol | citysim: scripted tools |
| `kernel::model` | crates/kernel/src/model.rs | gateway: native and endpoint | citysim: scripted model |
| `runtime::sandbox` | crates/runtime/src/sandbox.rs | wasmtime with fuel metering | pass-through and fault doubles |
| `browser::port` | crates/browser/src/port.rs | WebDriver BiDi session layer | two shipped transports and an offline replay |
| `protocol::mcp` | crates/protocol/src/mcp/outbound.rs | stdio child process, or HTTP | `ScriptedOutbound` for offline replay |

The *second adapter* column has no checker: a seam whose double was
deleted would still read as real here. That is a known hole, not a
guarantee.

**Two inner seams** stay `pub(crate)` because nothing outside their crate
needs them: `memory`'s `Vfs` (real filesystem / deterministic power-loss
model) and `gateway`'s `Vault` (platform credential service / in-session
store).

**Deliberately not seams**: `gateway::dialect` is a pure function and needs
no trait; the internals of `city`, `collab` and `eval` have one
implementation each and are driven from outside by citysim; `git2` is used
directly, because an interface with one implementation is decoration.

## 5 One dispatch, end to end

This is the path everything else supports. Following it once explains more
than any diagram of boxes.

1. **The page sends a Command.** A person fills in the control surface —
   address, what to produce, what counts as done — and the client's socket
   sends `Command::Dispatch` over the WebSocket. The frame carries no
   ceiling of any kind: nobody can price a piece of work before it runs,
   and the one brake is `Halt`.
2. **`channels::server` decides whether to accept it.** Two pure
   judgements — may this address be bound, may this peer be accepted — with
   the socket code that surrounds them making no judgement at all.
3. **`bin::assembly` turns it into work.** This is the only place that
   samples the clock, so the timestamp enters as a parameter from here on.
   A worker takes the dispatch and answers every refusal it can owe before
   it writes anything: the reserved subtree, a halted scope, rules that
   will not load, and a tag with no model behind it are all decided by
   `agree_to_work`, which reads and writes nothing. **Opening the room is
   the first thing this city puts on disk for a dispatch**, so work nobody
   could take leaves no room behind for a person to find.
4. **The city writes `run_started` before anything happens.** Every effect
   becomes an event first; that ordering is the design's load-bearing rule,
   not a logging preference.
5. **`runtime::prefix` assembles the frozen prefix** in four segments —
   city, building, resident, run — from `city::spine_files`, `city::policy`
   and `city::resident`. Assembling it is itself an event, and the result
   is frozen for the whole run.
6. **`runtime::catalog` decides what the model may see**: the three
   built-in tools, the collaboration tools this building admits, the skills
   its reading room allows, and any MCP tools discovered from the
   building's `CONFIG.toml`. `city::neighbourhood` is scanned in the same
   breath, so the run also knows which addresses it can reach and who
   stands at them — without it, `signal` takes an address the model has to
   have been told.
7. **`runtime::turn` enters its typestate**: Assembling → Calling →
   Applying → Settling, with four cancellation-safe points. An interruption
   inside a phase cannot be spelled.
8. **`gateway` makes the call.** `gateway::router` picks the endpoint
   attached to this tag; `gateway::dialect` translates the canonical
   Anthropic-shaped conversation into the provider's dialect;
   `gateway::credential` redeems a `secret:realm/name` reference into a
   header at the last moment. Nothing here holds a concurrency limit: the
   module that did was deleted for having no caller, and what it would
   take to grow one back is recorded where it was removed
   (gateway-SPEC.md section 8-6).
9. **The reply is scanned before it is recorded.** `runtime::redact` puts
   model output through the same secret scan as everything else, so a key a
   model repeated does not become permanent.
10. **Tools run behind gates.** `kernel::gate` answers with an exhaustive
    verdict — allowed, refused in three parts, or escalated to a person.
    `memory::checkpoint` puts a git fence before the wave and scans the
    worktree after it, so anything that disappeared becomes a
    `file_discarded` event carrying the way back.
11. **The result comes back shaped.** `runtime::pipeline` builds the result
    envelope — clock stamp, network reminder, any steer a person sent — and
    `runtime::compaction` shortens what is too long, always reporting how
    much it dropped.
12. **Everything lands in the Ledger, and the views follow.**
    `memory::hot` and `memory::attribution` fold the same event stream into
    what the pages ask for. The server pushes each event; the client folds
    it into what it believes. The same fold, on both sides of the wire.
13. **A signal reaches whoever it names, working or not.** After the run
    freezes, each signal it sent is recorded and then delivered. A
    steer-kind signal slips under the door of a run that is already going,
    landing at that run's next safe point with `@` and the sender's address
    in front of it; anyone else who was spoken to is *knocked* —
    `bin::assembly` starts a run for them, whose brief names the resident
    who spoke. Only the person's own entrance can render as `user`, which
    is what makes an answer go to the right place. A knock addresses a
    resident, never a frozen run: history is read, not woken.

When the process dies mid-call, `sprawling resume` verifies the chain,
closes tool calls whose outcome was lost as *unknown* rather than as
failed, and reports what waits for a person.

### When it does not go through

The walk above is the path that succeeds. Every way it can stop is one
shape, and that shape is a type rather than a convention.

**A failure that does not say what to do next cannot be spelled.**
`AxError` carries seven wire fields in declaration order — which is rule 6
of §10, so an error is as replayable as an event: a stable `code`, the
`action` and `subject` it failed on, `nearby` candidates, a `recovery`
sentence, whether it is `retriable`, and the gate's own refusal when a gate
is what stopped it. **The model is the audience**, so `nearby` and
`recovery` hold directly executable information rather than apologies. Both
constructors return an `ErrorDraft`, and `with_recovery` is the only way
across to an `AxError` — a failure with no next step is unconstructible,
not merely discouraged.

**A gate answers one of exactly three ways.** `GateOutcome` is `Allow`,
`Deny` carrying the refusal, or `Ask` carrying the question. Every caller
decides all three, so a door that *starts* asking a person is a compile
error at every call site rather than a behaviour that changes underneath
them. Exactly one door answers `Ask` today, and the refusal conformance
matrix asserts that count rather than trusting it. The roster of doors is
data (`kernel::gate::DOORS`), so a door added to the enum is a door the
matrix judges, and one without a sample does not compile.

**A refusal has three parts**: `rule` — what the rule is; `violation` —
what broke it; `alternative` — what to do instead. These are the same
three parts every gate in `xtask` reports, because a refusal a person reads
and a refusal a builder reads are one shape, not two.

**A halt names a scope, never a run**: the city, a building, or a
workshop. A frozen run is history, and history is read rather than woken.

## 6 On disk

A city is one directory. Copy it and it is the same city; delete it and
nothing outside it changes.

The tree below is the **trunk, not the full set**. Every path this product
writes is derived by `kernel::layout` from a city root and an address, and
that module is the authority; drawing the complete list here would be a
second home for it, and the one a person reads would be the one that went
stale.

```
<city>/
├─ .sprawling/                 the city's own reserved subtree
│  ├─ ledger/                  the only history — jsonl segments, appended, chain-verified
│  ├─ cas/                     content-addressed store, BLAKE3, one copy per content
│  ├─ worktrees/               one git worktree per reviewing run, objects shared
│  ├─ library/                 skills more than one building admits
│  ├─ CONFIG.toml              city layer of the three-layer configuration
│  └─ FILTERS.toml             what this scope keeps out of a transcript
└─ <building>/                 one building, one line of business
   ├─ .sprawling/              the building's reserved subtree: what governs it
   │  ├─ RULES.toml            what the building is and may do: one parsed file
   │  ├─ CONFIG.toml           building layer, including its MCP servers
   │  ├─ skills/               skills only this building admits
   │  └─ sessions/             the Ledger's projection of what ran here, disposable
   ├─ Roadmap.md               the plan tree, and the denominator of every progress reading
   ├─ Memo.md                  decisions and corrections
   ├─ Handoff.md               what the next session needs
   ├─ Archive/                 work this building has finished with, by kind and day
   └─ <room>/                  one session's workplace, named by the person who started it
      ├─ URBANITE.md           who this resident is and how it works
      └─ JOB.md                the task for this session
```

**One rule, applied at every scope: what governs a scope lives in that
scope's `.sprawling/`, and no write domain reaches it.** `is_reserved`
answers true for an address with `.sprawling` in any segment, so the check
is one predicate in `kernel::address` rather than a list of protected file
names. An agent therefore cannot edit its own accounting, its own
configuration, its own building's rules, or the history of what it did.

A run's write domain is what its building's `RULES.toml` declares, and
**the whole building when it names no prefix** — which is the shipped
template. The room is where a session works, not the boundary that
contains it.

**`Roadmap.md` is a tree, and the `plan` tool is what writes it.** The
index column is a path — `2.3.1` hangs under `2.3` — so one file states a
multi-level plan without a second file to say how the levels relate.
`Weight` is a ratio among the rows sharing a parent and `Needs` names what
must finish first, which is what makes a ready set computable. A resident
divides its own branch and cannot reach past it: `kernel::share` has no
constructor, so a share exists only by dividing another one, and the total
is the whole plan whatever the plan turns into. Only leaves are counted; a
branch's work is its children.

## 7 The wire

One WebSocket, three kinds of frame, and a schema hash that both ends check
on connect: a page from a different build refuses rather than misreads.
`WIRE_V` is <!-- xtask:begin wire_v -->34<!-- xtask:end -->.

| Frame | Count | What it is |
|---|---|---|
| `Command` | <!-- xtask:begin command_frames -->29<!-- xtask:end --> | something a person wants done: dispatch, steer, cancel, approve, halt, raise a building, attach an endpoint, set a goal the city works towards, write a document that governs the city |
| `Query` | <!-- xtask:begin query_frames -->34<!-- xtask:end --> | something a page wants to know: the city, one run, approvals, cost, the ledger, archive, discards, inboxes, which run wrote a commit, who answers and what was answered for the person, and one file's patch text |
| `Delta` | — | what a model is saying while it is still saying it: no sequence number, never written down, and a client that missed one has lost nothing |
| `Event` | the Ledger's own kinds | what happened, pushed as it happens |

Two properties are worth stating because they are enforced by types rather
than by review. A `Command` carrying a credential **cannot be serialised**:
`Sealed<T>` has no `Serialize`, and the `PutSecret` payload of the remote
command type is an uninhabited type, so entering a key over the network is
not a request that can be spelled. And every `Query` must be answered
exhaustively — the answer match has no catch-all, so adding a query without
answering it does not compile.

## 8 Where to change what

The table this document never had. Each crate's SPEC settles the interface
of its own modules; what belongs here is only the first hop, so that
nobody has to read a hundred module rows to find which SPEC to open.

The third column names what turns red when the change is wrong — which is
the part worth knowing before starting, not after.

| To change this | Go here | Held by |
|---|---|---|
| a new event kind, or a payload | `kernel::event` + kernel-SPEC | the kind set is closed; `memory` fixtures compare bytes across platforms |
| a new `Command` or `Query` frame | `channels::wire` + channels-SPEC | `WIRE_V` must rise, and every `Query` must be answered or it does not compile |
| what a model may call | `runtime::catalog`, tools in `runtime` or `collab` | `kernel::tool` is the seam; a tool with no conformance suite is not a seam |
| how a provider is spoken to | `gateway::dialect` + gateway-SPEC | a pure two-way translation with the canonical shape in the middle |
| how a login begins, finishes, renews | `gateway::credential`, `gateway::oauth_profiles` | plaintext may reach only the platform vault; `secret` gate reads every boundary |
| where a city keeps a file | `kernel::layout` | the reserved subtree is out of every write domain, by one predicate in `kernel::address` |
| what a building may do | `city::policy` (`RULES.toml`) + city-SPEC | a run's write domain is what its building declares |
| a crate depending on another | the `depmap` block in §3 | actual edges must be a subset; a hidden edge is a red build |
| a new seam | §4, and the trait's file | `depmap` refuses a `pub trait` outside the files §4 names |
| a new module, or a deleted one | `architecture.toml` | `modmap` refuses a file with no entry, and an entry whose file is gone |
| a platform the release ships | `xtask::platform`'s `PLATFORMS` | one row per platform; the npm scope and the bare root name are asserted there |
| the page | `client/` + client-SPEC | its own lint, typecheck and tests; the bundle is measured against a byte budget |
| a gate itself | `xtask/` + xtask-SPEC | `guard` asks for a `Verdict:` trailer when a gate loosens in the commit it would have refused |

Two documents sit beside this one rather than inside it: operating a
city — swapping a provider, pointing at another MCP server, running a
local model — is [`docs/operating.md`](docs/operating.md), and what a
change must satisfy before it merges is
[`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md).

## 9 Seven shapes

Every module instantiates exactly one of these. The classification earns
its place by what it catches: a module that cannot name its shape usually
holds two things that want to be separate files.

| # | Shape | The test for it |
|---|---|---|
| 1 | decision | no I/O, no clock, no global state; returns an exhaustive enum, never a bool |
| 2 | value | invariants enforced at one construction point; private fields; no setters |
| 3 | port | a trait on a seam, delivered with its conformance suite |
| 4 | adapter | thin, no policy: swapping in a second implementation changes no policy |
| 5 | typestate | phase changes enforced by types; no method returns a previous phase |
| 6 | data | data only, no branches. Editing it is editing behaviour |
| 7 | projection | folds the event stream into a view; deleting it and rebuilding gives the same bytes |

**The Humble Object is the recurring move**: the hard-to-test end is
stripped to nothing and the thick end stays pure. `runtime::watchdog`,
`gateway::endpoint` and the sandbox adapters are all instances of it.

### Making illegal states unrepresentable

Each of these is a type, not a slogan, and each has a compile-failure
counterexample in the test suite — because "unrepresentable" is itself a
claim that needs testing.

- `EventRef` has no public constructor and no serde ⇒ **a forged event reference cannot be spelled**.
- `Completion::Done` always carries `Evidence`, and `Completion` has no serde ⇒ **a deserialised "finished" cannot be spelled**.
- A `Delegate` value has no `delegate` method ⇒ **a grand-delegate cannot be spelled**; delegation is one level deep.
- `Discard`'s constructor requires a `Restoration` ⇒ **a deletion with no way back cannot be spelled**.
- `Sealed<T>` has no `Serialize` ⇒ **entering a credential remotely cannot be spelled**.
- `UnplannedProgress` has no `ratio` method ⇒ **a percentage with no denominator cannot be drawn**; there is nothing to call.
- `Share` has no constructor and no arithmetic; the only producer is `Share::split`, which consumes what it divides ⇒ **weight cannot be minted**, so a branch cannot be given more than its parent had and the total is always the whole plan.
- `Held` has one private field and two consuming methods ⇒ **a claimed plan node cannot be put down silently**: it is finished with evidence or stopped with a cause, and a run that simply ends spends it on `FrozeWithoutEvidence`.
- `Pursuit::declare` takes the depth-zero position and a `Delegate` cannot produce one ⇒ **a sub-agent cannot set the city working until the work runs out**.

These expectations are byte comparisons against a compiler's output, so the
toolchain is part of them. What to do when an installed component changes
that output is in [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md); it is
a fact about running the tests, not about the design.

## 10 Determinism and hardening

The whole city runs as real code, single-threaded, on a virtual clock,
driven by a fixed script. That is not a testing convenience — it is the
property that makes a failure replay exactly, and these seven rules are its
admission conditions. A seed is what a random scenario batch would need;
there is no random source in the simulator today to seed.

| # | Rule | Held by |
|---|---|---|
| 1 | Decision paths iterate `BTreeMap`; never a hash order | review, plus the citysim determinism scenarios |
| 2 | Time arrives as a parameter; the one sampling point is `bin::assembly` | `clippy.toml` disallowed methods |
| 3 | One spawn point | review; the two exceptions are runtime's concurrent wave, with structured cancellation, and the driving lanes in `bin::serving::pool`, each of which lives exactly as long as the run it drives |
| 4 | Seeded RNG handed out from one place | assembly derives per session |
| 5 | Execute in parallel, account in series, ordered by `seq` | the Ledger port owns `seq` and `prev` |
| 6 | Ledger payloads hold integers; timestamps are integer milliseconds; field order is declaration order | cross-OS byte fixtures |
| 7 | `IdemKey` derives from `(run, seq, normalised action)` — never from a clock or a random number | property tests |

Rules 1 and 6 together give a checkable property: **the same event sequence
replays byte-for-byte identically on any machine.**

**Hardening** is compile-time, workspace-wide, and identical in tests and
production except where a test module relaxes it locally: no `unwrap`,
`expect`, `panic!`, `todo!`, `unreachable!`, bare indexing or slicing;
arithmetic is checked; narrowing goes through `TryFrom`; `as` casts are
denied; `unsafe_code` is forbidden outright. Money and quantities are
integer newtypes (`UsdMicros`, `Tokens`, `ByteLen`, `Seq`), and floats stay
out of every decision path.

**The most fragile point in the design is one paragraph long.** A database
handle, one `Instant::now()`, or a bare spawn inside `kernel` disables
replay, formal verification and deterministic simulation at the same time.
The gates hold that line so the property survives a builder who has never
read this file.

## 11 How this is verified, and what it costs

Eleven layers, each catching what the layer above cannot. They deliberately
do not overlap: overlapping verification reads as more coverage than it is.

| Layer | Catches | Today |
|---|---|---|
| V0 unrepresentable | a whole class of error moved out of what can be written | <!-- xtask:begin compile_fail_cases -->16<!-- xtask:end --> compile-failure counterexamples |
| V1 types and lints | null, overflow, silent truncation, hidden panics | workspace lints, `-D warnings`, `--all-features` |
| V2 unit and property | a function wrong across a class of inputs | <!-- xtask:begin test_functions -->2030<!-- xtask:end --> test functions, properties before examples |
| V3 conformance | a second adapter behaving unlike the first | one suite per port, except `browser::port`, whose suite only ever ran against the replay it was written beside (browser-SPEC.md#8-6) |
| V4 fuzz | parsers meeting hostile bytes | <!-- xtask:begin fuzz_targets -->6<!-- xtask:end --> targets: address, locator, truncated ledger tail |
| V5 formal | termination, absence of overflow, monotonicity | 3 of 3 kani harnesses proved, Linux CI — every proposition in the roster has an unbounded domain and a solvable shape |
| V6 deterministic simulation | components each correct and wrong together | citysim, <!-- xtask:begin citysim_scenarios -->7<!-- xtask:end --> scenario files, failures replayed from their script |
| V7 mutation | tests that do not bite | `cargo-mutants`, by `just mutants` |
| V8 cross-version, cross-OS fixtures | byte drift after an upgrade or a platform change | golden ledgers in `fixtures/` |
| V9 end to end | the thing a person actually wants to do | the real client in a real browser against a real server, on a developer machine |
| V10 adversarial | a promise the door makes that holds on the traces we wrote and not on the ones we did not | `adversary/`, out of tree, in Lean, driving the shipped binary over the wire |

**V10 is not a gate, and the difference is load-bearing.** §8 says the wire
is the whole API and that a second client writes against it; `adversary/`
exercises that permission by writing a third one outside the workspace, in
another language, to attack rather than to use. It is reached by
`just adversary` and by a schedule, never by `just check` — on a machine
with no Lean toolchain, `just check` behaves byte for byte as it does where
the directory is absent, and `just adversary` prints one line and succeeds.
What it buys that V2 cannot is quantification over traces: V2 proves that
the paths we thought of hold, and V10 asks whether the door's stable error
codes survive any prefix, one halt, and any suffix. What it has found, and
what each finding cost to fix, is recorded in
`adversary/adversary-SPEC.md` section 4 — beside the mechanism rather than
here, so that retiring the mechanism retires its record.

**Four gaps, named rather than hidden.** CI has no browser driver, so V9 is
a command a developer runs rather than a gate. The isometric city compares
display lists rather than bitmaps: the preconditions for bitmap comparison
are paid for — placement is a pure function of the id, painter order is
total, projection and its inverse are exact — but there is no rasteriser.
V6 stops below `bin::assembly` (§3): a scripted scenario reproduces a run,
not a dispatch, because `RunWorker` builds its model adapter instead of
receiving one; what holds the dispatch policy is that module's own tests,
plus the integration tests the lib target makes possible. And the tree
holds 3 kani harnesses, all of which CI proves in under a minute each — a
harness that builds a `Vec`, a `String` or a `BTreeSet` gives CBMC loops it
cannot bound, and one over symbolic non-linear arithmetic gives the solver
work that grows with the data it walks, so propositions of those two shapes
are held by the `#[test]` and the proptest instead (kernel-SPEC.md section
2). The `// not-proved:` marker and the reader that honours it stay, and
today no harness carries one.

### The performance register

Sizes are gated because a byte count does not depend on how busy the
machine was. Wall-clock figures are measured, reported with the machine
that produced them, and never gated: a slow runner is not a defect, and a
gate that says it is teaches people to ignore gates. **The full register is
`xtask/budgets.toml`**, which is the authority; `cargo xtask budget` prints
it, and the readings below are written here by `cargo xtask docnum` rather
than typed.

| Metric | Budget | Measured | Gated |
|---|---|---|---|
| Client bundle, gzipped | ≤<!-- xtask:begin budget_bytes:frontend_artifact -->2,097,152 B<!-- xtask:end --> | <!-- xtask:begin budget_reading:frontend_artifact -->311,050 B<!-- xtask:end --> — <!-- xtask:begin budget_headroom:frontend_artifact -->6.7×<!-- xtask:end --> headroom | yes |
| The installed binary | ≤<!-- xtask:begin budget_bytes:release_binary -->134,217,728 B<!-- xtask:end --> | <!-- xtask:begin budget_reading:release_binary -->11,523,584 B<!-- xtask:end -->, client included | yes |
| Resident memory, one session | ≤<!-- xtask:begin budget_bytes:session_resident -->31,457,280 B<!-- xtask:end --> | <!-- xtask:begin budget_reading:session_resident -->4,292,608 B<!-- xtask:end --> idle | no: the counter means something different on each platform |
| Ledger append plus fsync | p50 ≤5 ms, p99 ≤20 ms | 0.97 ms / 1.61 ms on one NVMe machine | no |
| Projection rebuild | ≥50,000 records/s | about 493,000 records/s on the same machine | no |
| Prefix assembly | ≤1 ms | 0.022 ms for 16.5 KB over four slots | no |
| Runs driving at once | 4 lanes | one thread per run, and one accounting thread taking every write | no: it is a wall this city sets, not a measurement |
| Kernel mutation score | ≥90% | by `just mutants` | by that command, not by `just check` |

The two size rows are also rendered as the badges in `README.md`, from this
same reading — `cargo xtask badge --write`, which `just dist` ends with.
Nobody types a size into a document.

**One honest trade.** With network and model time removed, the throughput
ceiling of a city is the throughput ceiling of its Ledger. That is the
price of "the Ledger is the only history", stated in the open. The first
wall is one this city sets itself: a building working towards a goal drives
four runs at a time, because a lane past the provider's own admission
ceiling would only park a thread there. Then come the walls outside:
provider-side rate limits, Ledger fsync, worktree disk, file-descriptor
limits, then the blocking pool. RAM is not among them. **Runs are driven in
parallel and accounted for in series** — every line a lane writes crosses to
the one thread that owns the Ledger and waits there — so concurrency buys
model time back and buys nothing from the Ledger.

## 12 Changing this document

- **This document is as long as the system is complicated, never as long
  as the project is old.** What belongs here is what is true *across
  modules today*. A rule that governs one module belongs to that module's
  SPEC and its rustdoc; the history of a decision — what was tried, what a
  gate once caught — belongs beside the thing it constrains, so that
  retiring the thing retires the record. Anything whose length grows with
  the calendar rather than with the design has another home, and that is
  what keeps *read this before you start* a reasonable instruction.
- **A rejected alternative is recorded where the decision lives**, in the
  crate's SPEC, rather than in a separate register of regrets.
- **Three kinds of statement, and a reader may tell them apart.** A figure
  between `xtask:begin` markers is written from the code by
  `cargo xtask docnum` and cannot drift. A table marked **machine
  authority** is parsed by a gate, so a disagreement with the code is a red
  build. Everything else is prose a person maintains — and where it
  contradicts the code, the code is right and this document is the defect.
- **Structure is add-only.** The topology in §3, the seam list in §4, the
  shape set in §9 and the fields of `architecture.toml` change only with an
  explicit ruling, recorded in the commit that changes them.
- **The module map is `architecture.toml`**, not a section here. It is the
  machine's data face: one entry per module file, read only by
  `cargo xtask modmap`, and asked for one module at a time rather than read
  through. Keeping it here cost this document seven hundred lines and gave
  every entry a position that a person maintained.
- **Adding an entry is the registration step**: it lands in the same change
  as the file, before the file is written.
- **Removing an entry requires a ruling** — `cargo xtask guard` refuses the
  commit otherwise, because an entry that quietly disappears is a rule that
  quietly stops being enforced.

