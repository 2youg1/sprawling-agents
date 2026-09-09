# Concepts and vocabulary

> **For anyone who has to say what this thing does** — someone deciding whether to use it, someone about to operate it, someone about to change it. It answers what the words mean and how the pieces explain each other.
>
> It does not tell you how to install anything ([`getting-started.md`](getting-started.md)), how to run work day to day ([`operating.md`](operating.md)), or why the code has the shape it has ([`../ARCHITECTURE.md`](../ARCHITECTURE.md)).
>
> **This file says which word to use. Which words to avoid has its own single authority** — `xtask/lexicon.toml`, consumed directly by the `lexicon` gate in CI. Read that file to check whether a phrasing is retired; keeping the list in one place is what stops a second authority from forming.

## How the pieces explain each other

A **city** is one directory on one machine, and everything else is inside it. It has one Ledger, which is its complete history, and it never references another city. You can copy the directory to another machine and it is the same city; you can delete it and nothing outside it changes.

Inside a city are **buildings**, and inside a building are floors and rooms. That hierarchy is the directory tree — not a model of it, the tree itself. So an **address** like `lab/room1` names a place on disk, and naming that place settles three separate questions at once: which files an agent may write, which documents it starts with, and who it reports to. A design that separated those three would need rules to keep them agreeing; here they cannot disagree, because they are one fact.

Work happens as a **run**. A run has a start and an end, an address it works at, and a task. It is not a person: a **resident** is the standing identity that survives across runs, with a file of its own that says who it is; a run is the expensive thing that happens when that identity is put to work. Confusing the two is how a design ends up paying for a hundred idle personalities.

Everything a run does becomes an **EventRecord** in the **Ledger** before it becomes anything else — before a file changes, before a model is called, before money is spent. Every other view of the city is a **projection** of that stream: the pages in the browser, the cost report, the recycle bin. Projections are disposable by construction, and the test that keeps them honest is to delete one and rebuild it from the Ledger and require the same bytes.

That is also what makes the interface trustworthy in a specific, narrow way: it can be wrong about what it has not been told, and it cannot be wrong in a way the Ledger does not also record.

Two more relations are worth stating because they are easy to invert. A **Gate** decides one action inside the city, and it answers with an exhaustive verdict rather than a yes or no — so "this needs a person" is a real answer rather than a failure. And a **Discard** is a deletion that carries its own way back: the type has no constructor without one, so a deletion with no restoration is not something the code refuses, it is something the code cannot express.

> The tables below are the vocabulary itself. The order is reading order rather than alphabetical order, because alphabetical order helps nobody on a first pass.

## 1 Space and identity

| Name | What it is |
|---|---|
| **City** | One city on one machine: one Ledger, one complete history. Two cities never reference each other. |
| **Building** | A building within a city. The scope unit for configuration, Archive, and Policy. |
| **Floor** / **Room** | Floors and rooms inside a building. The directory tree is the space. |
| **Session** | One line of work a person named, kept in a room of that name. Dispatching to a building with a name opens the room; dispatching to the room again continues the session, and its `Handoff.md` is what carries it across. |
| **Build Floor** / **Workshop** | A floor given to one piece of collaborative work, and the node graph that routes work across it. |
| **Utilities** | The shared services of a city, in the reserved subtree: nothing a resident writes to. |
| **Resident** | A standing identity with an `URBANITE.md` and a dossier, surviving across runs. |
| **Ephemeral** | A derived worker discarded after use, with no standing identity. |
| **Neighbourhood** | The addresses one run can reach, and who stands at each: this building's rooms in full, the city's other buildings by name only. An empty room is listed as a place, because hiding it would read as "this address does not exist". Never called a directory — that word already names a place on disk. |
| **Run** | One piece of work with a start and an end. **A resident is an identity; an active run is the cost** — the two numbers differ by two orders of magnitude. |
| **Fork** | A new run branched from a point in another run's history. It records a lineage; it does not start driving by itself. |
| **resume** | Reopening a city after the process died: the chain is verified, tool calls whose outcome was lost are closed as unknown, and what waits for a person is reported. |
| **Address** | A path newtype relative to the city root. It sets the write domain, the default context, and who the work reports to. |
| **reserved prefix** | A `.sprawling/` directory and its subtree, at any depth, always outside every write domain. Each scope keeps what governs it there — the city, and from F2.09 each building. An agent cannot edit its own accounting, its own configuration, or its own building's rules. |
| **City Hall** | The building `hall`, raised with the city. It holds the city's plan and the two residents who serve every other building; it holds no project of its own. |
| **Mayor** | The resident `hall/mayor`: the city's planner, writing Markdown only. It turns an idea into `<city>/hall/Roadmap.md`, hands each building its part through `plan`, and raises a building through `city`. Its identity is `<city>/.sprawling/MAYOR.md`. |
| **clerk** | The resident `hall/clerk`: answers approvals when the person delegated them, in the same three parts a Gate uses, with its reason in the Ledger. Its identity is `<city>/.sprawling/CLERK.md`. |
| **governed document** | One of the three files that govern a city rather than belonging to a resident: `MAYOR.md`, `CLERK.md`, `PREFERENCES.md`, all in the city's reserved subtree and all written through one door. The Mayor cannot edit who the Mayor is, the clerk cannot edit what it answers by, and neither can edit how the person wants their city run. |
| **Vocation** | What the residents at an address are there to do: `Builds`, or `Plans`. It is read off the building, and it decides the tool set a run is given — a resident of City Hall gets no `exec`, no `delegate` and no `workshop`, and gets `city` instead. |

## 2 History and content

| Name | What it is |
|---|---|
| **Ledger** | The only history. Every effect becomes an EventRecord first. |
| **EventRecord** | One line of the Ledger, carrying `seq`, `prev`, `t`, `kind`, and a payload. Payloads hold integers. |
| **EventDraft** | An event that has not yet been given a `seq` and a `prev`. Only the Ledger port turns one into an EventRecord. |
| **EventRef** | A reference to an event. **Privately minted**: no public constructor and no serde, so a forged reference cannot be spelled. |
| **Locator** | The retrieval grammar, `cas:` or `file:`. Fail-closed: a shape that does not match is refused rather than guessed. |
| **CAS** | Content-addressed store (BLAKE3). Identical content is stored once for its lifetime. |
| **projection** | A view rebuilt from the event stream. **Disposable**: deleting the table and rebuilding from the Ledger gives byte-identical results. |
| **Snapshot** | The same idea inside the browser (`web::app`): equally disposable, equally forward-only. |
| **Provenance** | The five facts a commit the city makes carries as git trailers — `Sprawling-Run`, `Sprawling-Actor`, `Sprawling-Model`, `Sprawling-Effort`, `Sprawling-City` — and a sixth, `Sprawling-Predecessor`, when the run replaced another. A projection of the Ledger for readers outside the city; the Ledger stays the authority and the commit id reconciles the two. |
| **patch** | The text of one file's change between two checkpoints, asked for one file at a time. It is a separate request from the list of what moved, because patch text is file content on a socket: every line a credential scan matches is withheld and named by its line number, never echoed. |
| **fence** | The commit the city makes before and after a tool wave so a change can be shown and reverted. It lives under `refs/sprawling/runs/`, never on the person's `HEAD`. |
| **landing commit** | The one commit a reviewing run makes on its worktree branch when it offers a pull request; the merge that follows is the only commit trunk receives. |
| **accounting thread** | The city's one writer. It alone holds the Ledger, the endpoint book, the plans, the pursuits, the governance fold and the desks, and it alone settles what a run left behind — in the order the results arrive. |
| **driving pool** | The threads that drive runs. One of them is a **lane**, and it lives exactly as long as the run it drives: it holds that run's driving state and nothing else — no Ledger, no book, no desk. Four lanes, which is the provider's own admission ceiling; a wider pool would only park a lane there. citysim drives no lanes at all, because a scenario reproduces from a seed. |
| **relay** | The Ledger adapter a driving thread writes through. It carries the EventDraft to the accounting thread and waits for the answer, so `Ok` still means durable and the city still has one writer. |

## 3 Context and turns

| Name | What it is |
|---|---|
| **frozen prefix** | The frozen prefix in four segments — city, Building, Resident, Run. Assembling it is itself an event. |
| **Handoff** | The five-section artifact that carries a session across a freeze or a resume. |
| **turn** | The turn state machine: four phases, four cancellation-safe points. An interruption inside a phase cannot be spelled. |
| **Steer** | A redirection: add an instruction to a run **without interrupting it**. It lands at the end of the next tool result. |
| **Cancel** | Stop this run. When Cancel and Steer meet on the same boundary, Cancel wins. |
| **result envelope** | The envelope around a tool result, carrying three attachments: clock stamp, network reminder, and Steer. |
| **ClockStamp** | The clock stamp. With the feature off, output is byte-identical to a build that never had it. |
| **context reminder** | The line that says how full the window is, computed from the provider's reported `input_tokens` against the model's `context_tokens` and never from a byte count. Two thresholds, 25% and 65%; each sounds once per run, and the second says the budget left is still enough to write a handoff and `succeed`. |

## 4 Decisions and safety

| Name | What it is |
|---|---|
| **Gate** | Five doors plus idempotent deduplication. A decision returns an exhaustive verdict rather than a bool. |
| **three-part refusal** | A refusal states what was refused, why, and an alternative that can be acted on. |
| **Taint** | External content is data. Taint joins on the union, rises through doors, and has no unwrapping surface. |
| **WriteDomain** | The set of prefixes a resident may write, and what it may write inside them: `Everything`, or `Documents` — Markdown files only, and never a plan file. The decision primitive is `Address::is_within`; a building declares the second half with one `write:` line in its `BUILDING.md`. |
| **Documents** | The narrower of the two write-domain kinds: inside its prefixes a resident may write Markdown files and nothing else, and never a plan file. A building declares it with one `write:` line in its `BUILDING.md`; the wider kind is `Everything`. It is what a resident who plans is given, so a planner cannot reach into what a builder produces. |
| **Escalate** | Handing a decision up to a person, as an ApprovalItem, rather than deciding it. |
| **undoable effect** | An effect outside this city that nothing inside it can take back — today, a key pressed or a clipboard replaced through the **desktop connector**. It has no Restoration, so the Discard door has nothing to check; the door for it escalates instead of refusing, because refusing would make the tool equivalent to absent. One question per connector, not per tool. |
| **Sealed\<T\>** | A sealed value: no Debug, no Display, no Serialize, no Clone. |
| **SecretRef** | `secret:<realm>/<name>`. Configuration holds the reference; plaintext reaches the Vault only. |
| **Custody** | Credential custody: capture, replace in place with a reference, redeem at the wire. |
| **Discard** | Deletion. A Discard without a Restoration cannot be constructed. |
| **Restoration** | The way back: `Tracked` (committed), `Interred` (in the store), `Rebuildable` (reproducible). |
| **Recycle Bin** | The view over discarded things, where every row can state its own way back. |
| **ApprovalItem** | An item awaiting an answer. Two sources (Gate, agent), carrying a cluster key and a tainted flag. |
| **Policy** | An exemption rule settled from answered ApprovalItems. It expires. |
| **Reading Room** | The list of skills a building admits. A name on it that is not on the shelves is left out rather than promised. |
| **Autonomy** | Who answers: `Owner`, `Delegate`, or `Deferred`. |
| **Halt** | The brake, and the only one: stop a city, a building, or a workshop; `release` lets it go on. It shuts the scope to new work and terminates the backlog members inside it, so a command nobody can reach is not what a stopped city is still doing. Ending a run that is already going is `Cancel`, which is a different verb. There is no spend ceiling and no turn ceiling behind it — a city that must stop is stopped by somebody saying so. |
| **Fallback** | What a tag does when its endpoint will not answer: `None` (freeze, and record why) or `Then` (retreat to a named endpoint and model, which is itself an event). The default is `None`, because switching a person's model in silence is the last decision a default value should make. |

## 5 Tools and the outside

| Name | What it is |
|---|---|
| **exec** | The tool that runs a program, a Python artifact, or a shell line, inside the sandbox the frozen configuration allows. |
| **environment passthrough** | The environment variable names a building declares its `exec` children may inherit, in its `CONFIG.toml` `[sandbox]` section, on top of the four every run gets. A name shaped like a credential is refused where the file is read. It is a declaration rather than a longer built-in list because what a child inherits, it cannot forget. |
| **edit** | The tool that changes a file, against a base version, inside the write domain. |
| **status** | The tool that answers what a run's own situation is: mode, context used against the window it was given, what waits for it, and how many neighbours it has. It states no spend ceiling, because there is none. |
| **neighbours** | The tool that lists the Neighbourhood: this building's addresses with the line each resident's `URBANITE.md` offers about what to bring them, or the city's buildings by name. An address it does not list has no reader. |
| **read** | The tool that opens one file by its path, or one catalog entry — a skill, a mode, the developer entry — by the name the catalog lists it under. A model-chosen path never reaches a reserved subtree; a catalog name may, because a person admitted it. It answers by line interval: at most 512 lines, and a truncated answer states the total and the offset to continue from. |
| **search** | The tool that finds a substring under one address prefix, with the lines around each hit and the line number `read` continues from. A substring, never a regular expression, so a pattern a model wrote wrong cannot become a stall; the same predicate `read` uses keeps it out of a reserved subtree. |
| **dialect** (兼容格式) | The request and reply format one provider speaks — OpenAI-shaped or Anthropic-shaped. `gateway::dialect` translates between the canonical shape and one of them, in both directions. Chinese prose says 兼容格式; the identifier stays English. |
| **Endpoint** | One provider's chat URL, dialect, credential and headers. The city reaches an **external provider** only through one. |
| **Connector** | An external tool server a building configured, reached over MCP. Its tools carry a `Connector` effect, so the egress door knows where they go without a model naming a host. |
| **subscription login** | Signing in to a provider with a subscription instead of an API key: begin, approve in a browser, bring back the code the provider shows. |
| **Image** | A content block a model can see: a locator into the CAS, a media type, and integer dimensions. Bytes reach the wire only when the request is sent, never the Ledger. |
| **browser** | The tool that drives a browser over WebDriver BiDi: open, snapshot, act, screenshot, measure, console, viewport, close. A building gets it by writing `browser: true`; the city starts the engine itself (`bin::browser_bidi`) with that building's own profile, prefers Firefox because Firefox speaks the protocol without a driver, and downloads nothing. |
| **desktop connector** | `sprawling-desktop`, an out-of-tree MCP server a building may attach: windows, snapshot, act, screenshot, record, clipboard, in the browser tool's vocabulary, inside an allowlist the building wrote in `DESKTOP.toml`. Two doors stand in front of it, in this order: `desktop: true` in the building's `BUILDING.md` decides whether it may be attached at all, and the allowlist decides which windows on this machine it may then touch. |
| **doctor** | `sprawling doctor`: what this machine has against what a city needs, in two tiers — enough to use, enough to develop — and, with `--install`, one consented installation at a time. |

| **sieve** | The pass that decides what survives a command's output, by which command produced it. Deterministic and compulsory; so is the tee behind it, so nothing it cuts is unreachable. Distinct from **compaction**, which reads the shape of a text rather than the identity of its author. |
| **backlog** | The table of work that is running while the run goes on: background commands and delegated runs. Halt reaches into it; `status` reports the part of it that belongs to this run. |
| **succession** | A run replacing itself: same address, same depth, therefore the same tools — a successor is not a delegate, which is why it may delegate. It carries a handoff and the address of the conversation that produced it, and it needs no person. |
| **lineage** | The chain of runs a succession leaves behind. `Provenance` names each one's predecessor, so "how many times has this resident replaced itself since I last looked" is one question with one answer. |
| **transcript** | What one run actually saw, written beside its room as `<run-id>.jsonl` when the run freezes: the messages, the calls, the results as the model received them. Not the Ledger — the Ledger is the city's, this is the run's, and only the second one is a thing a resident may read. |

## 6 Interface

| Name | What it is |
|---|---|
| **WebUI** | One page in a desktop browser, embedded in the binary and served from inside it. Two of them exist while v0.0.4 is in flight — `client/`, TypeScript built by bun, and `crates/web`, Dioxus compiled to WebAssembly — and they speak the same wire, which is the point: the wire is the whole API, so the number of clients is a fact about this tree rather than a limit of the design. |
| **Lens** | Which reading of one history a page is showing: `Ledger`, `Archive`, or `Bin`. The three used to be three nav entries, which asked a person to choose before the question was formed. The lens lives in the address, so a link to the archive is still a link to the archive. |
| **control surface** | The intervention surface at the bottom: five verbs plus the steer input. |
| **Approval Inbox** | The queue of pending answers, grouped by cluster key. A tainted item is never grouped. |
| **progress bar** | The progress bar. |
| **ACCENT** | Jing blue, `H=264`, meaning "something is happening here". |
| **ALERT** | Champagne gold, `H=84`, meaning "a person is needed here". |
| **single-hue language** | The single-hue visual language. |

## 7 Construction vocabulary (not product concepts)

| Name | What it is |
|---|---|
| **SPEC** | A crate's construction authority, `crates/<crate>/<crate>-SPEC.md`, written before its code. It ships with the crate it governs, so a reader of the code has the reasons for it. |
| **shape** | Which of the seven module shapes a file instantiates — decision, value, port, adapter, typestate, data, projection. A module that cannot name its shape usually holds two things. |
| **seam** | A trait declared in the inner layer and implemented in the outer one. One adapter is a supposed seam; two make it real. |
| **conformance** | A generic assertion suite run against any implementation of a port. |
| **ablation** | Removing one passage of a document a model reads, and measuring what a resident stops being able to do without it. Run on demand for evidence about the document's length; never a gate. |
| **gate** (lower case) | A machine guard in CI, run by `cargo xtask gates`. **Deliberately a homonym of the product's Gate above, and the boundary is this line**: a Gate decides one action inside the city, a gate decides whether a change may merge. Neither counts the other, and neither is written down as a number here — `xtask` counts its own. |
