# sprawling-agent-protocols

The two protocols the city speaks with programs outside it, in three directions:

- **MCP out.** A resident calls the tools of an external server a building configured. The bytes travel over stdio, HTTP or SSE, and every tool call lands on the same `kernel::tool` seam as a local tool, so it enters the taint ring and has no unwrapping face here.
- **ACP in.** An editor drives the city as an agent. A request produces nothing until it has been authenticated and reduced to an ordinary dispatch, and what goes back is progress, not a conversation.
- **ACP out.** The city is the ACP client of an agent the person consented to. The five official harnesses are built-in entries of the agent catalog, and [`catalog/registry.json`](catalog/registry.json) is the snapshot of the ACP registry this build ships.

This is how a subscription reaches the city: the person signs in inside the agent, and the city records what the agent reports without governing its tools (`crates/gateway/Spec.lean` §8-5 puts the same boundary on the other side).

The package uses no `async`, holds no file handle, and names no vendor except the one OAuth broker `mcp::broker` calls. Whether a call may leave the machine is the egress door in `kernel::gate`, and what comes back is `kernel::taint`'s; `crates/agent_protocols/Spec.lean` §7 lists the neighbour that owns each of those decisions. Package `sprawling-agent-protocols`, library `agent_protocols`.

## How it is shaped

`mcp` (adapter) reaches a server: `handshake` is the one authority for the lifecycle `initialize` → `notifications/initialized` → ready, `tools` lists, names and calls, `reading` is the one reader of a line-framed connection and the ceiling above which a peer is refused, and `stdio`, `http` and `sse` are the three transports. `mcp::link` is one reachable server whichever transport reaches it, carrying the module a reader opens when it misbehaves; `mcp::redeeming` holds the name/value pairs a building wrote beside a server with every `secret:realm/name` reference redeemed; `mcp::broker` is the one module allowed to know which broker holds an outside application's OAuth.

`acp` (decision) is the inbound direction: `Incoming::parse` is the only constructor of a request, and `admit` is the only thing that turns one into a dispatch.

`harness` (adapter) is the outbound ACP half. `entry` is one agent entry and the digest a consent binds (data), `catalog` reads the registry index and the shipped snapshot, `roster` picks which agent a word names from the person's rows and the five built-in ones (data), `paste` reads a pasted command line or block, `detect` finds agents this machine shows evidence of without running anything (decision), `environment` is the whitelist a child sees (decision), and `process` with `session` starts a consented agent and drives one ACP session.

The crate declares one public trait, `Outbound` in `mcp/outbound.rs`, and it is the `agent_protocols::mcp` seam of [`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) §4: stdio, HTTP and SSE are the production adapters, and `ScriptedOutbound` answers a request from a recorded table keyed on its method and parameters — never on the JSON-RPC id, which `Rpc` mints fresh. The Ledger lines are the assembly layer's: this crate produces a `ToolOutcome`, and `accounting` records `tool_called` and `tool_result`, which is what a replay reads back instead of calling the server again.

## When it fails

A refusal is one `kernel::AxError`: the action, the subject, a stable code and a recovery. §12 of the SPEC holds the whole table with what makes each code possible or impossible to prevent.

- `E_INVALID_ARGS` — a call whose arguments carry a float, refused before it is sent with the JSON path of the offending field; an inbound request missing a field; a tool registered without a deadline.
- `E_WIRE_MISMATCH` — an answer or a listing whose shape this build cannot read, a message or HTTP body over `MESSAGE_CEILING` (8 MiB), or a stream that stopped mid-message. An oversized message is refused whole and never truncated and parsed.
- `E_TOOL_UNAVAILABLE` — the server answered with a JSON-RPC error, or with `isError: true`; the tool's own words travel in the subject, cut at 4 KiB on a character boundary.
- `E_TOOL_OUTCOME_UNKNOWN` — `isError` together with `_meta["sprawling/effect-unknown"]`: the server itself says it does not know whether its work took effect.
- `E_TIMEOUT` — no answer inside the call's patience. The request had already gone out, so the failure is marked `Retry::Unknown`.
- `E_CREDENTIAL_MISSING` — 401 or 403 from HTTP, SSE or the broker, or a `secret:realm/name` the vault does not hold.
- `E_CONFIG_INVALID` — an HTTP client that cannot be built, a header or environment variable with no name, or a consent whose digest no longer matches the launch it was given for.
- `E_GATE_DENIED` — a confidential building, refused where the tool would have been constructed, so the tool never existed.
- `E_OUTSIDE_WRITE_DOMAIN` — an inbound address in the reserved subtree, refused at `admit`, where there is no second entrance.
- `E_AUTH_REQUIRED` — an agent answered `-32000`; the recovery lists the login methods it offered, minus the subscription login this city never starts (D18).
- `E_PROVIDER` — a broker failure, or a harness that refused the request, could not be written to, or closed its output before answering.

A request that went out and lost its answer is `Retry::Unknown` and is not resent automatically, because the server may have done the work; the person who can see the call decides. The state after a refusal is part of the contract: a reader that refuses one message reads nothing more from that source and a stdio child is collected, so the holder of the link opens a new connection on the next dispatch; an HTTP session that answers 404 with a session id is marked ended and its session id forgotten; a refused `admit` leaves no dispatch behind.

## Constraints a reader acts on

- The `conformance` feature is off by default. It adds three child programs — `echoing`, `gated` and `counting_starts` — that answer every line with one fixed result, for the tests of a package that wires MCP servers in; `sprawling-accounting`'s dev-dependencies turn it on, and `just test` builds everything with `--all-features`.
- No `async`: an asynchronous runtime would turn one synchronous tool call into an async library.
- No file handle. `harness::detect` judges evidence the caller read and handed in as text, and `catalog::bundled` compiles its snapshot into the binary, so a path stays a question the assembly layer answers.
- Every HTTP client comes from `gateway::client_for`, which is the one place the proxy rule and the crypto backend are decided; this crate adds a user agent and a whole-request deadline and nothing else.
- A child process starts from `child::command`, and an ACP agent is started on a whitelist rather than on that constructor's blacklist, because after the open list the program is one anybody wrote (D17).
- `mcp::reading` and `mcp::sse` each start one reader thread bounded by the connection it serves; `ARCHITECTURE.md` §10 rule 3 lists both among the places a library crate starts a thread.
- Its SPEC §3 records what is specified and not built: the login executor after `-32000`, two further detection clues, the CLI's `/acp` list, and url elicitation. `AgentLogin` answers `not_built` today, and the ACP page draws no login key.

## Who uses it

`cargo tree --workspace -i sprawling-agent-protocols -e normal --depth 1` prints `sprawling`, the assembly root; `sprawling-accounting`, whose writer keeps a building's server connections and registers their tools; and `sprawling-desktop`, this Windows desktop offered as an MCP server, which names two items here — `PROTOCOL_VERSION` and `EFFECT_META_KEY` — rather than spelling either a second time.

## The specification

[`Spec.lean`](Spec.lean) is the entry and its parts live under [`spec/`](spec/): `spec/Acp.lean`, `spec/Harness/Consent.lean`, `spec/Harness/Session.lean`, `spec/Mcp/Link.lean`, `spec/Mcp/Reading.lean` and `spec/Mcp/Tools.lean`. §1 lists the modules, §3 what is assumed and what is not built, §7 the module boundaries, §8 the interfaces tag by tag, §12 the codes, and §16 the tests that hold what the proofs leave open. A decision is a comment opening with `D<n>` beside what it governs, cited elsewhere as `agent_protocols D<n>`.

Each part proves one property and carries a theorem that names what removing its guard would allow; §16 of the entry lists them. The SPEC is written in Chinese with concept names in English; [`docs/glossary.md`](../../docs/glossary.md) defines the words, among them *Connector*, *official harness*, *agent entry* and *agent catalog*.

## Building and checking

```bash
cargo nextest run -p sprawling-agent-protocols                       # the module tests beside the code
cargo nextest run -p sprawling-agent-protocols --features conformance  # the same, with the scripted child programs
lake build crates.agent_protocols.Spec                               # this package's Lean specification
```

`crates/agent_protocols/catalog/registry.json` is generated by `cargo xtask acp-catalog`, which reads the registry's CDN index and keeps only the fields the city reads; the file is committed, and `harness::catalog`'s tests read it and a recorded copy of the index through the same reader. `just models` builds every crate's SPEC in one run, and `just test` runs the whole workspace's tests.
