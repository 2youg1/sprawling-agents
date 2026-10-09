<div align="center">

# sprawling

**Let agents live like citizens in the “city” of sprawling, and save your attention and time.**

<p align="center"><a href="README.md">English</a> · <a href="README.zh-CN.md">简体中文</a></p>

<p align="center">
  <a href="https://github.com/2youg1/sprawling-agents/actions/workflows/ci.yml"><img alt="ci" src="https://github.com/2youg1/sprawling-agents/actions/workflows/ci.yml/badge.svg?branch=main"></a>
  <a href="https://crates.io/crates/sprawling"><img alt="crates.io" src="https://img.shields.io/crates/v/sprawling?logo=rust&amp;labelColor=171717&amp;color=DEA584"></a>
  <a href="https://www.npmjs.com/package/sprawling"><img alt="npm" src="https://img.shields.io/npm/v/sprawling?logo=npm&amp;labelColor=171717&amp;color=CB3837"></a>
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/github/license/2youg1/sprawling-agents?labelColor=171717&amp;color=4C8BF5"></a>
  <a href="https://deepwiki.com/2youg1/sprawling-agents"><img alt="Ask DeepWiki" src="https://img.shields.io/badge/Ask-DeepWiki-2B6CB0?labelColor=171717"></a>
</p>

</div>

In sprawling, a city is one directory on your machine, projects are the buildings in it, and agents live in the buildings as residents: they message one another, split up the work and tell you how far they have got. Behind it all is one Rust binary running locally; it serves the interface to your browser and records the city's history in an append-only Ledger.

The city is more than a metaphor. Buildings, floors and rooms are the directory tree itself, so an address such as `lab/room1` settles three things at once: which files an agent may write, which documents it starts with, and whom it reports to. The three are one fact to begin with, and no extra rule is needed to keep them agreeing ([glossary](docs/glossary.md)). A resident is an identity that lasts across runs, and what costs money is the run it is sent out on, so however many residents a city has, the idle ones cost nothing.

sprawling interrupts you as little as it can. Agents have full permission in their own building by default: in a building raised from the `minimal` template an agent may write every file, and finished work lands without review (to turn on review or limit writes, see [A building of your own](docs/getting-started.md#a-building-of-your-own)). What reaches you for a decision is a design question a resident cannot settle from the building's rules. Plans, decisions and handoffs are written down as files you can read directly, an agent in a new session picks the work up from these records, and the memory lives in your files, code and document libraries. There are no built-in workflows for now; you define the workflow through roles, skills and tool connections.

<p align="center">
  <img alt="The Main conversation page before a session: the text box, the workspace on the left, the model, effort and permission controls on the right" src="docs/images/main-before-a-session.png" width="49%">
  <img alt="A finished session with its sheet: time to first token, output rate, cache hits and tokens, then each tool call with how long it took, and the file it read open on the right" src="docs/images/a-session-at-work.png" width="49%">
  <img alt="The report the run wrote, with its table and numbered list, beside the CSV file it read" src="docs/images/a-report-beside-its-file.png" width="98%">
</p>

**Status: <!-- xtask:begin maturity:word -->alpha<!-- xtask:end -->.** Data formats, the wire and the interface may all still change between versions.

For a model introducing the project: [LLM.md](LLM.md) · Read before changing code: [AGENTS.md](AGENTS.md)

**Strengths**: small; concepts that are genuinely cool; built for many agents rather than for one agent with extensions bolted on.

**Countless weaknesses**: a student project, funded by no API reseller and maintained by no lab; no anime mascot; I want the WebUI to be good and I am not quite good enough at it yet; stability and usability both still need debugging.

## Quick start

Pick one way to install. npm/Bun and cargo-binstall download a prebuilt binary, `cargo install` compiles locally, and the shell installers need no JavaScript or Rust toolchain.

npm (needs Node.js and npm):

```sh
npm install --global sprawling@latest
```

Bun (needs Bun):

```sh
bun install --global sprawling@latest
```

crates.io (needs the Rust compiler the published package requires and the native build tools for your platform; with cargo-binstall installed, `cargo binstall sprawling` downloads a release archive directly):

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

Prebuilt archives support Windows x86-64, macOS on Apple silicon and Linux x86-64. To download one by hand, go to the [release list](https://github.com/2youg1/sprawling-agents/releases); for choosing a version, verifying it and building from source with Nix, see the [installation guide](docs/getting-started.md#1-install).

Whichever way you installed it, one command starts it:

```sh
sprawling up ./cities/first
```

The page opens in your browser, and the terminal shows only two lines, the serving address and a pairing code; Enter opens the page again. Esc turns the terminal into the city's CLI, where you can talk to the Mayor and run every slash command, `/help` lists them, and `/web` opens the page again. On the page, connect a provider or a local model, choose a model for `main`, then tell the Mayor what result you want and how far counts as done. Where the model offers thinking levels, the city asks for `high` until you choose another. The Mayor plans and the residents of the buildings carry the work out; on your side, you follow the progress, answer their questions and check the results.

Closing the browser stops nothing: the city keeps working, and the address opens the page again. To close the city, type `/quit` in the CLI or on the page; with runs still going, it asks whether to wait for them or stop them now. Ctrl+C and Ctrl+V keep their terminal meanings, copy and paste, and never close the city.

sprawling does not update itself; when you want to, check the version in Settings and follow the [update guide](docs/getting-started.md#updating).

Conversations, skills and tool connections work much as they do in other agents. The [getting-started guide](docs/getting-started.md) has a route for each of two kinds of reader: people already using another agent move their configuration over, and people who have only used chat start from their first piece of work; the routes go as far as the first task, reading its report and stopping work.

## What it does

### Long-running work and automation

Larger tasks are coordinated through hierarchical plans: the Mayor writes an idea down as a roadmap, then hands each building its part through `plan`. Once you set a standing goal, it dispatches the plan nodes that are ready and waits for the runs in progress; [daily operation](docs/operating.md) covers how to steer, pause and stop this work.

### Social simulation

Agents can find one another, send messages, coordinate tasks and wait for replies, without you relaying each one. The main agent explains the work to you and reports progress; this communication is recorded, so you can see how the group of agents actually interacts, and [playback](crates/city/skills/playback/SKILL.md) exports the history for you to check against the Ledger.

### Performance

The page and the whole city are served and run by one process, with no database and no separate service; the city's history is an append-only Ledger on disk. In Settings → Performance you can choose CPU placement and core priority, and give each run a memory ceiling; there is none by default, and only you can set one ([choosing how the city uses the machine](docs/performance.md#choose-how-the-city-uses-the-machine)). Commands start at a lower priority than the city itself, so running a build does not slow the page down. Before command output reaches the model, it is trimmed according to the command that produced it, and the full original text can still be fetched afterwards ([sieve](crates/runtime/spec/Sieve.lean)). The monitor and `sprawling gauge` show run timings, model calls, tokens, cost and resource use on your own hardware, and the project's budgets and recorded readings are kept in the [performance register](tools/xtask/budgets.toml) ([performance guide](docs/performance.md)).

### Privacy

A key you paste into a message goes into the vault first, and the model sees only a reference to it ([custody](crates/accounting/src/worker/dispatching/custody.rs)). Values in model replies and tool results that look like secrets are replaced with a marker before they are written into the permanent history ([redact](crates/runtime/src/redact.rs)). Apart from the model calls and tools you connect, plus the web search that is on by default (until you turn it off, it sends the search words to Exa), nothing leaves your machine; a confidential building calls no remote provider. The port on your own machine asks every caller for a credential, so a web page in another tab cannot drive the city: the browser `/web` opens is paired without typing, a second browser asks for the pairing code the terminal shows, and a script on this machine reads a key file that only your account can read ([SECURITY.md](SECURITY.md)). The programs the city starts, such as ACP agents, MCP servers and browsers, do not inherit the city's credentials from its environment. On Windows, Settings has 88 optional privacy controls, ranging from diagnostic data and speech input to app permissions and Windows AI features. Each one shows its current value, what it changes and what it costs; you apply or restore them one at a time, and each write is read back to check it ([Windows privacy controls](docs/operating.md#windows-privacy-controls)). Privacy is not security, and it is not always at odds with convenience, but many of these settings do cost some convenience; the page lays out each one so that you can weigh it yourself.

### Code signing policy

Free code signing provided by [SignPath.io](https://signpath.io), certificate by [SignPath Foundation](https://signpath.org). Once SignPath Foundation has issued the project's certificate, the Windows binary inside a release archive carries that signature, and each release's notes say whether it is signed; the archive's SHA-256 is published beside it, and `install.ps1` refuses an archive whose digest does not match.

- **Authors** — [@2youg1](https://github.com/2youg1) may change the source directly.
- **Reviewers** — a change proposed by anyone outside the project is reviewed in its pull request before it merges.
- **Approvers** — [@2youg1](https://github.com/2youg1) approves every signing request before a binary is signed.

Privacy policy: [Privacy](#privacy) in this file, and [SECURITY.md](SECURITY.md). The services a city reaches keep their own: the model providers you connect, and Exa, which the default web search sends search words to ([Exa privacy policy](https://exa.ai/privacy-policy)).

### A history you can replay

Every decision the city makes is a line in the Ledger, and replaying those lines on any machine gives a byte-for-byte identical result, because decision paths take time as a parameter, use no random source and keep a fixed order ([determinism](ARCHITECTURE.md#10-determinism-and-hardening)). A replay does not run tools or call models again; it reads back the results recorded at the time. The city supplies only the facts and the limits, and leaves how to do the work to the model ([LLM First](ARCHITECTURE.md#llm-first-mechanism-from-the-city-method-from-the-model)).

### Customisation and development

How agents work is defined by role documents, project rules and skills; connect whatever models and MCP tools you need (one provider can hold several accounts, used in the priority order you set), or bring in any agent that speaks ACP: the ACP page lists the agents found on this machine and the ACP registry's catalog, and takes a pasted command or configuration block. You can build your own interface on the wire, or change the runtime along the seams in the architecture. If your workflow or AgentOS needs standing project teams, handoffs through documents and a shared history on one machine, these parts are there to build it with; [integrations](docs/integrations.md) covers the existing connections, and [architecture](ARCHITECTURE.md#8-where-to-change-what) shows where to change the runtime.

## Why I built this

I don’t want to sit in front of a computer 24/7 until the 5-hour quota wall hits and I finally go to sleep. Neither do you.

I’ve tried a lot of harnesses. Some feel conceptually outdated; others overshoot what’s actually useful. Take RSI: until the LLM itself leaves the stateless regime, a harness can only keep adapting to the newest models and learning a company’s existing workflows so it can run them faster. The first trend looks like an ablation study; the second needs privacy.

More and more small companies are appearing—tiny teams shipping online services with a large number of agents. Ninety-nine percent of them are a pile of Markdown plus a few talented people.

So I wanted a harness that keeps up with the emerging multi-agent (graph engineering) wave while remaining pragmatic about RSI and memory-related fashion. I put practical extensibility, saving the user’s attention, experimental cost control for agent scale-up, privacy & reliability, and long-running capability at the core of the design, and mixed in a few ideas from urban studies and sociology. That’s how sprawling took shape.

The stronger and larger agents become, the more expensive human attention gets. I refuse to let sprawling become just another app that tries to hijack yours. Unlike most harnesses that obsess over prompt writing, the best way to use sprawling is to shift toward the loop: design the workflow, let agents develop sprawling itself, and hand over your fixed work… so you can focus on designing new business, learning new skills, and only occasionally checking how things are running.

Honestly, no multi-agent scheme yet delivers performance gains that justify the cost of scale. But exploration of this technology for business automation, social simulation, and AI alignment is only just beginning. We still need a lot of effort and resources to study how models interact, collaborate, and exhibit social behavior inside agent clusters.

Agent memory is indeed an important path toward RSI, but not via harness-level injection. Your files, code, and document libraries *are* the memory. Attempts to make an agent truly grow with you are, before LLMs leave the stateless regime, mostly a drag on the model.

If you want to keep a harness you already like, any harness that speaks ACP can join the city as a resident ([integrations](docs/integrations.md)). kasanagi, which I am preparing now, helps build chat software; later it will work as an MCP server for remote collaboration between several sprawling instances. sprawling is aimed at persistent operations for small teams and at research platforms (computer science or the humanities/social sciences). It is still in R&D. Contributions and conversations are both welcome.

Apart from migrating the necessary business skills / MCP / ACP pieces, I recommend staying lean for now and only adding things manually when you hit a concrete problem. Even the same model behaves completely differently under different harnesses.

My own machine is modest, so I refuse to let multi-agent workloads explode in performance cost. That also makes it suitable for old laptops or cheap cloud boxes.

I don’t sell APIs and I can’t afford a hard drive full of your data, so everything stays local. There is a dedicated confidential building; paired with a local model it is fully usable for private data. The trade-off is that I cannot run enormous-scale tests myself.

## Documentation

Apart from the two READMEs, the two getting-started guides and the comments in the crate specifications, the documents are in English.

| Document | What it holds |
|---|---|
| [`docs/getting-started.md`](docs/getting-started.md) ([中文](docs/getting-started.zh-CN.md)) | A guide for newcomers: from installing to a reviewed merge, with every concept met on the way |
| [`docs/operating.md`](docs/operating.md) | Daily use: steering and stopping work, answering residents, swapping providers and MCP servers, remote access, and what to do when something goes wrong |
| [`docs/glossary.md`](docs/glossary.md) | Every word the code, the page and the documents use, each with exactly one meaning |
| [`LLM.md`](LLM.md) | For a model that introduces sprawling: what it can do, where its limits are, and in what order to read |
| [`docs/wire.md`](docs/wire.md) | The CLI, frames, answers and exit codes for controlling the city from a program |
| [`docs/integrations.md`](docs/integrations.md) | Any ACP agent as a resident, MCP tool servers, and controlling the city through the CLI from an existing agent |
| [`docs/performance.md`](docs/performance.md) | Monitor readings, reproducible workloads and where the measurements come from |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | The crates and the dependency rules between them, the seams, one dispatch from start to finish, what is on disk, how it is verified, and where each kind of change goes |
| [`AGENTS.md`](AGENTS.md) | The rules every change keeps and the commands that check them; people and agents both read it first |
| [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md) | Preparing a contribution, feature admission and the submission process, with the way into the repository rules |
| [`docs/frontend-method.md`](docs/frontend-method.md) | How a screen is built and accepted, and the approved visual design |
| [`crates/README.md`](crates/README.md) | What each crate owns and where its specification is; the specification, `crates/<dir>/Spec.lean`, holds the crate's interfaces, decisions and proofs, with comments in Chinese |
| [`crates/city/templates/`](crates/city/templates/) | The documents the city writes into each building; agents read them, and so can you |
| [`crates/city/skills/`](crates/city/skills/) | The skills distributed with the release |
| [`docs/logging.md`](docs/logging.md) | What goes into the diagnostic log, what goes into the Ledger, and why the two are kept apart |
| [`docs/third-party.md`](docs/third-party.md) | The upstream facts this repository follows, credits and licence obligations |
| [`CHANGELOG.md`](CHANGELOG.md) | What each release changed, and which known problems are still unfixed |
| [`SECURITY.md`](SECURITY.md) | How to report a vulnerability |

## Sources and credits

Provider interfaces follow vendor documentation, and ACP agent launch commands follow the ACP registry, whose catalog ships with each release; where a fact needs checking precisely, it is read from the vendor's client without copying the client's code. This repository implements its interface controls itself, with keyboard behaviour following the W3C ARIA Authoring Practices, Kobalte and Ark UI documentation. Source versions and full licences are in [third-party](docs/third-party.md).

The shipped `sdd`, `tutor` and `translation` are English adaptations of the author's Chinese skills; `why`, `how` and `blast-radius` are adapted from Lauren Tan (poteto)'s [pstack](https://github.com/cursor/plugins/tree/main/pstack), and `authority-review` from Thermos in the same repository; all four keep the MIT licence. Each skill's credit and licence are in [skills/LICENSES.md](crates/city/skills/LICENSES.md).

To report a bug or request a feature, use the [issue forms](https://github.com/2youg1/sprawling-agents/issues/new/choose). For code changes, follow [AGENTS.md](AGENTS.md) and [CONTRIBUTING](docs/CONTRIBUTING.md). Report vulnerabilities through the private channel in [SECURITY.md](SECURITY.md).

The project is licensed under [MPL-2.0](LICENSE); the shipped skills each carry their own licence with their files.
