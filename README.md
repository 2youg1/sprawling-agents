<div align="center">

# sprawling

**Run many agents on your own machine as a city. One Rust binary; the interface is a page in your browser.**

<p align="center">
  <a href="https://crates.io/crates/sprawling"><img alt="crates.io" src="https://img.shields.io/crates/v/sprawling?logo=rust&amp;labelColor=171717&amp;color=DEA584"></a>
  <a href="https://www.npmjs.com/package/sprawling"><img alt="npm" src="https://img.shields.io/npm/v/sprawling?logo=npm&amp;labelColor=171717&amp;color=CB3837"></a>
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/github/license/2youg1/sprawling-agents?labelColor=171717&amp;color=4C8BF5"></a>
  <a href="https://github.com/2youg1/sprawling-agents/actions/workflows/ci.yml"><img alt="ci" src="https://github.com/2youg1/sprawling-agents/actions/workflows/ci.yml/badge.svg?branch=main"></a>
  <a href="https://deepwiki.com/2youg1/sprawling-agents"><img alt="Ask DeepWiki" src="https://deepwiki.com/badge.svg"></a>
</p>

</div>

Give your agents a city to work in. In sprawling, projects become buildings where agents exchange messages, divide the work and keep you informed as it progresses.

Plans, decisions and handoffs live in files. sprawling uses these records to carry long-running work across sessions, organise larger tasks and follow workflows you define. One Rust binary runs locally, serves the browser interface and records the city's history in an append-only Ledger.

**Status: <!-- xtask:begin maturity:word -->alpha<!-- xtask:end -->.** Data formats, the wire and the interface may still change between versions.

中文：[README.zh-CN.md](README.zh-CN.md) · Project introduction: [LLM.md](LLM.md) · Code changes: [AGENTS.md](AGENTS.md)

## Quick start

Choose an install channel. npm/Bun and cargo-binstall download prebuilt binaries; `cargo install` compiles locally. The shell installers need no JavaScript or Rust toolchain.

npm, with Node.js and npm installed:

```sh
npm install --global sprawling@latest
```

Bun, with Bun installed:

```sh
bun install --global sprawling@latest
```

crates.io, with the Rust compiler and native build tools required by the published package; with cargo-binstall installed, use `cargo binstall sprawling` to download a release archive:

```sh
cargo install sprawling --locked
```

macOS/Linux shell:

```sh
curl -fsSL https://raw.githubusercontent.com/2youg1/sprawling-agents/main/install.sh | sh
```

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/2youg1/sprawling-agents/main/install.ps1 | iex
```

Prebuilt archives support Windows x86-64, macOS on Apple silicon and Linux x86-64. Use the [release list](https://github.com/2youg1/sprawling-agents/releases) for manual downloads and the [installation guide](docs/getting-started.md#1-install) for version selection, verification and source builds with Nix.

The AUR package is not published yet. Release builds generate and validate its PKGBUILD and .SRCINFO; publication requires `AUR_SSH_KEY`. Once published, Arch Linux x86-64 users can inspect the package and run `makepkg -si` in its checkout, then update with `git pull --ff-only && makepkg -si`.

After installation, all channels use the same command:

```sh
sprawling up ./cities/first
```

The terminal becomes the city's console and prints the serving address; the browser opens the page. Connect a provider or local model, choose a model for `main`, then tell the Mayor what result you want and what counts as done. The Mayor plans and the buildings' residents execute; you follow progress, answer questions and inspect results. `Ctrl-C` in the console stops the city.

sprawling does not update automatically; check for updates in Settings and follow the [update guide](docs/getting-started.md#updating).

## Five capabilities

**Long-running work and automation.** Plans, decisions and handoffs stay in readable documents, giving agents a record to continue across sessions. Hierarchical plans coordinate larger tasks, while roles, skills and tool integrations let you define the workflow. A standing goal dispatches ready plan nodes and waits for active runs; [daily operation](docs/operating.md) explains how to steer, pause and stop work.

**Social simulation.** Agents can find one another, exchange messages, coordinate work and wait for replies without you relaying each conversation. Your main agent explains the work and reports its progress; the recorded exchanges let you inspect how the group interacts, and [playback](crates/city/skills/playback/SKILL.md) exports a history you can check against the Ledger.

**Easy to start.** Conversations, skills and tool connections follow patterns familiar from other agents. The [getting-started guide](docs/getting-started.md) offers separate routes for agent users moving their configuration and chat users starting their first project, through the first task, reading its report and stopping work.

**Built-in monitoring.** Inspect run timings, model calls, token use, costs and resource readings as work progresses. Use the monitor and `sprawling gauge` to build performance evals around your own workload and compare readings on your hardware. The [performance guide](docs/performance.md) describes counters, reproduction and measurement provenance.

**Customisation and development.** Define how agents work through role documents, project rules and skills; connect the models and MCP tools your tasks need, or bring a supported harness over ACP. Build another interface against the wire and use the documented seams for runtime changes. These parts suit a workflow or AgentOS that needs persistent project teams, document-based handoffs and a shared history on one machine; [integrations](docs/integrations.md) covers the existing connections, and [architecture](ARCHITECTURE.md#8-where-to-change-what) locates runtime changes.

## Documentation

| Document | What it holds |
|---|---|
| [`docs/getting-started.md`](docs/getting-started.md) ([中文](docs/getting-started.zh-CN.md)) | The guide for a newcomer: every concept met on the way, from installing to a reviewed merge |
| [`docs/operating.md`](docs/operating.md) | Daily use: steering and stopping work, answering residents, swapping providers and MCP servers, remote access, recovering from failures |
| [`docs/glossary.md`](docs/glossary.md) | One meaning for each word the code, the page and the documents use |
| [`LLM.md`](LLM.md) | Project capabilities, boundaries and reading paths for a model introducing sprawling |
| [`docs/wire.md`](docs/wire.md) | CLI, frames, answers and exit codes for programmatic control |
| [`docs/integrations.md`](docs/integrations.md) | ACP harnesses, MCP tool servers and CLI control from an existing agent |
| [`docs/performance.md`](docs/performance.md) | Monitor readings, reproducible workloads and measurement provenance |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | The crates and their dependency rules, the seams, one dispatch end to end, what is on disk, how it is verified, and where each kind of change goes |
| [`AGENTS.md`](AGENTS.md) | The rules every change follows and the commands that check them, read first by people and agents alike |
| [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md) | Contribution preparation, feature admission and submission, with links to the repository rules |
| [`docs/frontend-method.md`](docs/frontend-method.md) | How a screen is built and accepted, and the approved visual design |
| [`crates/README.md`](crates/README.md) | Every crate, what it owns and where its specification is; each specification, `crates/<dir>/Spec.lean`, holds that crate's interfaces, decisions and proofs, with comments in Chinese |
| [`crates/city/templates/`](crates/city/templates/) | The documents the city writes into each building, which agents read and so can you |
| [`crates/city/skills/`](crates/city/skills/) | The skills that ship with the release |
| [`docs/logging.md`](docs/logging.md) | What goes into the diagnostic log, what goes into the Ledger, and why the two stay apart |
| [`docs/third-party.md`](docs/third-party.md) | The upstream facts this tree follows, credits and licence obligations |
| [`CHANGELOG.md`](CHANGELOG.md) | What each release changed, and what it left known and unfixed |
| [`SECURITY.md`](SECURITY.md) | How to report a vulnerability |

## Sources and credits

Provider interfaces and harness launch commands follow the ACP registry and vendor documentation; where needed, vendor clients supply more precise facts without copying their code. This repository implements its own interface controls, with keyboard behaviour following W3C ARIA Authoring Practices, Kobalte and Ark UI documentation. [Third-party sources](docs/third-party.md) records source versions and full licence obligations.

The shipped `sdd`, `tutor` and `translation` are English adaptations of the author's Chinese skills. `why`, `how` and `blast-radius` adapt Lauren Tan (poteto)'s [pstack](https://github.com/cursor/plugins/tree/main/pstack), and `authority-review` adapts Thermos from the same repository; all four keep MIT. [skills/LICENSES.md](crates/city/skills/LICENSES.md) carries each skill's terms and credit.

Report bugs or request features through the [issue forms](https://github.com/2youg1/sprawling-agents/issues/new/choose). For code changes, follow [AGENTS.md](AGENTS.md) and [CONTRIBUTING](docs/CONTRIBUTING.md). Use the private reporting channel in [SECURITY.md](SECURITY.md) for vulnerabilities.


The project is [MPL-2.0](LICENSE); shipped skills carry their individual licences with their files.
