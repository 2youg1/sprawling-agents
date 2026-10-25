# sprawling-fuzz

The cargo-fuzz targets for the parsers that read bytes from outside the process: an address, a locator, the tail of a Ledger segment, an inbound wire frame, the text of a `CONFIG.toml` a person typed, and a line an MCP server wrote back. A parser that meets hostile bytes is where outside content first touches the city, so each target asserts more than the absence of a panic, for example that an accepted address reads back byte for byte, or that a decoded frame, encoded and decoded again, is the same frame; the comment at the top of each file under `fuzz_targets/` states the property it checks. `ARCHITECTURE.md` section 11 lists fuzzing as layer V4 of the verification.

This directory is a Cargo workspace of its own, deliberately outside the product's: fuzzing needs a nightly toolchain and libFuzzer, and neither may enter the product's build graph. It takes `sprawling-kernel`, `sprawling-storage`, `sprawling-wire` (without its listener), `sprawling-city` and `sprawling-agent-protocols` by path, and nothing depends on it. It has no SPEC; the property each target checks is written beside the target.

## Running it

[`rust-toolchain.toml`](../../rust-toolchain.toml) pins a stable release, and cargo-fuzz needs nightly-only sanitizer flags, so name the nightly toolchain explicitly:

```bash
cargo fuzz list --fuzz-dir tools/fuzz                                        # the targets
cargo +nightly fuzz run <target> --fuzz-dir tools/fuzz                       # one target, until it is stopped or finds a crash
RUSTUP_TOOLCHAIN=nightly just fuzz <target>                                  # the same through the recipe, which names no toolchain itself
cargo +nightly fuzz run <target> --fuzz-dir tools/fuzz -- -max_total_time=60 # sixty seconds, the length of the nightly smoke run
```

`.github/workflows/nightly.yml` runs every target for sixty seconds each night on Linux, with `--target x86_64-unknown-linux-gnu`. What a run leaves behind, the corpus, the crash artifacts, the build output and the lockfile, stays out of git through `.gitignore` in this directory.
