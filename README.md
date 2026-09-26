# sprawling

**Turn a swarm of agents into a city on your own machine. One Rust binary. The UI lives in the browser.**

![binary](docs/badges/release_binary.svg) ![client](docs/badges/frontend_artifact.svg)

The binary the badges refer to is attached to the [latest release](../../releases/latest). Both numbers are produced by the build gate that weighs the artifacts, so nobody writes a size into the docs by hand.

> **Status: pre-alpha, research & development.** The main loop works: register a provider in the browser, raise a building, dispatch a job, and the model calls tools and writes files into that building. Several agents work in one city, each in its own room, and several runs of one building drive at the same time, each on a lane of the driving pool, while one accounting thread writes all of them into the Ledger.
>
> What is still missing is listed under [What works / what doesn't](#what-works--what-doesnt). Read that section before you hand it real work.
>
> 中文: [README.zh-CN.md](README.zh-CN.md) · For an agent: [LLM.md](LLM.md)

**Strengths**: tiny footprint; concepts that feel genuinely cool; built for multi-agent from the start, not a single agent with a pile of extensions.

**Weaknesses are many**: a student project with no lab or middleware sponsor behind it; no anime mascot; the WebUI wants to be good but the implementation still lags; stability and usability both need work.

---

## Why this exists

I’ve tried a lot of harnesses. Some feel conceptually outdated; others overshoot what’s actually useful. Take RSI: until the LLM itself leaves the stateless regime, a harness can only keep adapting to the newest models and learning a company’s existing workflows so it can run them faster. The first trend looks like an ablation study; the second needs privacy.

More and more small companies are appearing—tiny teams shipping online services with a large number of agents. Ninety-nine percent of them are a pile of Markdown plus a few talented people.

So I wanted a harness that keeps up with the emerging multi-agent (graph engineering) wave while remaining pragmatic about RSI and memory-related fashion. I put practical extensibility, saving the user’s attention, experimental cost control for agent scale-up, privacy & reliability, and long-running capability at the core of the design, and mixed in a few ideas from urban studies and sociology. That’s how sprawling took shape.

The stronger and larger agents become, the more expensive human attention gets. I refuse to let sprawling become just another app that tries to hijack yours. Unlike most harnesses that obsess over prompt writing, the best way to use sprawling is to shift toward the loop: design the workflow, let agents develop sprawling itself, and hand over your fixed work… so you can focus on designing new business, learning new skills, and only occasionally checking how things are running.

Honestly, no multi-agent scheme yet delivers performance gains that justify the cost of scale. But exploration of this technology for business automation, social simulation, and AI alignment is only just beginning. We still need a lot of effort and resources to study how models interact, collaborate, and exhibit social behavior inside agent clusters.

Agent memory is indeed an important path toward RSI, but not via harness-level injection. Your files, code, and document libraries *are* the memory. Attempts to make an agent truly grow with you are, before LLMs leave the stateless regime, mostly a drag on the model.

If you prefer a harness you already like, try RefRain. sprawling is aimed at persistent operations for small teams and at research platforms (computer science or the humanities/social sciences). It is still in R&D. Contributions and conversations are both welcome.

Apart from migrating the necessary business skills / MCP / ACP pieces, I recommend staying lean for now and only adding things manually when you hit a concrete problem. Even the same model behaves completely differently under different harnesses.

sprawling is designed for modest hardware, so I refuse to let multi-agent workloads explode in performance cost. That also makes it suitable for old laptops or cheap cloud boxes.

I don’t sell APIs and I can’t afford a hard drive full of your data, so everything stays local. There is a dedicated confidential building: it stops a run before any call to a remote provider and starts no outside tool server, so paired with a local model it can work on private data. The trade-off is that I cannot run enormous-scale tests myself.

---

## What it is

One binary, one browser page, and the page is embedded inside the binary at build time. **The client is replaceable**: `client/` is TypeScript with Svelte and Effect, and bun installs and builds it. It is written against the WebSocket protocol in `crates/channels`, and anything else that speaks that protocol is a client too, in whatever language you and your agents write best.

The directory tree on disk *is* the space: a **City** is a directory tree, a project is a **Building**, an agent’s workspace is a **Room**.

**One address answers three questions at once.** `lab/room1` names a place on disk, and that place settles which files this agent may write, which documents it starts with, and whom it reports to. Nothing has to keep the three answers consistent, because they are read off one fact.

**Agents find each other and speak without you relaying.** A run can ask who shares its building and gets back every address it can reach, each with the line that resident's own `URBANITE.md` offers about what to bring them, so "who do I talk to" has an answer that is not a guess. Speaking to somebody who is working slips the message under their door: it lands at the end of their next tool result. Speaking to somebody who is not starts a run for them. Either way the message arrives labelled `@` and the sender's address, which is also the address that answers it. **A resident can never render as you**: only the person's own entrance can build a message that speaks as the person, and that is a property of the type rather than a convention.

**The Ledger is the only history.** Every effect first becomes an event, then becomes an effect. Every view in the UI is a projection of that event stream: delete one, rebuild it from the Ledger, and the bytes match. Change a single byte in the log and chain verification reports the line and refuses to go on.

**Deletion comes with its own undo path.** The type that means “discard a file” has no constructor without a Restoration, so “deleted and gone forever” is not rejected at runtime; it cannot even be written. Every row in the recycle bin carries its way back, and one press on the row puts the file back where it was, unless a file you made since stands at that path.

**Cost is cut five ways**: by run, by resident, by prefix segment, by tool, and by skill. Each amount is what the provider billed for the call when it reports one, and the price sheet's figure when it does not. When a provider supplies no price at all, as with a subscription, the page says there is no price instead of printing `$0.00`, and it counts the calls and tokens that went unpriced. Zero and unknown are different things.

**The UI is designed not to bother you.** A browser notification is raised for one kind of thing only: a decision that needs you. The progress of runs reaches a hidden tab through its title and icon, and everything else waits where you will find it.

**Every component carries its SPEC beside it.** `crates/<crate>/<crate>-SPEC.md` states that crate's interfaces and the reasoning behind them, and it is written before the code and changed before the code changes. A person and an agent therefore alter this project by reading the same file. The `specalign` gate refuses a kernel enum whose variants and SPEC table differ.

**Some states are not validated; they are unrepresentable.** Sending a credential over the wire, discarding a file with no way back, putting a sealed credential into a Ledger payload, claiming work is done without evidence, giving part of a plan more weight than its parent had, and verifying your own work cannot be expressed in the type system. The tests hold <!-- xtask:begin compile_fail_cases -->18<!-- xtask:end --> compile-fail cases, because “cannot be written” is itself an assertion that has to be proven.

## Getting it running

### Quick start

1. Download the archive for your system from the [latest release](../../releases/latest): Windows (x86-64), macOS (Apple silicon) or Linux (x86-64).
2. Unpack it anywhere.
3. Run **`sprawling.exe`** (Windows: double-click it) or **`./sprawling`** (macOS and Linux). It asks one question before it creates anything.

That is the whole install. Nothing is registered, and nothing outside that folder is written to: delete the folder and it is gone. A console window opens and stays open: **that window is the city**. Your browser opens at `http://127.0.0.1:8787`; if it doesn’t, open the address yourself. `Ctrl-C` in the window stops the city.

The binaries are not code-signed, so the first run trips a warning. Windows says “Windows protected your PC”: choose **More info → Run anyway**. macOS refuses the first launch: open it once from Finder’s right-click menu.

Or fetch and unpack in one line; the script ends by running `sprawling install`, which decides where the binary goes and puts it on your PATH:

```bash
curl -fsSL https://raw.githubusercontent.com/2youg1/sprawling/main/install.sh | sh        # macOS, Linux
irm https://raw.githubusercontent.com/2youg1/sprawling/main/install.ps1 | iex             # Windows PowerShell
```

**Before it can do anything you need a model to call**: an API key for a provider speaking the OpenAI or Anthropic dialect, or a subscription login. sprawling schedules agents, records what they do, and shows it to you; it does not think by itself.

### From a terminal

One binary is enough: the page ships inside it, and running it needs no JavaScript runtime. Building `client/` from source needs bun.

If you already have bun or node, `bunx sprawling up` — or `npx sprawling up` — fetches that same binary from npm and runs it. The runtime does the fetching, not the running.

The launcher runs one command, and you can run it yourself:

```bash
sprawling up [city-dir] [addr]      # raise the city if it is not there, serve it, open the WebUI
```

Taken apart, when you want the steps separately:

```bash
sprawling init  <city-dir>          # found a city; the name is written into the genesis record
sprawling serve <city-dir> [addr]   # serve a city that already exists; loopback only by default
# then open http://127.0.0.1:8787
```

`up --supervise` serves the city in a child process and, after a crash, resumes it and serves again, until the crashes come too fast to be worth another try.

> **Don’t `cargo install` this.** The client is built by [bun](https://bun.sh) before the binary and embedded into it. A plain cargo build cannot run that step, and yields a binary whose page is blank. Take a release archive, or build it with `just dist`.

Four steps on the page:

1. **settings** — enter the provider’s base URL, dialect (OpenAI or Anthropic), and key. The key goes straight into the OS credential store; after that the page only ever sees a reference of the form `secret:realm/name`.
2. On the same page, pick a model for each role: `main` does the thinking, `digest` reads long documents for it, and `transcribe`, if you want it, turns recordings into text.
3. Raise a building: type `/raise lab` in the box at the bottom of the page.
4. In the box, say what should be produced and what counts as done, and send it. **It never asks for a budget**: nobody can price a job before it runs, and subscriptions have no unit price anyway. Actual spend is reported from the record afterwards. **Nothing rations a conversation either**: when agents wake each other, how long they go on is theirs to decide. A single run has no turn ceiling: it runs until it concludes, and what stops one that should not go on is `/stop`, or `/halt` for a whole building or the city.

The page opens on a conversation with the Mayor, who plans work across buildings. Hand the Mayor an idea when you do not yet know which building it belongs in.

Other commands:

```bash
sprawling install [--uninstall]      # make `sprawling` a word your shell resolves, or take it back off
sprawling doctor [<city>] [--install] [--explain <code>]
                                     # what this machine has against what a city needs; --install offers each missing item, one at a time; --explain connects a refusal code to this machine
sprawling enrol <realm>/<name>       # read a credential from stdin and hand it to a city; it never touches the command line
sprawling dispatch <addr> <task>     # send one task to a served city and print its events until the run ends; -m <id> picks the model
sprawling call '<frame>'             # send one wire frame, print every frame back (see LLM.md)
sprawling top                        # watch a served city's monitor: a screen on a terminal, one JSON line a second otherwise
sprawling view <city>                # read a city's Ledger lines or its run tree, read-only
sprawling check <city>               # read every TOML file a city holds; print each error as path:line:column
sprawling resume <city-dir>          # after a restart: verify the chain, close tool calls whose results are lost, report who is waiting for a human
sprawling fork <city> <run> <seq> <addr>  # branch a lineage from one step of a run
sprawling adopt <city> <addr>        # take a directory already inside the city in as a building, without overwriting any file
sprawling replay <ledger-dir>        # offline chain verification, read-only
sprawling whose <city> <commit>      # which run wrote a commit this city made, answered from the Ledger
sprawling export <city> <bundle-dir> # pack a whole city
sprawling restore <bundle-dir> <city> # unpack a bundle on another machine
sprawling status [--deps] [--check]  # this binary: version, client, what it is built from; --check asks npm for a newer release
sprawling help                       # every command, on one screen
```

Launched with no command at all, by double-clicking it for instance, it shows a single screen, names the folder it would create, and waits for you to agree before creating anything. Founding a city writes the genesis record, and that does not happen because somebody double-clicked a file.

A step-by-step walk from empty directory to first Run lives in [`docs/getting-started.md`](docs/getting-started.md) ([中文](docs/getting-started.zh-CN.md)).

### Staying current

Nothing here updates itself, and nothing checks for a release unless you ask it to. Every release is a pre-alpha, so what changes between two of them is worth reading before you take it.

```bash
sprawling version                   # which release this is, and the day it was cut
sprawling status --check            # ask npm whether a newer one is published
```

The **machine** section of the settings page has the same check behind a button. Both print what to run and stop there: updating replaces a binary, and which binary you replace depends on how you installed it.

```bash
bunx sprawling@latest up            # if npm is how you run it
```

Installed from an archive, download the new one and run `sprawling install` again. Neither route touches a city's folder.

## History

A city works inside a git repository that is usually **yours**, and it leaves two kinds of history behind.

**The Ledger is the city's own history**, and the only one: every effect becomes a line before it becomes anything else, the chain is verifiable offline, and every page you read is a projection of it.

**Git is the restoration authority for files.** Before each wave of tool calls the city commits what is there, so anything that disappears has a commit to come back from. Those fences do not land on your branch: they are commits nobody's `HEAD` points at, kept alive by a reference under `refs/sprawling/runs/`. `git log` therefore does not grow one line per tool wave, and your own history stays the shape you left it. Only two things move a branch: the first commit of a repository that had none, and the merge that lands a reviewed piece of work.

**Every commit the city makes says who made it.** The author is the resident's own address at a mailbox derived from the city — `lab/parser@1a2b3c4d5e6f.sprawling`, unroutable on purpose, because it identifies a city rather than promising to deliver mail — and the message carries git trailers:

```
Sprawling-Run: <run id>
Sprawling-Actor: lab/parser
Sprawling-Model: <model id>
Sprawling-Effort: high
Sprawling-City: <city hash>
```

A run that replaced another adds `Sprawling-Predecessor: <run id>`. The block is git's own trailer syntax, so `git interpret-trailers --parse` reads it with no help from us. **The trailers are a projection for readers outside the city, not a second history**: where a trailer and the Ledger disagree, the trailer is the side that is wrong.

**The question also reads backwards.** Given a full forty-digit commit id, `sprawling whose` answers which run wrote it, out of the Ledger and never out of git, so a city exported and restored somewhere else with no `.git` beside it still answers:

```
$ sprawling whose ./mycity <commit id>
run     <run id>
actor   lab/parser (session refactor-the-ledger)
model   <model id> (effort high)
ledger  seq 4127
```

Exit code 1 means this city has no record of writing that commit. That is a different answer from "nothing changed", and it is the one an audit needs.

## Five words

| Word | What it is |
|---|---|
| **City** | One city on one machine: a directory tree, one Ledger, one complete history. Two cities never reference each other. |
| **Building** | A building inside the city; one building, one business line. Configuration, Archive, and WriteDomain are all scoped to it. |
| **Room** | A room inside a building, i.e. a subdirectory. One agent works in one room. |
| **Run** | A piece of work with a beginning and an end. **Resident is identity; Run is cost.** |
| **Ledger** | The only history. One line, one event; append-only; offline-verifiable. |

The rest of the vocabulary is in [`docs/glossary.md`](docs/glossary.md).

## What works / what doesn't

**Works**: register a provider and select models; raise a building, dispatch work, and let the model call tools and write files into that building; residents find each other, speak, and wake each other without a person relaying a message; attach an external MCP server to a building; several agents at work in one city, each with its own git worktree, changes merging back only after another resident has reviewed them (verifying your own work is a compile error, not a rule); a building working towards a goal drives its whole ready set at once; a run given no model continues on the one its room started with; a new run waits while memory is tight, and a dispatch is refused before anything is written when the city's disk is close to full; pause a city and release it; put a discarded file back from the recycle bin; take a building out of the city, with its files kept under the reserved subtree and its history kept in the Ledger; offline chain verification; export a city and restore it on another machine.

The page has a conversation with any room (the Mayor's by default), the city, each building, each run, the record (Ledger, archive, recycle bin, log), cost, the registry, MCP, a performance monitor, and settings.

**Not done, and why**:

| Missing piece | Reason |
|---|---|
| An OS sandbox on every platform | A command the agent runs is confined by whatever the platform offers, and the exec tool's own description names which arm it got and what that arm does not hold. On Linux with a namespace wrapper installed, the command runs in namespaces of its own. On Windows and macOS it runs in a copy of the working tree, so your files are safe from it but the network is open to it. Isolation that nobody verified is worse than none, because people will treat it as a defense. The claim today is therefore “a deletion can be undone,” not “a deletion cannot happen.” |
| Browser end-to-end in CI | CI opens every settled screen in a real browser engine on fixtures (`cargo xtask render`), but no CI job drives a live city through a browser. |
| Byte-identical builds across machines | `cargo xtask repro` builds the release binary twice from one tree and compares the bytes, and a nightly job runs it. Two machines building the same tree still record different source paths, and removing them needs a compiler switch this project's pinned toolchain does not offer. |
| Attributing spend to skills | A tool call does not happen “under” a skill: a skill is a disclosure line in the prefix, not call context. The cost page keeps a by-skill cut, and every call lands in its `no_skill` bucket. |

## What you can swap

I sell neither APIs nor account hosting, so everything external sits on a seam and can be replaced without touching the rest:

| Piece | Lives in | How to replace |
|---|---|---|
| Subscription-login intel (followed from the four harness families listed in [`docs/third-party.md`](docs/third-party.md) §1) | `gateway::oauth_profiles` (data only, zero branches), `gateway::credential` (flow & renewal) | Add one profile line. **Credential custody is never outsourced**: plaintext reaches only the local credential store. |
| Model endpoint & dialect | `gateway::endpoint`, `gateway::dialect` | Enter base URL and dialect on the settings page. By default a model served on this machine is called directly rather than through the machine's proxy, and a setting changes that. |
| SaaS & external tools ([Composio](https://composio.dev) is one MCP server among others) | the `protocol::mcp` `Outbound` seam, `protocol::mcp::stdio`, `protocol::mcp::http` and `protocol::mcp::sse`, the broker in `protocol::mcp::broker`, the building’s `CONFIG.toml` | Change one URL or one command to switch servers; confidential buildings start none. |
| Sandbox | `runtime::sandbox` seam (the current adapter is wasmtime with a fuel budget); `runtime::tools` confinement for host commands | Implement the seam and pass its conformance assertion suite. |
| Client | `channels::wire` is the sole API surface | Want a second client? Write against this wire format. [`LLM.md`](LLM.md) is the same surface, written for an agent. |

Location and replacement steps for each piece are in [`ARCHITECTURE.md`](ARCHITECTURE.md).

## A sister repository: [kusanagi](https://github.com/2youg1/kusanagi)

Inside one city, history is **one chain**. Every effect becomes an event on a single append-only Ledger before it becomes an effect, and the single total order is what makes five ordinary questions answerable at all: who claimed this work, whose edits collided, which goal wins, has this message already been delivered, did anybody write since I read. Every one of them asks *who was first*, and a city that split its history could no longer say.

Between machines, that same total order is the thing you must not have. `kusanagi` is a decentralised collaboration network for agents: **one chain per pair**, every address derived so that no two drops of one conversation are relatable by the host carrying them, and the host is one neither party runs or trusts. A global order there would be a fact an observer could read.

One chain inside a city, one chain per pair between cities. The two repositories are halves of one answer to how agents keep a history they can trust; read either alone and the other half reads as missing.

## Where it listens, where credentials live

**It listens on loopback only by default.** To let another machine on the same network connect, bind a non-loopback address. Such an address always needs a pairing token: the city adopts `SPRAWLING_PAIRING_TOKEN` when it is set, and otherwise mints a token for this serve and prints the address to open with it. The port is never open without a token. Beyond that, this repository ships neither tunnel nor relay: each of those carries its own trust model, and choosing one for you would be making a security decision on your behalf.

**Credential plaintext never enters any file, any event, or any log.** Keys go into the OS credential store; configuration keeps only `secret:realm/name`. What a model says passes a secret scan before it becomes a Ledger payload, the log passes the same scan, and content staged for a git fence is scanned before it is committed, so a key the model happens to echo never becomes permanent history.

## Documentation

Apart from this page and the getting-started guide, the docs are in English.

- Just arrived and want to know what this is: this page is enough; one level deeper is [`docs/glossary.md`](docs/glossary.md).
- Want to put it to work: [`docs/getting-started.md`](docs/getting-started.md) → [`docs/operating.md`](docs/operating.md).
- An agent driving a city from outside: [`LLM.md`](LLM.md).
- Want to change it: [`ARCHITECTURE.md`](ARCHITECTURE.md) → [`AGENTS.md`](AGENTS.md) → the code and tests of the neighbouring modules.

Also available: [`CHANGELOG.md`](CHANGELOG.md) (what each release changed), [`SECURITY.md`](SECURITY.md) (how to report a vulnerability), [`docs/logging.md`](docs/logging.md) (why logs are not history), [`docs/frontend-method.md`](docs/frontend-method.md) (how a screen is built and accepted), [`docs/third-party.md`](docs/third-party.md) (whose shoulders we stand on, and the license obligations). [`docs/City.md`](docs/City.md) and [`docs/templates/`](docs/templates/) are the documents the city writes into buildings: agents read them, and so can you.

## Contributing

Start with [`AGENTS.md`](AGENTS.md). The thirty-second version:

```bash
cargo install just --locked
just prereqs                        # every other tool the loop needs, with the install line for each one that is absent
just check
```

When `just check` is green, a change is finished. **PR bodies, issues, and review comments may be written in your native language.** If you can, attach a parallel translation (English if your native language is not English, Chinese if it is): side by side, both humans and agents read faster, and a mistranslation is visible instead of silent. [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md) has the rest.

## Standing on the shoulders of others

Logging into a provider requires a small set of endpoints and parameters. Rather than stare at those API docs myself, I follow the vendors' own actively maintained harnesses — OpenAI's `codex`, Anthropic's agent SDK, xAI's `grok-build`, Moonshot's `kimi-cli` — one for each of the four families this city signs in to directly.

**Which repository, which path inside it, and which commit has been read live in one place: [`docs/third-party.md`](docs/third-party.md) §1.** That table is what a daily workflow parses to ask each upstream whether it moved, so a copy of it here would be a second home for a fact a machine already depends on, and the two would drift on the first move.

**What is followed is intelligence, not code.** Endpoints and parameters are facts; the flow and credential custody are implemented here. That holds for an upstream under a proprietary licence exactly as it holds for one under Apache-2.0, and one of the four is proprietary.

**The browser page stands on the same kind of thing.** Its runtime dependencies are `svelte`, `effect`, and the `@lezer` syntax highlighter with its grammars (MIT), which the page downloads only when it first shows code; no component library is among them: every control in `client/src/views/parts/` is this repository's own. What is taken from the W3C's ARIA Authoring Practices and from the Kobalte and Ark UI documentation is behaviour published as prose: which pattern a control implements, what each key does, where the focus returns when it closes. **Not one line of their code is in this tree, so nothing is owed for it**, and the keyboard table that reading produced is specified in [`client/client-SPEC.md`](client/client-SPEC.md).

**The skills under [`skills/`](skills/) stand on earlier work, and say so.** Three of them — `sdd`, `tutor`, `translation` — are English translations and adaptations of Chinese-language skills I wrote and published as open source under AGPL-3.0-or-later (the translation skill's original byline also credits Claude Fable 5); here they carry MPL-2.0 like the rest of this tree. Three more — `why`, `how`, `blast-radius` — are my modified adaptations of [pstack](https://github.com/cursor/plugins/tree/main/pstack) (Lauren Tan (poteto), MIT); they keep their licence, and every file names me as the one who modified it. `authority-review` is a modified adaptation of the Thermos plugin in the same `cursor/plugins` tree (MIT): its second pass is recut as the one-fact-one-authority audit I wrote for the configuration this city was built against. `skills/LICENSES.md` travels with the directory, in the release archive too. This paragraph is the acknowledgment; [`docs/third-party.md`](docs/third-party.md) §5 is the terms.

Connections to external applications are likewise outsourced: the city speaks MCP to any MCP server, and Composio is one of them. This repository carries no one’s keys, pays for no one, and acts as no proxy. The full list, how to re-verify, and how licenses are handled live in [`docs/third-party.md`](docs/third-party.md). Licenses of code dependencies are checked one by one by `cargo deny`; the allow-list is [`deny.toml`](deny.toml).

## License

MPL-2.0 — see [`LICENSE`](LICENSE). Under [`skills/`](skills/), my three skills carry MPL-2.0 like the rest of the tree, and the four adaptations of `cursor/plugins` keep MIT; all seven ship in the release archive with `skills/LICENSES.md`. Terms and credit: [`docs/third-party.md`](docs/third-party.md) §5.

---

Questions, bug reports and disagreements are all welcome: open an issue, or write to the address on my profile.
