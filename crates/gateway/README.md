# sprawling-gateway

Everything between a decision to call a model and the bytes on the wire. The city keeps one canonical request shape, and `dialect` translates it to and from each provider's format, OpenAI-shaped or Anthropic-shaped, as pure functions. `endpoint` is the HTTP client this crate writes itself and the production adapter of the `kernel::model` seam. `credential` takes custody of the API keys a person hands over, keeps them in the vault, and redeems a `secret:` reference only when a request is assembled; a subscription does not pass through this crate, and reaches the city through an ACP agent instead. The other modules record cost against market snapshots (`market`, `cost`), give each endpoint its permits and queue and hold calls that share a cold prefix behind one pre-warm (`concurrency`, `hold`), keep the endpoint book the Ledger rebuilds (`router`), and write down once what each vendor's documentation says about hosts, output ceilings and thinking levels (`provider`). Rust code names the crate `gateway`; `cargo -p` takes `sprawling-gateway`.

It depends on `kernel` and `child`. `agent_protocols`, `accounting` and `sprawling` depend on it in the product, and `citysim` outside it; `cargo tree --workspace -i sprawling-gateway --depth 1 -e normal` prints the current list, and the `depmap` block in [`ARCHITECTURE.md`](../../ARCHITECTURE.md) section 3 holds what each crate may name.

## Specification

The SPEC is [`Spec.lean`](Spec.lean), with its parts under [`spec/`](spec/); `ARCHITECTURE.md` section 11, *Specifications in Lean*, describes that layout. Section 1 of the SPEC lists the modules, section 8 says which part holds each interface section, a decision is a comment that opens with `D<n>` beside what it governs, and section 16 names the tests that hold what the proofs leave open. The SPEC is written in Chinese with concept names in English, as the *Language* section of [`AGENTS.md`](../../AGENTS.md) requires; [`docs/glossary.md`](../../docs/glossary.md) defines the words, among them *dialect*, *Endpoint*, *provider account* and *prefix hold*.

## Checking it

```bash
cargo nextest run -p sprawling-gateway --all-features   # the tests, as `just test` runs them
lake build crates.gateway.Spec                          # the SPEC's proofs; `just models` builds every crate's
just bench                                              # the timing instruments, this crate's among them; measured, never gated
```

The model lists under `tests/listings/` are the answers providers publish, from a public list or the example in an API reference, and the endpoint tests read them the way a probe reads a live answer. The check that calls a real endpoint is `just e2e`, a test of the `sprawling` package: it needs a key a person exports, and without one it prints a line and passes.
