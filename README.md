# sprawling

**Run many agents on your own machine as a city. One Rust binary; the interface is a page in your browser.**

[![npm](https://img.shields.io/npm/v/sprawling?logo=npm&labelColor=171717&color=CB3837)](https://www.npmjs.com/package/sprawling) [![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/2youg1/sprawling-agents) [![License](https://img.shields.io/github/license/2youg1/sprawling-agents?labelColor=171717&color=4C8BF5)](LICENSE)

> **Status: pre-alpha, research and development.** The main loop works: register a provider in the browser, raise a building, dispatch a job, and the model calls tools and writes files into that building. Several agents work in one city, each in its own room, and several runs of one building drive at the same time, while one accounting thread writes all of them into the Ledger. What is still missing is listed under [What works, and what does not](#what-works-and-what-does-not); read that section before you hand it real work.
>
> 中文：[README.zh-CN.md](README.zh-CN.md) · For an agent driving a city from outside: [LLM.md](LLM.md) · To change the code: [AGENTS.md](AGENTS.md)

## What it is

If you have used Claude Code, Codex CLI or a similar terminal agent, you know the shape of one agent: a model, a set of tools, one working folder, one conversation. sprawling keeps that shape and puts many of them in one place. A **city** is a directory on your machine. Each project in it is a **building**, each agent works in a **room** of a building, and each piece of work is a **run** with a start and an end. You talk to the city through a page that the binary serves on `127.0.0.1:8787`.

If you have only used a chat window, the difference is that an agent does work rather than answering. You describe a result and what counts as done; the model then reads files, runs commands, writes code and documents, and asks you only when it reaches a decision it cannot make from the rules you wrote down. Everything it did is recorded, and every file it deleted can be put back.

sprawling does not think by itself. It needs a model to call: an API key for a provider that speaks the OpenAI or the Anthropic format, or a local model server. It schedules the agents, gives them tools, keeps the history, and shows you all of it.

**Who it is for.** Small teams who want a set of agents to keep running fixed work for them, and researchers — in computer science or in the humanities and social sciences — who want to watch how agents divide work and talk to each other. It is designed for modest hardware, so an old laptop or a cheap cloud machine can hold a city.

## Quick start

1. Download the archive for your system from the [latest release](../../releases/latest) — Windows (x86-64), macOS (Apple silicon) or Linux (x86-64) — and unpack it anywhere. Or install in one line:

   ```bash
   curl -fsSL https://raw.githubusercontent.com/2youg1/sprawling/main/install.sh | sh    # macOS, Linux
   irm https://raw.githubusercontent.com/2youg1/sprawling/main/install.ps1 | iex         # Windows PowerShell
   ```

   With bun or node already installed, `bunx sprawling up` or `npx sprawling up` fetches the same binary from npm.

2. Raise a city and open it:

   ```bash
   sprawling up ~/cities/first
   ```

   The terminal becomes the city's console, and the browser opens at `http://127.0.0.1:8787`. `Ctrl-C` in the console stops the city. Run with no arguments, the binary names the folder it would create and waits for you to agree first.

3. On the page, **connect a provider**: a base URL, the format it speaks, and a key. The key goes into your operating system's credential store, and the page only ever sees a reference of the form `secret:realm/name`. Choose a model for the `main` role.

4. Tell the Mayor what you want, in the box at the bottom of the page. The Mayor plans, raises buildings for the work, and hands each building its part; you watch the runs, answer what they ask, and read the changes they made.

The binaries are not code-signed, so the first run trips a warning: on Windows choose **More info → Run anyway**, and on macOS open the binary once from Finder's right-click menu. Do not `cargo install` this: the page is built by bun and embedded into the binary, and a plain cargo build yields a blank page. Take a release archive, or build with `just dist`.

**[`docs/getting-started.md`](docs/getting-started.md) is the full guide** ([中文](docs/getting-started.zh-CN.md)): every concept a newcomer meets — harness, provider, model roles, skills, templates, sessions, approvals — and the whole loop from an empty directory to a reviewed merge on your branch.

## Five words

| Word | What it is |
|---|---|
| **City** | One city on one machine: a directory tree, one Ledger, one complete history. Two cities never reference each other. |
| **Building** | One project inside the city. Configuration, the archive and the write domain are scoped to it, and its rules live in `RULES.toml`. |
| **Room** | A subdirectory of a building. One agent works in one room. |
| **Run** | A piece of work with a beginning and an end. **A resident is an identity; a run is the cost.** |
| **Ledger** | The only history. One line per event, append-only, verifiable offline. |

The rest of the vocabulary is in [`docs/glossary.md`](docs/glossary.md).

## How it behaves

**One address answers three questions at once.** `lab/room1` names a place on disk, and that place settles which files the agent there may write, which documents it starts with, and whom it reports to. Nothing has to keep the three answers consistent, because they are read off one fact.

**Agents find each other and speak without you relaying.** A run can ask who shares its building and gets back every address it can reach, each with the line that resident's own `URBANITE.md` offers about what to bring them. Speaking to somebody who is working slips the message under their door: it lands at the end of their next tool result. Speaking to somebody who is not starts a run for them. Either way the message arrives labelled with the sender's address. A resident can never write as you: only the person's own entrance can build a message that speaks as the person, and that is a property of the type rather than a convention.

**The Ledger is the only history.** Every effect first becomes an event and then becomes an effect. Every view in the page is a projection of that event stream: delete one, rebuild it from the Ledger, and the bytes match. Change a single byte in the log, and chain verification names the line and refuses to go on.

**Deletion carries its own way back.** The type that means "discard a file" has no constructor without a restoration, so "deleted and gone for ever" cannot be written. Every row in the recycle bin states its way back, and one press puts the file back where it was, unless a file you made since stands at that path.

**The work you merge was reviewed by somebody else.** Several agents work in one city, each on its own git worktree, and their changes merge back only after another resident has verified them. Verifying your own work is a compile error, not a rule anybody has to remember.

**Cost is cut five ways**: by run, by resident, by prefix segment, by tool and by skill. Each amount is what the provider billed when it reports one, and the price sheet's figure when it does not. When a provider supplies no price at all, as with a subscription, the page says there is no price instead of printing `$0.00`, and it counts the calls and tokens that went unpriced.

**The page is designed not to take your attention.** A browser notification is raised for one kind of thing only: a decision that needs you. Progress reaches a hidden tab through its title and icon, and everything else waits where you will find it.

**Some states are unrepresentable rather than validated.** Sending a credential over the wire, discarding a file with no way back, putting a sealed credential into a Ledger payload, claiming work is done without evidence, giving part of a plan more weight than its parent had, and verifying your own work cannot be expressed in the type system. The tests hold <!-- xtask:begin compile_fail_cases -->18<!-- xtask:end --> compile-fail cases, because "cannot be written" is itself an assertion that has to be proven.

## What works, and what does not

**Works**: registering a provider and choosing models; raising a building, dispatching work, and letting the model call tools and write files into that building; residents that find each other, speak, and wake each other without a person relaying a message; attaching an external MCP server to a building; several agents at work in one city, each on its own git worktree, their changes merged only after another resident reviewed them; a building working towards a standing goal, driving its whole ready set at once; a run given no model continuing on the one its room started with; a new run that waits while memory is tight, and a dispatch refused before anything is written when the city's disk is close to full; pausing a city and releasing it; putting a discarded file back from the recycle bin; taking a building out of the city with its files kept under the reserved subtree and its history kept in the Ledger; offline chain verification; exporting a city and restoring it on another machine; opening a file the monitor shows at its line in VS Code, VS Code Insiders, VSCodium, Cursor, Windsurf or Zed.

The page has a conversation with any room (the Mayor's by default), the city, each building, each run, the record (Ledger, archive, recycle bin, log), cost, the registry, MCP, a performance monitor, and settings.

**Not done, and why**:

| Missing piece | Reason |
|---|---|
| An OS sandbox on every platform | A command the agent runs is confined by what the platform offers, and the exec tool's own description names which arm it got and what that arm does not hold. On Linux with a namespace wrapper installed, the command runs in namespaces of its own. On Windows and macOS it runs in a copy of the working tree, so your files are safe from it but the network is open to it. Isolation that nobody verified is worse than none, because people treat it as a defence, so the claim today is "a deletion can be undone", not "a deletion cannot happen". |
| A page laid out for a phone | The render gate judges the page at 768, 1280 and 2560 CSS pixels, so a tablet held upright is the narrowest screen it is known to fit. A phone is narrower than anything that gate checks, and a city reached from outside your own network needs a tunnel you choose (see below). |
| Browser end-to-end in CI | CI opens every settled screen in a real browser engine on fixtures (`cargo xtask render`), but no CI job drives a live city through a browser. |
| Byte-identical builds across machines | `cargo xtask repro` builds the release binary twice from one tree and compares the bytes, and a nightly job runs it. Two machines building the same tree still record different source paths, and removing them needs a compiler switch the pinned toolchain does not offer. |
| Attributing spend to skills | A tool call does not happen "under" a skill: a skill is a disclosure line in the prefix, not a calling context. The cost page keeps a by-skill cut, and every call lands in its `no_skill` bucket. |

## From the terminal

The page is not the only door. Every verb below talks to the same city, and [`LLM.md`](LLM.md) documents the wire for an agent or a script.

```bash
sprawling up [city-dir] [addr]        # raise the city if it is not there, serve it, open the page
sprawling init <city-dir>             # found a city; the name is written into the genesis record
sprawling serve <city-dir> [addr]     # serve a city that already exists; loopback only by default
sprawling dispatch <addr> <task>      # send one task to a served city and print its events until the run ends; -m <id> picks the model
sprawling call '<frame>'              # send one wire frame, print every frame back; the exit code is the answer
sprawling top                         # watch a served city's monitor
sprawling view <city>                 # read a city's Ledger lines or its run tree, read-only
sprawling check <city>                # read every TOML file a city holds; print each error as path:line:column
sprawling doctor [<city>] [--install] # what this machine has against what a city needs
sprawling enrol <realm>/<name>        # read a credential from stdin and hand it to a city
sprawling resume <city-dir>           # after a crash: verify the chain, close lost tool calls, report who waits for you
sprawling fork <city> <run> <seq> <addr>  # branch a lineage from one step of a run
sprawling adopt <city> <addr>         # take a directory already inside the city in as a building
sprawling whose <city> <commit>       # which run wrote a commit this city made, answered from the Ledger
sprawling replay <ledger-dir>         # offline chain verification, read-only
sprawling export <city> <bundle-dir>  # pack a whole city; `restore` unpacks it on another machine
sprawling install [--uninstall]       # put `sprawling` on your PATH, or take it back off
sprawling status [--check]            # this binary; --check asks npm whether a newer release exists
sprawling help [<verb>]               # every command, or one explained
```

`up --supervise` serves the city in a child process and, after a crash, resumes it and serves again, until the crashes come too fast to be worth another try. Nothing updates itself: `sprawling status --check`, or the button under **settings**, tells you whether a newer release is published, and replacing the binary stays your command to run.

## History in git

A city usually works inside a git repository that is yours, and it leaves your history the shape you left it. Before each wave of tool calls it commits what is there, so anything that disappears has a commit to come back from, but those fences are commits nobody's `HEAD` points at, kept under `refs/sprawling/runs/`. Only two things move a branch: the first commit of a repository that had none, and the merge that lands a reviewed piece of work.

Every commit the city makes names the resident that made it, and carries git trailers that `git interpret-trailers --parse` reads without help:

```
Sprawling-Run: <run id>
Sprawling-Actor: lab/parser
Sprawling-Model: <model id>
Sprawling-Effort: high
Sprawling-City: <city hash>
```

The trailers are a projection for readers outside the city; where a trailer and the Ledger disagree, the trailer is wrong. The question also reads backwards: `sprawling whose <city> <commit>` answers which run wrote a commit, out of the Ledger, so a city restored somewhere with no `.git` beside it still answers.

## Where it listens, where credentials live

**It listens on loopback only by default.** To let another machine on the same network connect, bind a non-loopback address. Such an address always needs a pairing key: the city adopts `SPRAWLING_PAIRING_TOKEN` when it is set, and otherwise mints a key for this serve and prints the address to open with it. The port is never open without one. This repository ships neither a tunnel nor a relay, because each carries its own trust model, and choosing one for you would be a security decision made on your behalf.

**Credential plaintext never enters a file, an event, or a log.** Keys go into the OS credential store, and configuration keeps only `secret:realm/name`. What a model says passes a secret scan before it becomes a Ledger payload, the log passes the same scan, and content staged for a git fence is scanned before it is committed, so a key the model happens to echo never becomes permanent history.

**Everything stays on your machine.** There is no account, no telemetry and no hosted service. A **confidential** building stops a run before any call to a remote provider and starts no outside tool server, so paired with a local model it can work on private data.

## What you can swap

Everything external sits on a seam and can be replaced without touching the rest:

| Piece | Lives in | How to replace |
|---|---|---|
| Model endpoint and format | `gateway::endpoint`, `gateway::dialect` | Enter a base URL and a format on the settings page. A model served on this machine is called directly rather than through the machine's proxy, and a setting changes that. |
| SaaS and external tools ([Composio](https://composio.dev) is one MCP server among others) | the `protocol::mcp` `Outbound` seam and its stdio, HTTP and SSE adapters; a building's `CONFIG.toml` | Change one URL or one command to switch servers; confidential buildings start none. |
| Sandbox | the `runtime::sandbox` seam (wasmtime with a fuel budget today); `runtime::tools` confinement for host commands | Implement the seam and pass its conformance suite. |
| Client | `channels::wire` is the whole API | Write a second client against the wire; [`LLM.md`](LLM.md) is the same surface written for an agent. |

Where each piece lives and how to replace it is in [`ARCHITECTURE.md`](ARCHITECTURE.md).

## Why this exists

I have tried many harnesses. Some feel conceptually dated, others overshoot what is useful. Take recursive self-improvement: until the model itself leaves the stateless regime, a harness can only keep adapting to the newest models and learn a company's existing workflows so it runs them faster. The first trend looks like an ablation study; the second needs privacy.

More and more small companies are tiny teams shipping online services with many agents, and most of them are a pile of Markdown plus a few talented people. I wanted a harness that keeps up with multi-agent work while staying practical about self-improvement and memory: practical extensibility, saving the person's attention, cost control while agents scale up, privacy and reliability, and long-running operation sit at the core of the design, mixed with a few ideas from urban studies and sociology.

The stronger agents become, the more expensive human attention gets, and sprawling is not meant to be one more application that tries to take it. The best way to use it is to move from writing prompts to designing loops: lay out the workflow, let agents do the fixed work, and come back now and then to see how it runs. Your files, code and documents are the memory; a harness that injects "memory" into a stateless model mostly slows the model down.

No multi-agent scheme yet delivers gains that justify the cost of scale, and the study of how models interact, collaborate and behave socially inside agent clusters has only begun. Apart from the skills, MCP servers and ACP pieces your work needs, keep a city lean and add things when a concrete problem asks for them: the same model behaves differently under different harnesses. If you prefer a harness you already like, try RefRain.

**Strengths**: a small footprint, concepts that are fun to work with, and a design built for many agents from the start rather than one agent with a pile of extensions. **Weaknesses**: a student project with no lab or sponsor behind it, a page that still lags what it wants to be, and stability and usability that both need work.

## A sister repository: [kusanagi](https://github.com/2youg1/kusanagi)

Inside one city, history is one chain. Every effect becomes an event on a single append-only Ledger, and the single total order is what makes ordinary questions answerable: who claimed this work, whose edits collided, which goal wins, has this message been delivered, did anybody write since I read. Between machines, that total order is what you must not have. `kusanagi` is a decentralised collaboration network for agents with one chain per pair, every address derived so that no two drops of one conversation are relatable by the host carrying them. One chain inside a city, one chain per pair between cities: the two repositories are halves of one answer to how agents keep a history they can trust.

## Documentation

Apart from this page and the getting-started guide, the docs are in English.

| You want to | Read |
|---|---|
| know what this is | this page, then [`docs/glossary.md`](docs/glossary.md) |
| put it to work | [`docs/getting-started.md`](docs/getting-started.md), then [`docs/operating.md`](docs/operating.md) |
| drive a city from a script or another agent | [`LLM.md`](LLM.md) |
| change it | [`ARCHITECTURE.md`](ARCHITECTURE.md), [`AGENTS.md`](AGENTS.md), [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md) |
| change a screen | [`docs/frontend-method.md`](docs/frontend-method.md) |

Also: [`CHANGELOG.md`](CHANGELOG.md) (what each release changed), [`SECURITY.md`](SECURITY.md) (how to report a vulnerability), [`docs/logging.md`](docs/logging.md) (why logs are not history), [`docs/third-party.md`](docs/third-party.md) (whose shoulders this stands on, and the licence obligations). [`docs/City.md`](docs/City.md) and [`docs/templates/`](docs/templates/) are the documents the city writes into buildings: agents read them, and so can you.

## Contributing

Start with [`AGENTS.md`](AGENTS.md). The short version:

```bash
cargo install just --locked
just prereqs        # every other tool the loop needs, with the install line for each one that is absent
just check          # a change is finished when this is green
```

Pull request descriptions, issues and review comments may be written in your own language. If you can, attach a parallel translation — English if your language is not English, Chinese if it is — because side by side both people and agents read faster, and a mistranslation is visible instead of silent. [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md) has the rest.

## Standing on the shoulders of others

Reaching a vendor by host name requires knowing the path its API hangs under and which format it answers in. Those are facts, and where a vendor's own client states them more precisely than its documentation, the client is followed rather than copied: which repository, which path inside it, and which commit has been read live in one place, [`docs/third-party.md`](docs/third-party.md) §1, which a daily workflow parses to ask each upstream whether it moved. No vendor's login is followed. The city signs in to no subscription; a key is the only credential it takes, and its custody is implemented here.

The browser page stands on the same kind of thing. Its runtime dependencies are `svelte`, `effect`, and the `@lezer` syntax highlighter with its grammars (MIT), which the page downloads only when it first shows code; no component library is among them, and every control in `client/src/views/parts/` is this repository's own. What is taken from the W3C's ARIA Authoring Practices and from the Kobalte and Ark UI documentation is behaviour published as prose: which pattern a control implements, what each key does, where the focus returns when it closes. Not one line of their code is in this tree, so nothing is owed for it, and the keyboard table that reading produced is specified in [`client/client-SPEC.md`](client/client-SPEC.md).

The skills under [`skills/`](skills/) stand on earlier work, and say so. Three of them — `sdd`, `tutor`, `translation` — are English translations and adaptations of Chinese-language skills I wrote and published under AGPL-3.0-or-later (the translation skill's original byline also credits Claude Fable 5); here they carry MPL-2.0 like the rest of this tree. Three more — `why`, `how`, `blast-radius` — are my modified adaptations of [pstack](https://github.com/cursor/plugins/tree/main/pstack) (Lauren Tan (poteto), MIT); they keep their licence, and every file names me as the one who modified it. `authority-review` is a modified adaptation of the Thermos plugin in the same `cursor/plugins` tree (MIT): its second pass is recut as the one-fact-one-authority audit I wrote for the configuration this city was built against. `skills/LICENSES.md` travels with the directory, in the release archive too. This paragraph is the acknowledgment; [`docs/third-party.md`](docs/third-party.md) §5 is the terms.

Connections to outside applications are outsourced in the same way: the city speaks MCP to any MCP server, and Composio is one of them. This repository carries no one's keys, pays for no one, and acts as no proxy. Licences of code dependencies are checked one by one by `cargo deny`, against the allow-list in [`deny.toml`](deny.toml).

## License

MPL-2.0 — see [`LICENSE`](LICENSE). Under [`skills/`](skills/), my three skills carry MPL-2.0 like the rest of the tree, and the four adaptations of `cursor/plugins` keep MIT; all seven ship in the release archive with `skills/LICENSES.md`. Terms and credit: [`docs/third-party.md`](docs/third-party.md) §5.

---

Questions, bug reports and disagreements are all welcome: open an issue, or write to the address on my profile.
