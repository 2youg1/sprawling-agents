# Operating — running work, answering it, and what to do when something breaks

> **For someone using this city every day.** It covers steering and stopping work, answering what residents ask, what each page answers, driving a city from a script, and the routes out of the situations that actually occur.
>
> It does not cover installation ([`getting-started.md`](getting-started.md)) or the design ([`../ARCHITECTURE.md`](../ARCHITECTURE.md)). The diagnostic log has its own file ([`logging.md`](logging.md)), because it is a thing to understand once rather than a step to follow.

## What a person does to running work

Every one of these is recorded as an event like anything else. Each is a command you can type into any message box, and the pages offer the same commands as controls.

| Command | What it does |
|---|---|
| `/steer <text>` | adds an instruction to the run that is going, **without stopping it** |
| `/stop` | cancels the run in front of you |
| `/halt <addr>` or `/halt --all` | stops one building, or the whole city: its runs freeze, and nothing new starts there until it is released |
| `/release <addr>` or `/release --all` | lets a halted building, or the halted city, work again |
| **allow** / **deny** | answers a design question a resident asked |

Steering is not interruption. The instruction lands at the end of the next tool result, so a request already on the wire is never rewritten mid-flight; when the run is inside a command, the instruction is heard at the command's next phase boundary. What a person said arrives at the next safe point, and the run continues from where it is rather than from where it was.

The rest of the `/` menu starts and shapes work: `/dispatch <task>` opens a run in the room the box speaks to, `/new` starts a fresh session at the same address (`/new --carry` brings the room's handoff along), `/clear` drops this conversation and starts a new session here, `/fork` starts a second line from the newest run in a room, `/raise <addr>` raises a building from a template, `/model <id>` points `main` at another model, and `/effort` sets how hard the model thinks. `/diff` opens the changes of the run in this room, `/go` opens a page by name, `/mcp` opens the tool servers page, `/doctor` checks this machine again, and `/help` lists every command. **everything** in the rail, or `Ctrl-K`, offers the same list with every page and session beside it.

## What each page answers

**the Mayor** is the conversation with `hall/mayor`, and the page the city opens on. The panel beside the conversation shows what the current run produced: the change, the file it last worked on, and what its last command printed.

**city** is the whole city as a drawing, one block per building. The bar above it says what has been spent and carries `/halt --all` and `/release --all`. Picking a building shows its progress, what is ready and what is stuck, and the runs that worked there; **open the building** goes to its page.

A building's own page is reached from the city, so that a city with fifty buildings keeps a short rail. It holds **the plan**, the building's **rooms**, its **files**, its **commits**, its **changes** and its **skills**, and the box that gives it a standing goal. **Remove** takes the building out of the city: its files move to `.sprawling/removed`, its history stays in the Ledger, and moving the folder back to the city root and adopting it brings it back. A building with runs going is not removed until they stop.

A run's page has seven lenses: **time**, **turns**, **monitor**, **prompt**, **context**, **changes** and **evidence**. **monitor** follows the run live — the terminal, the changed files and the file it is working on — and a hunk there can be reverted or commented on, which reaches the run as a steer.

**the record** is the history through four lenses. **the ledger** is the event stream, with filters that say how many rows they hid. **the archive** searches every building's shelves at the moment you ask. **the recycle bin** is every discarded file with the way back. **the log** is the diagnostic log of this process, at the level it was started with.

**cost** is money and tokens, cut five ways, each cut summing exactly to what was billed. **the registry** lists what residents filed for keeping, by kind, building and date. **MCP** is where tool servers are added, by command, by URL or from a pasted JSON block. **settings** is providers and models, who answers approvals, the network rule a new endpoint starts with, the programs this city needs, skills, appearance, keybindings, and which release this is.

Looking at a room does not empty its queue: the queue is folded from the Ledger, and only a run that takes a signal removes it.

## Answering

An item on **waiting on you** is a design question a resident could not settle by reading the rules, and it states what is being asked, who asked, and what it is about. **Nothing else waits there.** An action is never an item: a door either allows it or refuses it from the rules, and the recovery line of a refusal names the `RULES.toml` field that would change the answer. The one door that escalates — a resident asking to drive the browser you are already using — asks for your own action on that browser rather than filing an item.

- **allow** — the resident proceeds on your answer.
- **deny** — the run is told, in three parts: what was refused, why, and what it can do instead.

There is no third answer that turns one ruling into a standing waiver. A rule is edited where the rule lives, so a permission you meant to grant for good outlives the process rather than expiring with it.

Answering does not merely unblock. What was blocked is dispatched again with your answer already settled, and identical questions are grouped into one card, so answering a group of five is one action rather than five rounds of the same question.

Settings → **run** → **approvals** chooses who answers: **me**, or **the clerk**, which answers in the three parts a refusal uses and writes its reason into the Ledger. **answered for you** under the same setting lists what the clerk decided.

An item marked **outside content** began with text from outside the city — a web page, an inbound request, a tool result. The mark is shown to whoever answers, and such an item is never grouped with another. It decides nothing by itself: what taint refuses is an effect, at the doors that cannot be undone and at discard, and a question is not an effect.

## Reading cost honestly

The cost page shows shares against the billed total rather than normalising its own rows, so an unattributed remainder stays visible instead of being divided away. Where calls came back with tokens and no amount, the page says how many calls that was and how many tokens they used. A subscription reports no price at all, and a page that rendered that as `$0.00` would be inventing a fact.

**by skill** is one bucket. A tool call does not happen "under" a skill — a skill is a line of disclosure in the prefix, not a calling context — so nothing in the Ledger names the skill a call was made under, and every call lands in `no_skill`. The cut stays, reporting what it can defend.

## When something breaks

**A run ended and nothing is waiting.** Its outcome is on its page: **done**, **stopped**, **hit a limit**, or **ended**, with the reason. An ended run does not restart itself; dispatch again, or open the run's page and fork from the step before the problem.

**A provider fails.** The refusal says which way: the provider refused with a status, went silent mid-answer, broke off part-way, or answered in a shape the city cannot read, and each names what to check — the model name, the key, the dialect, the base URL, the extra headers. Calls carry a deadline, so a silent provider ends as a timeout rather than holding a run for ever. **list models** on the settings page asks the endpoint again and reports which step failed.

**A file is gone.** The recycle bin has a row for it, with the way back: the commit that still holds it, the stored copy in the content store, or how to rebuild it. A row git still holds has **put back**, which writes the file back to its path and records that it came back; it refuses when a file is already at that path, when a link stands on the path, or when the path is inside a reserved subtree. An approval card that asks about deleted files links here, because *deny* stops the work and puts nothing back.

**A setting does not take effect.** `sprawling check <city>` reads every TOML file the city holds and prints each error as `path:line:column`.

**The whole city is behaving oddly.** `/halt --all`. The halt is recorded, runs freeze, and nothing new starts until `/release --all`. Then read **the ledger** from before the trouble, or `sprawling view <city>` in a terminal, which filters the Ledger by run, kind, address, text or position and prints the run tree with `--runs`.

**The process died mid-call.** `sprawling resume <city>` verifies the chain, closes tool calls whose outcome was lost as unknown rather than as failed, and reports what waits for a person. Serving with `--supervise` does this after every crash and serves the city again, until crashes come too close together; then the supervisor stops and waits for Enter.

**Something looks wrong with the history itself.** `sprawling replay <city>/.sprawling/ledger` verifies the chain offline, read-only. If a byte was changed, it names the line and refuses to go on. This is the check to run before trusting a city exported from somewhere else.

**You need more detail than the pages give.** Raise the log level: `sprawling serve <city> --log decide` explains why verdicts came out the way they did, `--log trace` adds phase changes and retries, `--log wire` adds the bytes of frames. Every line carries the Ledger position it happened at, so a surprising log line and a surprising event meet on one integer. See [`logging.md`](logging.md).

## Driving a city from a script

`sprawling call` sends one wire frame and prints every frame that comes back, one JSON object per line. A query stops on its answer. A command stops when the city has been quiet for the `--quiet-ms` window, or, with `--until <event-kind>`, on the first event of that kind, such as `--until run_frozen`; the window then bounds the silence between two frames. With no frame, `sprawling call` prints the exit table and the name of every command and query the wire carries. **The exit code is the answer**, so a script branches on it instead of parsing the JSON.

| Exit | What it says | What to do about it |
|---|---|---|
| 0 | the city answered inside the window, and refused nothing | go on |
| 1 | the city refused; the refusal is the last frame printed, with its recovery line | read the refusal and act on it |
| 2 | this command line was not readable: a missing frame, a frame the wire cannot carry, a bad `--quiet-ms`, an `--until` that names no event kind, an unknown subcommand | fix the command; the city was never asked |
| 3 | the frame went out and **nothing came back** before the window closed, or the answer or the `--until` event did not | the city may still be working: ask again with a longer `--quiet-ms`, or read the city's own log |
| 4 | nothing at `--at` answered as a city | start the city, or point `--at` at the one that is running |

**3 is neither a failure nor a success.** A refusal can take longer than the window, and silence read as either of the first two answers is how a failure becomes a success. With `--json`, a refusal this binary writes itself, rather than a frame the city sent, goes to stderr as one JSON line with the same fields as the refusal frame, so a script reads both with one parser.

`sprawling dispatch <addr> <task>` builds the dispatch frame for you. It draws its own idempotency key, prints every frame until the run freezes, and exits by the same table; `--detach` prints the run id alone once the run starts, and `-m <id>` runs on that registered model instead of `main`'s. Given a building rather than a room, the city names the room itself.

`sprawling top` watches a served city's monitor: a screen in a terminal, one JSON line per sample otherwise. `sprawling enrol <realm/name>` reads a credential from stdin and hands it to a served city, which files it in the vault and answers with the reference.

## A second city that watches the first

A building whose `RULES.toml` says `browser = true` gets the **browser** tool, so a resident can open a page, look at it, click something, and take a screenshot of what it did. The arrangement worth learning is two cities: the first does the work, and the second watches the first one's own WebUI.

**Raise the two cities.** One port serves one city, so the second one gets another.

```bash
sprawling up ~/cities/first                         # 127.0.0.1:8787, and it opens the page
sprawling up ~/cities/watcher 127.0.0.1:8788        # in a second shell
```

**Give the watcher a building.** `/raise watchtower` in the second city's page does it, and so does one frame. In a frame, the idempotency key is yours to choose, and it is what makes sending the same frame twice one effect rather than two. The frame comes first and the flags follow it; a frame that arrives after `--at` reads as a missing frame, and the command exits 2 without asking the city anything.

```bash
sprawling call '{"command":{"create_building":{"addr":"watchtower","template":"minimal","idem":"idem1-00000000000000000000000000000001"}}}' --at 127.0.0.1:8788
```

**Let that building drive a browser.** The line goes in the building's own rules, which live where its own residents cannot write them:

```bash
echo 'browser = true' >> ~/cities/watcher/watchtower/.sprawling/RULES.toml
```

The template already wrote `confidential = false` above it. A browser opens whatever address it is given, so it is a way out of a building whose data does not leave, and a city that reads both `browser = true` and `confidential = true` refuses the pair and says which line to change rather than choosing one of them.

**Send it to look at the first city.**

```bash
sprawling dispatch watchtower 'open http://127.0.0.1:8787/, take a snapshot, press the button that starts work, screenshot what happened, and write what you saw into Memo.md' --at 127.0.0.1:8788
```

A city with no model chosen for `main` answers that in one frame, `E_MODEL_UNCHOSEN`, so choose one on the second city's settings page first. Otherwise the second city's own page shows the browser's actions going out one at a time.

**What the resident is holding.** The browser starts on the first action and stops when the run ends. Firefox is preferred because it speaks WebDriver BiDi itself and needs no driver; Chromium works when `chromedriver` is on the search path, and `sprawling doctor` says which of the two this machine has. The profile is `~/cities/watcher/.sprawling/browser-profiles/watchtower`, so a login the watchtower performs belongs to the watchtower and to no other building, and it sits in the reserved subtree, which no write domain reaches, so a run cannot edit its own stored credentials.

**A screenshot is not a picture in a log.** Its bytes go into the content store, the tool result carries the `cas:` locator and the picture's size in pixels, and the picture is attached to what the model reads, so the resident that took it can look at it on the next turn, and so can you, from the ledger row, afterwards.

**Stopping.** `Ctrl-C` in either console stops that city and closes the browser it started. The two share nothing but a machine: two directories, two Ledgers, two ports.

## Moving a city

```bash
sprawling export ~/cities/first city.bundle
sprawling restore city.bundle ~/cities/copy
sprawling replay ~/cities/copy/.sprawling/ledger
```

The manifest is the integrity test: restore walks the chain and compares it with the manifest before it reports success, so a short copy is refused there rather than becoming a city that quietly lost an hour. Verifying the chain afterwards is the step that makes the copy trustworthy rather than merely present.

## Two habits worth having

**Give a building a plan.** `Roadmap.md` is the only task table, and it is the denominator for every progress reading you will see. A building without one is not broken — the pages say there is no plan to measure against rather than inventing a percentage — but nothing can report how far along it is either.

**Let the agents keep their own notes.** `Memo.md` for what needs recording and has no other home, `Handoff.md` for the next session, the archive for what was worth keeping. They are ordinary files: readable in the browser, editable in your editor, and the same bytes either way.

## Which browser the page opens in

`sprawling up` hands the address to whatever your operating system opens links with, once the port answers — the browser you already use, with its profile and its logins. `sprawling serve` opens nothing unless given `--open`. `--no-open`, or `SPRAWLING_OPEN=never` in the environment, keeps the screen alone; a refusal beats a request. When nothing can be opened, the address is already in the console.

`SPRAWLING_BROWSER` is a different setting. It names the browser engine this city drives — the one the **browser** tool starts and `sprawling doctor` reports — and it takes precedence over every engine the doctor finds. It has no effect on which browser shows you the page.

## Parts you can replace

This repository bundles nobody's key, pays for nothing, and proxies nothing. Everything that reaches outside is therefore an adapter you can swap, and this section says where each one lives.

### Provider intelligence — followed from upstreams

Signing in to a provider means knowing four things: authorization endpoint, token endpoint, client id, scopes. Those are facts, and they change without warning, so they are followed from actively maintained projects rather than watched by hand: [`openai/codex`](https://github.com/openai/codex) for OpenAI, [`anthropics/claude-agent-sdk-typescript`](https://github.com/anthropics/claude-agent-sdk-typescript) for the Claude agent protocol types, [`xai-org/grok-build`](https://github.com/xai-org/grok-build) for xAI, and [`MoonshotAI/kimi-cli`](https://github.com/MoonshotAI/kimi-cli) for Moonshot. **What is followed is intelligence, not code**: [`third-party.md`](third-party.md) gives each upstream's licence, the path watched, the commit it is tracked to, and how to re-check.

| To do this | Change this |
|---|---|
| add or correct a subscription provider | `gateway::oauth_profiles`, a table with data and no branches |
| change how a login is begun, finished or renewed | `gateway::credential` |
| use an API key instead | the settings page: base URL, wire API, key |
| speak a third dialect | `gateway::dialect`, a pure two-way translation with the canonical shape in the middle |
| run a local model | the settings page: a loopback base URL and the chat face; `gateway::endpoint` takes a loopback address off the proxy and streams it like any other |

**What you cannot move out**: credential custody. Plaintext reaches the platform credential service and nothing else, configuration holds a `secret:realm/name` reference, and that is part of what the product promises rather than an implementation detail.

### Outside applications — MCP, and Composio as one server among many

Mail, GitHub, Figma, Discord: writing an integration for each is a weekly chore unrelated to the problem here, so the whole class is outsourced over **MCP**. [Composio](https://composio.dev) is the first choice, and it is reached the way any other server is.

| To do this | Change this |
|---|---|
| give a building tools from a server | its `CONFIG.toml`: a `command` starts a child process, a `url` reaches a hosted server; the **MCP** page writes the same entries |
| point at a different provider of the same tools | the same URL field. Nothing else changes |
| add a transport | `protocol::mcp::stdio` and `protocol::mcp::http` are the two adapters behind `protocol::mcp`'s `Outbound` seam |
| drive this city from an editor | `protocol::acp` accepts an outside request as an ordinary dispatch |

A confidential building constructs none of them: data may enter and may not leave.

#### Documents as Markdown: `markitdown-mcp`

[MarkItDown](https://github.com/microsoft/markitdown) turns PDF, Word, Excel, PowerPoint, HTML and similar files into Markdown, which a model reads far more cheaply than the original bytes. It joins a building as one more stdio MCP server, so the kernel gains no converter and no dependency on Python: the server is a child process the building's configuration names, and removing the entry removes the tools.

Install it on the machine the city runs on with `pip install markitdown-mcp`, then add one entry to the building's `.sprawling/CONFIG.toml`:

```toml
[[mcp]]
label = "docs"
command = "markitdown-mcp"
```

A `command` entry is stdio by definition, so it carries no `transport` key, and the configuration refuses one. The server offers one tool, `convert_to_markdown(uri)`, which reaches a run as `docs_convert_to_markdown`. Its `uri` may be `file:`, `data:`, `http:` or `https:`, so the same tool that converts a file in the building also fetches a page from the network.

Three limits apply today, and each is a fact about the code rather than a choice this entry can change:

- A call that has not answered within `protocol::mcp::EXTERNAL_CALL_PATIENCE` (60 s) is refused and the child is stopped. The deadline is the same for every server; `[[mcp]]` has no key to lengthen it for a slow conversion.
- One answer is at most `protocol::mcp::MESSAGE_CEILING` (8 MiB), and an answer above it ends the connection.
- An answer whose text exceeds `runtime::CONNECTOR_CAP_BYTES` (16 KiB) goes through `runtime::pipeline::package`, the same step that shapes an `exec` result (runtime-SPEC §8-27-10). Plain text is stored whole and the model reads a window with the path it pages with `read`. Markdown is the exception, and it is what this server returns: the pipeline shortens a long Markdown document to its section outline and stores nothing, so the text it drops is not reachable from that call. Convert one document, or one part of one, per call.

**A confidential building starts no MCP server at all**, `markitdown-mcp` included, because the city cannot tell a server that only converts local files from one that also fetches URLs, and the tool itself accepts `https:`. Local conversion there goes through `exec` with the command-line converter on a file inside the building (`pip install markitdown`, then `markitdown report.pdf`): that result passes through the same `runtime::pipeline::package`. The same caveat as every `exec` applies: nothing yet stops a host command from reaching the network (city-SPEC, the execution points still missing), so this stays safe only while the converter is given local paths.

### The rest

| Part | Seam or surface | Note |
|---|---|---|
| execution sandbox | `runtime::sandbox` | implement the trait, pass its conformance suite; the shipped adapter is wasmtime with fuel |
| the client | `channels::wire` | the wire is the whole API; a second client writes against it |
| the browser driver | `browser::port` | frames in, replies out; the shipped adapter speaks WebDriver BiDi |
| where views are stored | `bin::sprawling::views` | delete the process and they rebuild from the Ledger, byte-identical |
