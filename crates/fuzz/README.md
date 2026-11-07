# sprawling-fuzz

The cargo-fuzz targets for the parsers that read bytes from outside the process: an address, a locator, the tail of a Ledger segment, an inbound wire frame, the text of a `CONFIG.toml` a person typed, and a line an MCP server wrote back. A parser meeting hostile bytes is where outside content first touches the city, so each target asserts a property of what the parser accepted, beyond the absence of a panic. [`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 11 lists fuzzing as layer V4 of the verification, and the number of targets there is a generated figure rather than one this file repeats.

This directory is a Cargo workspace of its own — the empty `[workspace]` table at the end of its manifest is what says so — and it is deliberately outside the product's: fuzzing needs a nightly toolchain and libFuzzer, and neither may enter the product's build graph. Its manifest writes `publish = false`. Nothing in the repository depends on it.

## The targets

| Target | What it feeds | What it asserts beyond "no panic" |
| --- | --- | --- |
| `fuzz_address` | `kernel::Address::parse` with any UTF-8 text | An accepted address reads back byte for byte, is not empty, and smuggles no banned component through: no leading `/`, no `\`, no `:`, and no empty, `.` or `..` segment. |
| `fuzz_locator` | `kernel::Locator::parse` with any UTF-8 text | An accepted locator prints as the bytes it was read from, `secret:` never parses, and every refusal carries `AxCode::LocatorInvalid`. |
| `fuzz_jsonl_tail` | one ledger segment of arbitrary bytes, read through `storage::JsonlLedger::open` | When the segment opens, a second open is clean — tail recovery is idempotent and truncates nothing further — and reads back the same lines. A refusal is a legal outcome. |
| `fuzz_wire_frame` | the bytes of one inbound `wire::ClientFrame` | A frame that decodes re-encodes and decodes to an equal frame, so the auth check and the dispatch cannot see two different commands; a present `Hello` pairing token is never an empty string. |
| `fuzz_config_toml` | the text of one `CONFIG.toml`, through `city::ConfigLayer::parse` | One file text states one configuration; no sandbox mount reaches into the city's own subtree; two MCP servers never share a label. |
| `fuzz_mcp_stdio` | one line an MCP child wrote to its stdout, through `agent_protocols::Rpc::read` | A line carrying an `error` object is never read as a result, and a `tools/list` result that is admitted names every tool under that server's label prefix. |

The comment at the top of each file under [`fuzz_targets/`](fuzz_targets/) states the property that file checks; this table is a reading of those comments, and they are what a change is made against.

A new target is one file there whose top comment states its property, plus one `[[bin]]` entry shaped like the others. `cargo xtask docnum` recounts the directory, so the number the architecture document quotes follows the file rather than a second list kept in a document.

## When a target finds something

An input the parser rejects is the ordinary outcome: every target returns early on it, and only a broken property panics. cargo-fuzz keeps the accepted inputs in `corpus/` and the failing input in `artifacts/`, both under this directory, and `.gitignore` here keeps those two, `target/` and `Cargo.lock` out of git. What a run leaves behind is an artifact rather than a report: a run that stops with one has found a defect in the crate that owns the parser, and the fix belongs there rather than in a target.

## Constraints

- It is a separate workspace, so it resolves its own lockfile and its own versions. It takes `sprawling-kernel`, `sprawling-storage`, `sprawling-wire` without its listener (`default-features = false`, because a frame is decoded before any socket code sees it), `sprawling-city` and `sprawling-agent-protocols` by path. No product crate names this one.
- [`rust-toolchain.toml`](../../rust-toolchain.toml) pins a stable release, and cargo-fuzz needs the nightly-only sanitizer flags, so every command names `+nightly`; `RUSTUP_TOOLCHAIN=nightly` is the same override for the recipe.
- The manifest is not a member of the product workspace, so every command run from the repository root names `--fuzz-dir crates/fuzz`.
- Each target is a binary of its own with `test`, `doc` and `bench` turned off and `#![no_main]`, which is the shape libFuzzer's entry point needs.
- A target that needs a hook in a parser asks the crate that owns the parser for it; no rule moves in here for a fuzz target's convenience.

## Who uses it

Nothing does, and no gate runs it: `just check` never builds or touches this directory. `.github/workflows/nightly.yml` runs every target for sixty seconds each night, with `--target x86_64-unknown-linux-gnu`; `just fuzz <target>` runs one target until it is stopped or finds a crash.

## Its SPEC

There is none. Each property lives beside the target that checks it, in the comment at the top of its file, because a target has one property and its file is where a change to that property is made. What a parser must hold is specified in the SPEC of the crate that owns it, never here.

## Checking it

```bash
cargo +nightly fuzz list --fuzz-dir crates/fuzz                                        # the targets
cargo +nightly fuzz run <target> --fuzz-dir crates/fuzz                               # one target, until stopped
RUSTUP_TOOLCHAIN=nightly just fuzz <target>                                           # the same through the recipe, which names no toolchain itself
cargo +nightly fuzz run <target> --fuzz-dir crates/fuzz -- -max_total_time=60          # sixty seconds, the length of the nightly run
```
