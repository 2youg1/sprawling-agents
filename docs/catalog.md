# The catalog — every built-in capability a resident can be given

This guide is for someone who writes a building's `RULES.toml` or `CONFIG.toml` and wants to know what a resident there will be able to do, and for someone reading a transcript who wants to know why a model called one tool and never mentioned another. It names each tool the city builds in, the namespace the catalog files it under, the sentence a model reads about it first, what the tool does, what admits it, and where it appears.

The rules themselves live elsewhere, and this guide points to them rather than restating them: the three tiers a capability can sit in and the shape of the dormant index are `crates/runtime/spec/Catalog.lean` §8-60, a call through `tool` is §8-61 of the same file, which tier an admitted tool takes is `exposure_of` in [`crates/runtime/src/catalog/exposure.rs`](../crates/runtime/src/catalog/exposure.rs), whose default is the mode cores of `core_tools` in [`crates/runtime/src/mode.rs`](../crates/runtime/src/mode.rs), and which tools a building's runs are given is decided in one function, `lay_out_workbench` in [`crates/accounting/src/worker/workbench/tools.rs`](../crates/accounting/src/worker/workbench/tools.rs).

## How a model meets a capability

A tool reaches a model in one of three ways. A tool the building did not admit, or admitted at the `off` tier, costs nothing: it is in no prompt, no tool list and no answer of `tool`. An admitted tool at the `core` tier travels in the request's tool list on every turn, with its whole description and input schema. Every other admitted tool is `dormant` and waits in the **dormant index**, a block of at most 1024 bytes in the resident's part of the prompt that gives each namespace one line, with its member count and a hint, then one line for the skills as a group and one line for each admitted skill; the model reaches those tools through the resident tool `tool`, whose `list`, `find`, `detail` and `run` answer from the catalog. The tool list is fixed when the run starts, so admitting a tool mid-run is not something the city does.

Where no `[catalog]` table in the building's or the city's `CONFIG.toml` names a tool or its namespace, a tool in the core of some mode is `core` and every other tool is `dormant`; `tool` and `read` are `core` whatever a table says. This guide describes that default, and the tables below give each tool's tier under it.

Above the dormant index the catalog writes two rows that are not tools: the row for the run's mode (`mode:chat` or `mode:work`) and the `dev` row, which tells a run that this city's own code can be changed and opens, through `read` or `tool detail`, into the evidence such a change can be asked for. Their text is in [`crates/runtime/src/mode.rs`](../crates/runtime/src/mode.rs).

Each tool below opens with its first sentence in bold, copied from the file named beside it. The first sentence is what the catalog's own hint rule reads: the description up to its first `. `, or to the end of its first line. A model in a run with that tool in its tool list reads the whole description; a model reaching it through the dormant index reads only a hint from `tool list` or `tool find`, which is the same sentence without its final period and cut at 64 bytes (`HINT_MAX_BYTES` in [`crates/runtime/src/catalog/dormant.rs`](../crates/runtime/src/catalog/dormant.rs)), and reads the whole description only after `tool detail`. Where a sentence is longer than 64 bytes, the cut can fall inside a word.

## Where each tool appears

| Namespace | Tool | Appears as, by default | Admitted where |
|---|---|---|---|
| `sprawling` | `read`, `search`, `status`, `tool` | resident, both modes | every building |
| `sprawling` | `edit` | resident; refused in chat mode | every building |
| `sprawling` | `exec` | resident; refused in chat mode | every building except City Hall |
| `sprawling` | `archive`, `goal`, `neighbours`, `plan`, `pr`, `rules`, `signal`, `succeed` | dormant | every building |
| `sprawling` | `delegate`, `workshop` | dormant | every building except City Hall |
| `sprawling` | `city` | dormant | City Hall only |
| `computer` | `browser` | dormant | `browser = true` in `RULES.toml` |
| `computer` | `usersbrowser` | dormant | `usersbrowser` in `RULES.toml` not `false` |
| `desktop` | `desktop_desktop_windows` and five more | dormant | `desktop = true` in `RULES.toml` |
| `media` | `transcribe`, `ocr` | dormant | the endpoint book names a model for the job that this building may use |
| `record` | `playback`, `proposal` | dormant | every building |
| `web` | `web_search` | dormant | every building that is not confidential, unless `[search]` is `off` |
| `skill` | each skill on the reading room | dormant, a group line and one line each | `reading_room` in `RULES.toml` |
| a server's label | each tool the server lists | dormant | an `[[mcp]]` row in the building's `CONFIG.toml` |

City Hall is the building `hall`; its vocation is to plan, so it gets `city` and none of `exec`, `delegate` and `workshop` (`crates/city/spec/Vocation.lean`). A confidential building never holds a browser, the desktop or `web_search`, starts no MCP server, and has `transcribe` and `ocr` only when the endpoint for them is on this machine.

The resident core differs by mode in what it may run, not in what it carries: the tool list of every request is the union of both modes' cores, so changing a run's mode mid-session leaves the list's bytes alone, and a call to `edit` or `exec` in chat mode is refused with `E_GATE_DENIED` before it does anything (`mode_admits` in [`crates/runtime/src/mode.rs`](../crates/runtime/src/mode.rs)).

## Choosing how much of the catalog a run carries

The city's `CONFIG.toml` and a building's may give a tool or a whole namespace one of three exposures under `[catalog.tools]` or `[catalog.namespaces]`: `core` sends the tool in every request, `dormant` lists it in the dormant index for `tool` to fetch on demand, and `off` does not admit it. For example:

```toml
[catalog.tools]
exec = "dormant"
web_search = "off"

[catalog.namespaces]
record = "core"
```

A tool's setting is read in this order: the building's tool key, the building's namespace key, the city's tool key, the city's namespace key, then the built-in default. `tool` and `read` are always `core`, because they are the doors to the index and the documents it names. Only the city and building layers state these tables, so every room of a building keeps the same cacheable tool array; each table is frozen independently, which means a building that states one tool still follows the city for every other. Every run records the table it actually received, `off` rows included, in `prompt_assembled.tools` ([`crates/runtime/src/catalog/exposure.rs`](../crates/runtime/src/catalog/exposure.rs), runtime D111). The building page's capabilities section edits these two layers and shows the newest run's byte counts and recent use beside the choice.

## The resident tools

These travel in the tool list of every request, as far as the building admitted them. They are filed under `sprawling`, so `tool list sprawling` names them beside the dormant members.

### `read`

**Reads a file by its path, or a skill by the name the catalog lists it under, with line numbers counted from 1.**

[`crates/runtime/src/tools/read.rs`](../crates/runtime/src/tools/read.rs). Its arguments are `file_path`, an optional `offset` whose line number counts from 1, and an optional `limit` of at most 512 lines. It answers by numbered lines and says where to continue, and it is also how a run opens a catalog entry — a skill, the mode row or the `dev` row — by name. A path the model chose is held to the **read bound**: all of the run's own building, every building that is not confidential, and never a reserved subtree. A file it returns counts as read for `edit`. Resident in both modes, in every building.

### `search`

**Searches the lines of files under one path for a regular expression in ripgrep's syntax, or with `glob` alone finds files by name.**

[`crates/runtime/src/tools/search.rs`](../crates/runtime/src/tools/search.rs). `pattern` is a regular expression matched one line at a time by the `regex` crate, whose engine does not backtrack; `output_mode` is `files_with_matches` by default, or `content` and `count`, and `-i`, `-A`, `-B`, `-C` and `context` adjust the match and its context. With `glob` and no `pattern`, the call finds files by name instead of searching their lines. A reported line number counts from 1 and is the `offset` `read` continues from. The call refuses `multiline`, `type`, a nonzero `offset` and a pattern that spells a line break, because matching line by line would otherwise answer wrongly; `-n`, `-o` and `head_limit` go unread. Search asks the same read bound as `read`, so what one may open the other may find. Resident in both modes, in every building.

### `status`

**Report your current situation in thirteen lines: who you are, where you are, your mode and run policy, context used against the window, your trust, your write domain and its locks, the worktree and its size, the signals waiting, your children, the clock, whether the provider is serving, your neighbours, and your backlog.**

[`crates/runtime/src/tools/status.rs`](../crates/runtime/src/tools/status.rs). Most of its answer is read when the run is dispatched; the children, the backlog and the context used are read when it is called. It is in both modes because the city prompt sends every resident to it. Resident in every building.

### `tool`

**Reach what this building admits beyond your tool list.**

[`crates/runtime/src/tools/tool.rs`](../crates/runtime/src/tools/tool.rs). Its `op` is one of `list`, `find`, `detail` and `run`. `list` with no namespace prints one line per dormant namespace and skill; with a namespace it prints each member and its hint, or names alone past 4096 bytes. `find` returns at most eight capabilities ranked by where the words were found. `detail` returns a tool's whole description and input schema. `run` calls a tool this run was admitted, after checking the arguments against the top level of its schema; the call then passes that tool's own gates, as if the model had called it directly. Resident in both modes, in every building.

### `edit`

**Performs exact string replacements in a file you have read in this run: `old_string` must match exactly once, or set `replace_all` to change every occurrence.**

[`crates/runtime/src/tools/edit.rs`](../crates/runtime/src/tools/edit.rs). Its arguments are `file_path`, `old_string`, `new_string` and the optional `replace_all`. A file changed since this run read it is refused, and the refusal shows the passage as it reads now; an empty `old_string` creates a file that does not exist. Every write is checked against the building's write domain (`write` and `prefixes` in `RULES.toml`) and against the run's write limit, which under `Create` lets it create a file and change none. Resident in every building, run only in work mode.

### `exec`

[`crates/runtime/src/tools/exec/disclosure.rs`](../crates/runtime/src/tools/exec/disclosure.rs) and [`crates/runtime/src/tools/exec/dispatch.rs`](../crates/runtime/src/tools/exec/dispatch.rs). The description names the sandbox arm the run got. With no arm named, or with `native`, `copied_tree`, `container` or `none`:

**Executes a command, or a Python script.**

With `python`:

**Executes a Python script.**

The call gives exactly one of `command` and `python`, and may ask for `where: host` to run outside the confinement; `where` defaults to `sandbox`, and the Python arm has no host form. With `[sandbox] shell = true`, a command is interpreted by the configured shell. Without it, the default, a command is one program and its arguments: quotes group arguments, and unquoted shell syntax such as pipes, redirection and wildcards is refused rather than passed to a program as arguments. Under the `Create` write limit host execution is refused and a confined command writes only in a copy. A file a command prints or writes does not count as read for `edit`. Which mechanism fills each arm on each platform, and what each one does not guarantee, is in [How `exec` is confined](operating.md#how-exec-is-confined). Resident in every building except City Hall, run only in work mode.

## `sprawling` — working with other residents, and the run itself

**talk to other residents, split and hand off work, see your run**

The namespace's sentence is in `own_about` in [`crates/runtime/src/catalog/namespace.rs`](../crates/runtime/src/catalog/namespace.rs), with the other namespaces' sentences below. The rest of `sprawling` is dormant. In the dormant index, and in what `tool list` prints with no namespace, the whole namespace is one line; for a building that builds it reads:

```text
- sprawling (16): talk to other residents, split and hand off work, see your run
```

The sentence is 62 bytes long, so it fits under the 64-byte cut whole. In City Hall the count is 14: `city` is added, and `exec`, `delegate` and `workshop` are absent. `tool list sprawling` lists every member, the resident ones included, in byte order of their names. For a building that builds, with no sandbox arm named, it prints this; the lines for `archive` and `signal` end in a space where the cut fell, which is left out here:

```text
- archive: Write down what this building should not have to be told twice,
- delegate: Hand one piece of work to another agent in this building, one le
- edit: Performs exact string replacements in a file you have read in th
- exec: Executes a command, or a Python script
- goal: Claim the ground you are about to work on, so two residents do n
- neighbours: Who else this building has, and what each of them is for; call i
- plan: Take a node of this building's plan before unassigned work
- pr: Offer your work for review, or check someone else's; nothing you
- read: Reads a file by its path, or a skill by the name the catalog lis
- rules: What this building's runs are judged by
- search: Searches the lines of files under one path for a regular express
- signal: Speak to another resident, or to every one of them, or take the
- status: Report your current situation in thirteen lines: who you are, wh
- succeed: Replace yourself with a successor at this address when the windo
- tool: Reach what this building admits beyond your tool list
- workshop: Split one creation into nodes that run in dependency order, each
```

What each of these tools may do is decided in its own desk; `crates/collab/Spec.lean` §8 holds the rules for `archive`, `delegate`, `goal`, `plan`, `pr`, `signal` and `workshop`.

### `neighbours`

**Who else this building has, and what each of them is for; call it before signalling or delegating to an address you have not been given.**

[`crates/city/src/neighbours_tool.rs`](../crates/city/src/neighbours_tool.rs). It lists the Neighbourhood: this building's rooms with the line each resident's `URBANITE.md` offers, and the city's other buildings by name only. The list is read when the run is dispatched. Dormant, every building.

### `signal`

**Speak to another resident, or to every one of them, or take the signals waiting for you; call it when `status` says signals are pending.**

[`crates/collab/src/signal_tool.rs`](../crates/collab/src/signal_tool.rs). `send` speaks to an address in this building as a mention, a thread, a broadcast or a steer, and `wait: true` stops the sending run until the reply arrives; `pull` takes what waits in the run's own queue. A send to another building is refused with `E_CROSS_BUILDING_DENIED`. The words arrive in the other run's window as a letter, never as the User's. Dormant, every building.

### `delegate`

**Hand one piece of work to another agent in this building, one level down.**

[`crates/collab/src/delegate_tool.rs`](../crates/collab/src/delegate_tool.rs). It names the room the work happens in, the task and what counts as done, and the child starts at once. The spawn gate holds it to one level: a delegated child cannot delegate again. The room must be inside the caller's building. The child reports back with a handback when it ends. Dormant, every building except City Hall.

### `workshop`

**Split one creation into nodes that run in dependency order, each in its own room, and judge what comes back.**

[`crates/collab/src/workshop_tool.rs`](../crates/collab/src/workshop_tool.rs). `lay_out` hands out the nodes whose dependencies are done and the rest follow as handbacks arrive; `question` and `judge` settle what the join asks. Every node passes the same gates as a `delegate` call, so a node cannot hand work further down. One graph per run. Dormant, every building except City Hall.

### `plan`

**Take a node of this building's plan before unassigned work.**

[`crates/collab/src/claim_tool/tool.rs`](../crates/collab/src/claim_tool/tool.rs). `Roadmap.md` is the plan, and this tool is its one editing entrance: `claim` takes a ready node, and `finish` (with evidence), `block`, `release` and `split` act only on the node this run holds. `add` puts top-level parts into an empty plan and is the Mayor's alone. Dormant, every building.

### `goal`

**Claim the ground you are about to work on, so two residents do not edit the same thing; refused if someone holds it already.**

[`crates/collab/src/goal_tool.rs`](../crates/collab/src/goal_tool.rs). A claim names paths and outside resources; one that overlaps a goal another run holds is refused with `E_GOAL_CONFLICT`, and the refusal says which level settles the clash. Dormant, every building.

### `pr`

**Offer your work for review, or check someone else's; nothing you wrote reaches the building until another resident has checked it.**

[`crates/collab/src/pr_tool.rs`](../crates/collab/src/pr_tool.rs). `open`, `list` and `check`. Only a building with `review = true` in `RULES.toml` gives a run its own tree, and in any other building `open` is refused with `E_TOOL_UNAVAILABLE`. A resident cannot check its own work. Dormant, every building.

### `archive`

**Write down what this building should not have to be told twice, or read what it already knows.**

[`crates/collab/src/archive_tool.rs`](../crates/collab/src/archive_tool.rs). `record` writes one sentence of one of four kinds — preference, decision, correction, fact — and `recall` reads them back. Dormant, every building.

### `rules`

**What this building's runs are judged by.**

[`crates/city/src/rules_tool.rs`](../crates/city/src/rules_tool.rs). Its one `op`, `read`, shows the building's `RULES.toml` as it stands. No run may change the rules: the tool offers no operation that would, and a call that asks for one meets the effect layer's refusal of the `Govern` effect, which names the file the User edits (`crates/city/spec/RulesTool.lean` §8-2b). Dormant, every building.

### `succeed`

**Replace yourself with a successor at this address when the window fills: same tools, same depth, without asking the User.**

[`crates/runtime/src/tools/succeed.rs`](../crates/runtime/src/tools/succeed.rs). The successor is the same resident in the same room, so unlike a delegated child it may still delegate. It starts from the `Handoff.md` the run writes first and from the run's transcript. Dormant, every building.

### `city`

**This city's buildings: list them, raise a new one, or adopt a directory that is already here.**

[`crates/city/src/city_tool.rs`](../crates/city/src/city_tool.rs). A run may list. Raising and adopting are refused in a run, because the shape of the city is the User's decision. Dormant, City Hall only.

## `computer` — browsers

**drive a browser: the building's own, or the person's**

A building's rules give its residents a browser; `city::policy` refuses either setting on a confidential building. The index line, when one of the two is admitted:

```text
- computer (1): drive a browser: the building's own, or the person's
```

and `tool list computer`, with both admitted:

```text
- browser: drive this machine's browser: open a page, look at it, act on it
- usersbrowser: drive the browser the User is already using, with their own logi
```

### `browser`

**drive this machine's browser: open a page, look at it, act on it, take a screenshot, measure boxes, fetch an address as text, read the console, resize, close.**

[`crates/sprawling/src/browser_tool/building.rs`](../crates/sprawling/src/browser_tool/building.rs). The city starts the browser engine itself, with this building's own profile, on the first call. Every page it opens leaves the machine, so each call passes the egress door, and the building's `egress` list in `RULES.toml`, when it has one, limits the domains. Dormant, in a building with `browser = true`.

### `usersbrowser`

**drive the browser the User is already using, with their own logins.**

[`crates/sprawling/src/browser_tool/person.rs`](../crates/sprawling/src/browser_tool/person.rs). The User starts that browser with remote debugging, and `usersbrowser` in `RULES.toml` is either `true` or the `ws://` address it answers on. Each attach passes the attach door, the one door that asks the User rather than deciding alone; with `true` and no address, every attach asks for the address. Dormant, in a building whose `usersbrowser` is not `false`.

## `desktop` — this machine's desktop

The namespace has no sentence of its own, because the city files it as a server's. The desktop is reached through an MCP server this binary carries. For a building whose `RULES.toml` says `desktop = true`, the city starts its own executable as `sprawling desktop`, scoped by the building's `.sprawling/DESKTOP.toml`, and admits the six tools it lists like any other server's: under the namespace of the server's label, `desktop`, each named `desktop_` followed by the server's own name with its dot turned into `_` (`crates/sprawling/spec/Main.lean` §8-4d). A building that names its own `desktop` server in `[[mcp]]` keeps that one, and the built-in one is not started. Every call is refused with `E_TOOL_UNAVAILABLE` outside Windows.

Two more things decide each call. The scope file decides which windows the tools may see, and whether recording and the clipboard are switched on at all. The gate for effects nothing in the city can undo refuses every call unless `CONFIG.toml` lists `desktop` in `[sandbox] trusted`, and refuses a call that carries content from outside the city whatever that list says (`crates/kernel/spec/Gate.lean` §8-49).

A server's index line lists its members without the label's prefix, so the desktop's line is cut inside its fourth name:

```text
- desktop (6): desktop_act, desktop_clipboard, desktop_record, desktop_screensh
```

`tool list desktop` prints the six members; the lines for `act` and `record` end in a space where the cut fell, which is left out here:

```text
- desktop_desktop_act: Do one thing to one window: click, double, right, drag, scroll,
- desktop_desktop_clipboard: Read this machine's clipboard as text, or replace it with text
- desktop_desktop_record: Start or stop recording one named window: an mp4 when ffmpeg is
- desktop_desktop_screenshot: Capture one named window, or a region of it, and return the imag
- desktop_desktop_snapshot: Read one named window as an outline: one line per element, its r
- desktop_desktop_windows: List the top-level windows on this desktop: title, owning proces
```

Each description below is in [`crates/desktop/src/tools.rs`](../crates/desktop/src/tools.rs), and each ends with what the tool does not do.

### `desktop_desktop_windows`

**List the top-level windows on this desktop: title, owning process, bounds, and a ref the other tools take.**

Only windows the scope file lists are reported. It does not focus, move, resize or close a window, and does not read what is inside one.

### `desktop_desktop_snapshot`

**Read one named window as an outline: one line per element, its ref, role and name, indented by depth and in the order the window lists them, then a line with the window's title, process and the generation number `desktop.act` carries back, which stops being accepted once the window is snapshotted again, moves or changes size.**

No pixels, no positions and no native handles.

### `desktop_desktop_act`

**Do one thing to one window: click, double, right, drag, scroll, type or key, at a ref from a snapshot or at a point.**

An action decided against an older snapshot, or before the window moved or changed size, is refused, and nothing is sent unless the named window holds the keyboard and lies under the point.

### `desktop_desktop_screenshot`

**Capture one named window, or a region of it, and return the image as an image block, with its width and height in a text block beside it.**

It writes no file and never captures the whole screen.

### `desktop_desktop_record`

**Start or stop recording one named window: an mp4 when ffmpeg is on this machine's PATH, otherwise a directory of PNG frames.**

The scope file has to switch recording on, and a recording ends itself after ten minutes.

### `desktop_desktop_clipboard`

**Read this machine's clipboard as text, or replace it with text.**

The scope file has to switch the clipboard on, and nothing it replaces is kept.

## `media` — pictures and recordings

**read the text in a picture or a recording**

Both tools send bytes the run may read — a file under the read bound, or a `cas:` locator — to an endpoint, so each is offered only when the endpoint book chooses a model for the job that this building's policy allows; a confidential building has them only when that endpoint is on this machine (`crates/sprawling/spec/Accounting/Worker.lean` §8-131, §8-142).

### `transcribe`

**Turn a recording of speech into text - a file you may read, or a recording by its cas: locator - through the transcription endpoint this city was given.**

[`crates/accounting/src/worker/workbench/tools/transcribe.rs`](../crates/accounting/src/worker/workbench/tools/transcribe.rs). Dormant, where the book names a transcription endpoint.

### `ocr`

**Read the text in a PNG picture - a file you may read, or a screenshot by its cas: locator - through the model this city chose for OCR.**

[`crates/accounting/src/worker/workbench/tools/ocr.rs`](../crates/accounting/src/worker/workbench/tools/ocr.rs). Dormant, where the book names an OCR model.

## `record` — this building's history

**look back at what this building did, and propose changes**

Both tools are admitted in every building.

### `playback`

**Look back at this city's history as your building may read it.**

[`crates/accounting/src/worker/workbench/tools/playback.rs`](../crates/accounting/src/worker/workbench/tools/playback.rs). `export` writes a playback bundle, or a playback page, into the building's playback exports and never over an earlier one; `check` checks one of them against the city (`crates/sprawling/spec/Main.lean` §8-132). Dormant, every building.

### `proposal`

**Suggest a change to a document for the User to decide.**

[`crates/accounting/src/worker/workbench/tools/proposal.rs`](../crates/accounting/src/worker/workbench/tools/proposal.rs). `offer` quotes the city's copy of the document and suggests new text; the User sees a card and decides it sentence by sentence, and the document does not change until they do. `withdraw` takes back a card still open. Dormant, every building.

## `web` — the web

**search the web through the city's own supplier**

The namespace holds one tool.

### `web_search`

[`crates/accounting/src/worker/workbench/tools/search.rs`](../crates/accounting/src/worker/workbench/tools/search.rs) writes the sentence with the configured supplier's id in it. With no `[search]` setting the supplier is `exa`, and the sentence reads:

**Search the web through exa.**

It sends the query to that one supplier over MCP and returns the answer as the service gave it; a supplier that fails is not replaced by another. A confidential building is never offered it, and `[search] choice = "off"` removes it (`crates/accounting/spec/Connectors.lean` §8-35). Dormant, every building that is not confidential.

## Skills and servers

**the steps this building follows for a kind of work**

That is the sentence the catalog keeps for `skill`. When the building admits at least one skill, the dormant index writes it on a group line after every namespace line and before the skills themselves, with the number of skills admitted:

```text
- skill (2): the steps this building follows for a kind of work
```

Each skill the building's `reading_room` admits is then one line of the dormant index, `- skill <name>: <hint>`, whose hint is cut from the first non-empty line of its `SKILL.md`; `tool list skill` lists them, and `read` opens one by name. A name on the list that no shelf holds is left out.

Every other namespace belongs to an MCP server and is named by its label. The index line for a server lists its members' names without the `<label>_` prefix, because a server writes no sentence of its own there, and each tool's description is whatever the server's `tools/list` said. A server label may not be one of the six namespaces the city keeps, so a server's tool is never listed as the city's own. How a server is configured, started and called is in [Connect an MCP tool server](integrations.md#connect-an-mcp-tool-server).
