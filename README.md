<div align="center">

# sprawling

**Run many agents on your own machine as a city. One Rust binary; the interface is a page in your browser.**

<p align="center">
  <a href="https://crates.io/crates/sprawling"><img alt="crates.io" src="https://img.shields.io/crates/v/sprawling?logo=rust&amp;labelColor=171717&amp;color=DEA584"></a>
  <a href="https://www.npmjs.com/package/sprawling"><img alt="npm" src="https://img.shields.io/npm/v/sprawling?logo=npm&amp;labelColor=171717&amp;color=CB3837"></a>
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/github/license/2youg1/sprawling-agents?labelColor=171717&amp;color=4C8BF5"></a>
  <a href="https://deepwiki.com/2youg1/sprawling-agents"><img alt="Ask DeepWiki" src="https://deepwiki.com/badge.svg"></a>
</p>

</div>

> **Status: <!-- xtask:begin maturity:word -->pre-alpha<!-- xtask:end -->, research and development.** The main loop works: connect a provider or a harness, raise a building, dispatch work, and several agents call tools and write files there at once. Read [What works today, and what does not](#what-works-today-and-what-does-not) before you hand it real work.
>
> 中文：[README.zh-CN.md](README.zh-CN.md) · For an agent driving a city from outside: [LLM.md](LLM.md) · To change the code: [AGENTS.md](AGENTS.md)

## What it is

Claude Code, Codex CLI and similar terminal agents give you one agent at a time: a model, a set of tools, one working folder and one conversation. sprawling runs many of them on one machine as a city. The **city** is a directory, each project in it is a **building**, each agent works in a **room** of a building, and each piece of work is a **run** with a start and an end. You describe a result and what counts as done, and the agents read, run commands and write code and documents, asking you only for decisions your written rules cannot settle. Everything they do goes into one append-only **Ledger**, which you follow on a page the binary serves to your browser.

A resident is either a model the city drives itself, through an API key for any provider that speaks the OpenAI or the Anthropic format or through a local model server, or an official harness such as Claude Code, which the city starts over the Agent Client Protocol (ACP) and which you sign in to with your own subscription. sprawling schedules, hands out tools, keeps the history and shows it to you; the thinking is the model's.

It is built for small teams who want agents to keep running fixed work, and for researchers, in computer science or in the humanities and social sciences, who want to watch how agents divide work and talk to each other. An old laptop or a cheap cloud machine can hold a city.

## Quick start

1. Install:

   ```bash
   curl -fsSL https://raw.githubusercontent.com/2youg1/sprawling/main/install.sh | sh    # macOS, Linux
   irm https://raw.githubusercontent.com/2youg1/sprawling/main/install.ps1 | iex         # Windows PowerShell
   ```

   The other ways in are the archive for your system from the [latest release](../../releases/latest) (Windows x86-64, macOS on Apple silicon, Linux x86-64), `bunx sprawling up` or `npx sprawling up`, `cargo binstall sprawling`, which fetches that same archive, and `cargo install sprawling --locked` with Rust 1.97 or later. The binaries are not code-signed: on Windows choose **More info → Run anyway**, and on macOS open the binary once from Finder's right-click menu.

2. Raise a city and open it:

   ```bash
   sprawling up ~/cities/first
   ```

   The terminal becomes the city's console and prints the address it serves, and the browser opens the page. `Ctrl-C` in the console stops the city.

3. On the page, connect a provider with a base URL, the format it speaks and a key, then choose a model for the `main` role. The key goes into your operating system's credential store, and the page only ever sees a reference of the form `secret:realm/name`.

4. Tell the Mayor what you want, in the box at the bottom of the page. The Mayor plans, raises buildings for the work and hands each building its part; you watch the runs, answer what they ask and review what they changed.

A git checkout builds with `just dist`, because a plain `cargo build` embeds a blank page until `just build-web` has built it. [`docs/getting-started.md`](docs/getting-started.md) ([中文](docs/getting-started.zh-CN.md)) is the full guide, from an empty directory to a reviewed merge on your branch.

## Five words

| Word | What it is |
|---|---|
| **City** | One city on one machine: a directory tree, one Ledger, one complete history. Two cities never reference each other. |
| **Building** | One project inside the city. Configuration, the archive and the write domain are scoped to it, and its rules live in `RULES.toml`. |
| **Room** | A subdirectory of a building. One agent works in one room. |
| **Run** | A piece of work with a beginning and an end. **A resident is an identity; a run is the cost.** |
| **Ledger** | The only history. One line per event, append-only, verifiable offline. |

The rest of the vocabulary is in [`docs/glossary.md`](docs/glossary.md).

## Bring your own harness over ACP

A room's resident can be one of five official harnesses, Claude Code, Codex, Grok Build, Kimi Code and Pi, which the city starts as an ACP agent. This is how a subscription reaches a city: you sign in inside the harness, and the city itself signs in to nothing. Name the harness in a building's `.sprawling/CONFIG.toml`, and the rooms opened in that building from then on take it as their resident:

```toml
[resident]
harness = "claude_code"   # or "codex", "grok_build", "kimi_code", "pi"
```

The **official harnesses** group in settings shows the command that starts each harness (Node's `npx` for four of them, the `kimi` binary for Kimi Code), whether this machine can run it, and where the vendor explains signing in. A harness runs its own tools, so the city decides where it writes but not what it does: the harness works in its room's own git worktree, the city records what it reports, a halt or the building's `harness_minutes` ceiling becomes an ACP cancel, and when the harness answers, the city commits the tree and offers the work for review before it merges. A confidential building refuses harness residents, because a harness sends the room's content to its own vendor.

## How it behaves

- An address answers three questions at once: `lab/room1` is a place on disk, and that place decides what the agent there may write, which documents it starts with and whom it reports to.
- Agents find each other and talk without you relaying. A message reaches a working resident at the end of its next tool result and starts a run for an idle one. Only the User's own entrance can speak in the User's name, and a type enforces it.
- The Ledger is the only history: every effect is written as an event before it happens, every view on the page is rebuilt from it byte for byte, and one changed byte stops chain verification at that line.
- Every deletion carries its way back, and the recycle bin restores a file with one press.
- Each agent works on its own git worktree, and its change merges only after another resident has verified it; verifying your own work is a compile error.
- Your git history keeps its shape. Checkpoints live under `refs/sprawling/runs/`, and a branch moves only for the first commit of an empty repository or the merge of reviewed work. `Sprawling-*` trailers name each commit's run, resident and model, and `sprawling whose <city> <commit>` answers from the Ledger which run wrote it.
- Cost is cut by run, resident, prefix segment, tool and skill, from the provider's bill or else the price sheet. With no price at all, as with a subscription, the page says so instead of printing `$0.00`.
- A browser notification is raised only for a decision that needs you.
- Some mistakes cannot be written: sending a credential over the wire, a deletion with no way back, finishing without evidence, giving part of a plan more weight than its parent and verifying your own work are unrepresentable in the types, as <!-- xtask:begin compile_fail_cases -->18<!-- xtask:end --> compile-fail tests prove.

## What works today, and what does not

| Area | What works |
|---|---|
| Residents | Any provider that speaks the OpenAI or the Anthropic format, a local model server, and the five official harnesses over ACP. |
| Work | Several runs at once, each on its own git worktree and merged after review; a standing goal that drives every ready part of a plan; residents that talk to and wake each other, where a signal or a delegation takes effect when it is sent and a sender can stop and wait for the reply. |
| Tools | Built-in tools, the skills a building admits, any MCP server over stdio, HTTP or SSE, a browser the city drives, and the Windows desktop behind an allowlist. |
| Running | New runs wait while memory is tight, and a dispatch is refused before anything is written when the disk is nearly full; a city can be paused, and `up --supervise` brings it back after a crash. |
| Recovery | The recycle bin, `resume` after a crash, offline chain verification, and export and restore onto another machine. |
| The page | A conversation with any room, sessions with a name and tags of their own, the city, buildings, runs, the record, cost, MCP, a performance monitor, settings in a tree that folds, a colour page, and a document workspace that edits Markdown and text and previews PDF and DOCX, with versions and diffs. |
| Reach | Machines on the same network with a pairing key, and the remote door through a route you choose. The city key is kept in the city's vault, in Credential Manager on Windows or the Keychain on macOS, so a paired device stays paired when the city restarts; on Linux the kernel keyring keeps it until the computer reboots, and the encrypted vault file keeps it across reboots. |

| Not yet | Where it stands |
|---|---|
| An OS sandbox on Windows and macOS | Commands run in a copy of the working tree: your files are safe, but the network is open. Linux with a namespace wrapper also closes the network and contains the process tree. The promise today is that a deletion can be undone, not that it cannot happen. The arm names and each platform's default are decided ([`docs/operating.md`](docs/operating.md), *How `exec` is confined*); the Windows and macOS arms are not built. |
| Harness residents with the city's tools | A harness gets neither the city's collaboration tools nor the building's MCP servers, a steer sent during its turn is recorded but not delivered, and its permission requests get the first allow-once option, inside its own worktree. |
| Skill audit and usage | The Ledger has a line for a skill's audit and a skill's audit state is read from it, but nothing asks skills.sh or SkillSpector yet, so every skill shows as unaudited, and no per-skill usage table is folded. |
| A checked phone layout | Below 768 px the page draws one column, but the render gate checks nothing narrower than 768 px. |
| Browser end-to-end tests in CI | CI renders every settled screen in a real browser engine on fixtures, but no job drives a live city. |
| Byte-identical builds across machines | Two builds on one machine match, checked nightly. Across machines the recorded source paths differ until `trim-paths` reaches the stable toolchain this project pins. |
| Spend attributed to skills | A skill is a line in the prefix, not a calling context, so every call lands in the `no_skill` bucket. |

## From the terminal

The page is not the only door. A few of the verbs, each talking to the same city:

```bash
sprawling up [city-dir] [addr]        # raise the city if it is not there, serve it, open the page
sprawling dispatch <addr> <task>      # send one task to a served city and print its events until the run ends
sprawling view <city>                 # read a city's Ledger lines or its run tree, read-only
sprawling doctor [<city>]             # what this machine has against what a city needs
sprawling resume <city-dir>           # after a crash: verify the chain, close lost tool calls, report who waits for you
sprawling export <city> <bundle-dir>  # pack a whole city; `restore` unpacks it on another machine
sprawling help [<verb>]               # every verb, or one explained
```

Nothing updates itself: `sprawling status --check`, or the button in settings, tells you whether a newer release is published, and replacing the binary stays your command to run. [`LLM.md`](LLM.md) documents the wire for a script or another agent.

## Security and privacy

- The city listens on loopback only by default. Binding any other address needs a pairing key: the city adopts `SPRAWLING_PAIRING_TOKEN` when it is set, and otherwise mints a key and prints the address to open with it.
- A phone or a second computer outside your network reaches the city through the remote door over a route you choose. The door opens, closes and takes a new city key from Settings > Remote; opening it and replacing the key act only after you type the code the city prints on its own console, so a tool that drives the page cannot do either, and a device pairs only at the console. The route is a Cloudflare named tunnel, or a command you write, such as one that wraps `tailscale serve`. Pairing and every frame are encrypted end to end with keys the route never holds, so the one thing you trust the route with is delivering the page unchanged. [`docs/operating.md`](docs/operating.md) shows both kinds of route.
- Keys live in the OS credential store, and configuration holds only `secret:realm/name`. Model output, the log and the content of every git checkpoint pass a secret scan before they are written.
- There is no account, no telemetry and no hosted service. A confidential building calls no remote provider, starts no outside tool server and takes no harness resident, so with a local model it can work on private data.

## Why this exists

I have tried many harnesses. Some feel conceptually dated, others overshoot what is useful. Take recursive self-improvement: until the model itself leaves the stateless regime, a harness can only keep adapting to the newest models and learn a company's existing workflows so it runs them faster. The first looks like an ablation study; the second needs privacy.

More and more small companies are tiny teams that ship online services with many agents, and most of them are a pile of Markdown plus a few talented people. I wanted a harness that keeps up with multi-agent work while staying practical about self-improvement and memory, so practical extensibility, saving the User's attention, cost control as agents scale up, privacy and reliability, and long-running operation sit at the core of the design, mixed with a few ideas from urban studies and sociology.

The stronger agents become, the more the User's attention costs, and sprawling tries not to take it. Use it by designing loops rather than writing prompts: lay out the workflow, let agents do the fixed work, and come back now and then to see how it runs. Your files, code and documents are the memory; "memory" that a harness injects into a stateless model mostly slows the model down.

No multi-agent scheme yet delivers gains that justify the cost of scale, and the study of how models interact and behave socially inside agent clusters has only begun. Keep a city lean, with the skills, MCP servers and harnesses your work needs, and add things when a concrete problem asks for them, because the same model behaves differently under different harnesses.

**Strengths**: a small footprint, concepts that are fun to work with, and a design built for many agents from the start rather than one agent with a pile of extensions. **Weaknesses**: a student project with no lab or sponsor behind it, a page that still lags what it wants to be, and stability and usability that both need work.

Two sister projects are on hold. [RefRain](https://github.com/2youg1/RefRain), a writing workbench where an agent proposes and only you merge, is delayed, and its document features are moving into this page. [kusanagi](https://github.com/2youg1/kusanagi), agent messaging that needs no trusted server, is paused.

## Documentation

| Document | What it holds |
|---|---|
| [`docs/getting-started.md`](docs/getting-started.md) ([中文](docs/getting-started.zh-CN.md)) | The guide for a newcomer: every concept met on the way, from installing to a reviewed merge |
| [`docs/operating.md`](docs/operating.md) | Daily use: steering and stopping work, answering residents, swapping providers and MCP servers, remote access, recovering from failures |
| [`docs/glossary.md`](docs/glossary.md) | One meaning for each word the code, the page and the documents use |
| [`LLM.md`](LLM.md) | The wire and the command line, written for an agent or a script that drives a city from outside |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | The crates and their dependency rules, the seams, one dispatch end to end, what is on disk, how it is verified, and where each kind of change goes |
| [`AGENTS.md`](AGENTS.md) | The rules every change follows and the commands that check them, read first by people and agents alike |
| [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md) | The longer form of `AGENTS.md`: each rule with the gate that holds it |
| [`docs/frontend-method.md`](docs/frontend-method.md) | How a screen is built and accepted, and the approved visual design |
| [`crates/README.md`](crates/README.md) | Every crate, what it owns and where its specification is; each specification, `crates/<dir>/Spec.lean`, holds that crate's interfaces, decisions and proofs, with comments in Chinese |
| [`crates/city/templates/`](crates/city/templates/) | The documents the city writes into each building, which agents read and so can you |
| [`crates/city/skills/`](crates/city/skills/) | The skills that ship with the release |
| [`docs/logging.md`](docs/logging.md) | What goes into the diagnostic log, what goes into the Ledger, and why the two stay apart |
| [`docs/third-party.md`](docs/third-party.md) | The upstream facts this tree follows, credits and licence obligations |
| [`CHANGELOG.md`](CHANGELOG.md) | What each release changed, and what it left known and unfixed |
| [`SECURITY.md`](SECURITY.md) | How to report a vulnerability |

## Contributing

Start with [`AGENTS.md`](AGENTS.md):

```bash
cargo install just --locked
just prereqs        # every other tool the loop needs, with the install line for each one that is absent
just check          # a change is finished when this is green
```

Pull request descriptions, issues and review comments may be written in your own language. A parallel translation, English if your language is not English and Chinese if it is, lets people and agents read faster and makes a mistranslation visible. [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md) has the rest.

## Credits

Facts about vendors, such as the path an API hangs under, the format it answers in and how each official harness starts, are followed from the ACP registry and, where a vendor's own client is more precise than its documentation, from that client, without copying its code; [`docs/third-party.md`](docs/third-party.md) §1 names each repository, path and commit, and a daily workflow asks each upstream whether it moved. Every control on the page is this repository's own, and its keyboard behaviour follows the W3C's ARIA Authoring Practices and the Kobalte and Ark UI documentation, read as prose.

Of the skills under [`crates/city/skills/`](crates/city/skills/), `sdd`, `tutor` and `translation` are English adaptations of Chinese skills I published under AGPL-3.0-or-later (the translation skill's original byline also credits Claude Fable 5), here under MPL-2.0. `why`, `how` and `blast-radius` are my modified adaptations of [pstack](https://github.com/cursor/plugins/tree/main/pstack) by Lauren Tan (poteto), and `authority-review` adapts the Thermos plugin from the same `cursor/plugins` tree; all four keep MIT. [`docs/third-party.md`](docs/third-party.md) §5 gives the terms.

## Build on sprawling

sprawling is meant to be built on. Most of it sits behind a seam or in a single file, so you can replace one part without touching the rest:

| Layer | What you can replace |
|---|---|
| The page | The whole client, because the wire in `crates/wire` is the entire API and a client in any language can stand in for the shipped one; or keep it and change its colours and motion (`client/src/theme.css`) or every word it shows (`client/src/lang.json`). |
| Residents | The official harnesses and how each one starts (`agent_protocols::harness`), the documents a building is raised with (`crates/city/templates/`), and the skills a building admits. |
| Tools | The built-in tools behind `kernel::tool`, any MCP server, the browser driver behind `browser::port`, and the Windows desktop server (`crates/desktop`). |
| Models | The known provider hosts (`gateway::provider::preset`), the request formats (`gateway::dialect`), and the endpoint itself, a local model included. |
| Execution | The WebAssembly sandbox behind `runtime::sandbox`, and the confinement host commands run under (`runtime::tools::exec::confinement`). |
| History and reach | The Ledger store behind `kernel::ledger`, and the remote route behind `remote_access::route`, such as a tunnel of your own. |

How to replace each part is in [`ARCHITECTURE.md`](ARCHITECTURE.md): §4 lists the seams and the second implementation each one already has, and §8 names the first file to open for each kind of change and the check that turns red when the change is wrong. Hand an agent that file, the crate's specification and [`AGENTS.md`](AGENTS.md) together with what you need, and it can usually carry the change through.

## License

MPL-2.0, see [`LICENSE`](LICENSE). Each skill under [`crates/city/skills/`](crates/city/skills/) states its own licence, and [`crates/city/skills/LICENSES.md`](crates/city/skills/LICENSES.md), which ships in the release archive, gives each one's terms and credit.

---

Questions, bug reports and disagreements are all welcome: open an issue, or write to the address on my profile.
