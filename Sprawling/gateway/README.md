# sprawling-gateway

Everything between a decision to call a model and the bytes on the wire. A caller gets the endpoint client and the `kernel::model` adapter behind it, the custody of the API keys a person hands over, the per-call settlement that says what a call cost, and the pure judgements that fix a request's shape: which dialect translates it, what its context window and output ceiling are, which thinking level it carries. `dialect` turns the city's canonical conversation into a provider's format and back, and the request that leaves is explainable byte for byte.

A subscription does not pass through this crate. A person signs in inside one of the five official harnesses, and that ACP agent carries the subscription into the city (`crates/gateway/Spec.lean` §8-5); this crate logs in as no vendor's client, holds no subscription token and renews none. It holds no Ledger handle, so every event payload it produces is a value the caller records. Whether a call may leave this machine is `kernel::gate`'s judgement, and what a provider's answer means for the Ledger is `runtime::redact`'s. Rust code names the crate `gateway`; `cargo -p` takes `sprawling-gateway`.

## How it is shaped

| module | the one thing it decides | shape |
|---|---|---|
| `dialect` | which dialect answers a question, over a closed set of three: chat, responses, messages | decision |
| `anthropic`, `openai` | one wire's request and its answer in both directions, with every loss the translation takes stated where it is taken | decision |
| `mismatch` | the four readers both wires need, and the one word for a shape this build did not ask for | decision |
| `endpoint` | one call as one HTTP round trip, with no retry hidden inside it | adapter |
| `credential` | custody: capture, replace with a `secret:` reference, redeem once per operation, describe without the value | adapter |
| `market`, `cost` | the pinned catalogue, and a call's settlement in which the provider's own figure wins | value, decision |
| `concurrency`, `hold` | one endpoint's permits, and the prefix hold in front of the ticket queue | decision |
| `router` | the book of attached endpoints and the model chosen per tag, rebuilt from the Ledger | projection |
| `provider` | what the vendors' documentation says about a host, a model, a level and a connecting face, written down once | value |
| `reach` | where a call stops, stage by stage, and which calls take this machine's proxy | adapter |
| `transcribe`, `ocr`, `adviser` | three optional facilities, absent by default and refused by name when absent | adapter |

`provider` carries three ladders as modules of their own: `provider::ceiling` settles which statement about a window or an output ceiling wins and who stated it, `provider::input` settles what a model accepts, and `provider::thinking` settles which levels one (Endpoint, model) offers and refuses an explicit level outside the offer. `endpoint::permit::Gated` is the only `kernel::model` a caller outside this crate can obtain: it takes the endpoint's permit before the request leaves and gives it back on every return path, including a failed one.

Two seams meet here. `kernel::model` is a seam whose production adapter is this crate's (`endpoint`), with `citysim`'s `ScriptModel` as the second implementation. `Vault` is an inner seam that stays `pub(crate)`: the platform credential service, session memory and an encrypted file sit behind two sentences of interface. The crate declares no public trait — `dialect` is a pure function and does not become one (`ARCHITECTURE.md` §4).

## When it fails

A failure reaches the caller as one `kernel::AxError`: the action that failed, the subject, a stable code and a recovery sentence. §12 of the SPEC holds each code beside the decision that says why it cannot be designed away.

- `E_PROVIDER` — a network or provider failure. The subject names the endpoint and the status and never the request body. Whether the same request may go out again is decided once, in `endpoint::failure`: a provider that says busy or broken (408, 429, 5xx) is asked again, a request it refused as wrong (400, 401, 403, 404, 422) is not, and a failure whose effect is unknown is sent again only on the same account.
- `E_WIRE_MISMATCH` — the provider answered in a shape this dialect does not know. The subject names the JSON path, so a reader knows which field to look at, and the recovery asks for the endpoint's dialect and base URL to be checked against the provider's documentation. Its carrier is loadtime, so no event line carries it.
- `E_CONFIG_INVALID` — a value that cannot be used: a base URL with a scheme the city cannot call or with no host, a `max_in_flight` outside the range 1 to 256, an HTTP client that cannot be built, a credential file its passphrase does not open.
- `E_CREDENTIAL_MISSING` — a `secret:` reference the store does not hold. The subject is the reference, and the recovery states what that store's grade costs: on the Linux kernel keyring it says the value lasts one boot, because a reboot is the commonest way to reach this refusal.
- `E_BACKPRESSURE_SHED` — a call that queued past its bound at an endpoint already at its permit ceiling. The subject names the endpoint and the wait, and the recovery says to raise that endpoint's `max_in_flight` or to dispatch fewer runs at once (D20).
- `E_GATE_DENIED` — a confidential building asking a request to leave this machine. The subject is the endpoint's base URL, and the recovery names a local model or dropping `confidential = true` from the building's `RULES.toml`.
- `E_MODEL_UNCHOSEN` — no model is chosen for the tag a call asks for. The recovery says to attach a provider on the settings page and pick a model for that tag.
- `E_INVALID_ARGS`, `E_TOOL_UNAVAILABLE`, `E_STORAGE_FATAL` — the optional facilities' refusals: a recording this build cannot send, and a reader a `Recording` was handed that failed.

Four requests are refused rather than attempted. A base URL carrying a key before the host or in its query is refused when it is entered, because a secret in a URL reaches logs and error text (`router::normalise_entered`). A conversation that outgrew the model's window is refused and never retried, and its recovery names `/new --carry` and a model with a larger window. An explicit thinking level the (Endpoint, model) does not offer is refused before dispatch (`crates/gateway/spec/Provider/Thinking.lean`). A key the environment supplies is refused on delete, because removing the stored copy while the variable still answers would read as done and change nothing (D33).

§12 also lists `E_ENDPOINT_DIALECT_UNSUPPORTED` for a compatible format this build cannot speak (D5); no call site in the crate produces it today.

## Constraints a reader acts on

- The crate has no features. `cargo nextest run -p sprawling-gateway` is its whole test surface.
- It is blocking end to end: `reqwest::blocking`, no `async` and no tokio, because the turn loop is a synchronous function and one HTTP call must not asyncify the library (§3).
- Time is a parameter. `adapter_for(chosen, redemption, dialect_headers, monotonic)` takes `fn() -> Instant`, and this crate never calls `Instant::now`; `clippy.toml` refuses it. `bin::assembly` hands in the production clock, a test hands in one that counts (D20).
- Every HTTP client is built by `gateway::client_for`, which installs the process's one crypto backend first; `clippy.toml` refuses `reqwest`'s own constructors, so a second build point is a red build (`crates/gateway/spec/Reach.lean`, D10).
- No credential is cached. `Custodian::resolve` redeems once per operation, and plaintext exists only in the vault and in the last register before the wire (§7). `Custodian::describe` answers a reference's standing — configured or not, from which store, writable or not, how long that store keeps it — without the value, and `Persistence` is a fact about the target rather than about the probe: `AcrossReboots` where the platform service writes to disk, `ThisBoot` where the Linux build talks to the kernel keyring, `ThisProcess` in session memory. `Custodian::probe` tries the platform service and falls back to session memory with a `provider_degraded` payload it may not drop (D22); the encrypted-file backend is built and no probe selects it yet (§3).
- `EndpointBook` is a value and not a store: deleting it and replaying the Ledger produces the same book (§7). The crate still holds no Ledger handle, and the assembly layer writes what it produces.
- D21: `max_in_flight` is a field of `EndpointTuning`, settled once per endpoint, never an argument of a single call. D8: the crate root re-exports only items another crate names, and a test-only item stays behind `#[cfg(test)]`.
- From this workspace it may name `kernel` and `child` and nothing else. `child` is named for one constant, the environment prefix the vault reads.
- No `unsafe` block and no Zig leaf sit in this crate. `rustls` is named for one call: `reach::tls` installs aws-lc-rs as the process's crypto backend before any client is built.
- `endpoint::stream` gives each streamed call one detached reader thread, bounded by that call; `ARCHITECTURE.md` §10 rule 3 lists it among the places a library crate starts a thread.

## Who uses it

`cargo tree --workspace -i sprawling-gateway -e normal --depth 1` prints `sprawling-accounting`, `sprawling-agent-protocols`, `sprawling` and, outside the product, `citysim`. What each may name is the `depmap` block in [`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) §3.

## Specification

The SPEC is [`Spec.lean`](Spec.lean), with its parts under [`spec/`](spec/). §1 lists the modules, §2 the acceptance criteria, §3 what is assumed and what is specified but not built, §8 the index from each interface tag to the part that holds it, §12 the codes, §16 the tests that hold what the proofs leave open, and §17 what a change here reopens. A decision is a comment opening with `D<n>` beside what it governs, cited elsewhere as `gateway D<n>`. `ARCHITECTURE.md` §11, *Specifications in Lean*, describes the layout.

The SPEC is written in Chinese with concept names in English; [`docs/glossary.md`](../../docs/glossary.md) defines the words, among them *dialect*, *Endpoint*, *provider account*, *prefix hold* and *provider queue*.

## Checking it

```bash
cargo nextest run -p sprawling-gateway   # every test beside the code
lake build crates.gateway.Spec           # the SPEC's proofs; `just models` builds every crate's
just bench                               # the timing instruments, this crate's among them; measured, never gated
```

The model lists under `tests/listings/` are the answers providers publish, from a public list or the example in an API reference, and the endpoint tests read them the way a probe reads a live answer. `just bench` runs the `#[ignore]`d instruments, among them the one that measures the wait in front of an endpoint's permits (`instrument_provider_queue`).

`just e2e` is the one check that calls a real provider: a test of the `sprawling` package that enters by `RunWorker::handle` and calls every host `gateway::known_hosts` names with the key a person exported for that host. Where the key is absent it prints one line and passes, because a gate that fails on every machine without a key is one nobody runs; a job that holds the credentials sets `SPRAWLING_E2E_REQUIRED=1`, and there an absent variable turns it red.
