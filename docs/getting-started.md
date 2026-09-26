# Getting started — from one install command to a merge on your trunk

> **For someone installing this for the first time.** It walks one closed loop and skips no step: install the binary, raise a city, register a provider, hand the Mayor an idea, watch the buildings work, read the diff, and read the merge that landed on your branch.
>
> It does not explain the vocabulary ([`glossary.md`](glossary.md)), day-to-day operation ([`operating.md`](operating.md)), or the design ([`../ARCHITECTURE.md`](../ARCHITECTURE.md)). 中文版：[`getting-started.zh-CN.md`](getting-started.zh-CN.md).

## What you need

A desktop browser, and one model to call: an API key for a provider that speaks the OpenAI or the Anthropic dialect, an Anthropic subscription you can sign in to, or a local server that speaks the OpenAI dialect.

Nothing else. No npm, no node, no language runtime, no database.

## 1 Install, in one line

macOS or Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/2youg1/sprawling/main/install.sh | sh
```

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/2youg1/sprawling/main/install.ps1 | iex
```

The script asks the release list for the newest archive built for your platform, downloads it, and checks it against the sha256 the release publishes; an archive that does not match is not unpacked and nothing is installed. It prints the release tag, the platform it matched and the archive's size before the download, and the version the unpacked binary reports about itself after the check, so you can compare that against the tag you asked for. A platform the release does not build is reported with the list of archives the release does carry, rather than guessed at.

Where the binary finally lives, and what happens to `PATH`, is decided by `sprawling install`, which the script runs last and which you can run again later; `sprawling install --uninstall` takes it back off `PATH`. Set `SPRAWLING_VERSION` to a release tag to install that release rather than the newest one.

Neither runtime is needed, but if you already have [bun](https://bun.sh) or node, one command fetches the same binary:

```sh
bunx sprawling help      # or: npx sprawling help
```

The npm packages carry the archives' own binaries, so what arrives is byte for byte what a download gives you. One platform package is fetched and the rest are skipped, and nothing is written outside the package directory: `PATH` stays as it was until you run `sprawling install` yourself.

Neither channel updates itself, and neither asks about a newer release until you do. `sprawling version` prints which release this binary is and the day it was cut, and it reads only what was compiled in. `sprawling status --check` is the one command here that reaches the internet: it asks npm's `latest` dist-tag what the newest release is. The settings page carries the same check under **which release this is**, behind a **check npm** button that asks when you press it and at no other time. Both report and stop. The path of an installed binary belongs to whoever installed it, so replacing it is `bunx sprawling@latest`, or a download followed by `sprawling install`, and it stays your command to run.

The binaries are not code-signed, so a first run trips a warning. Windows says "Windows protected your PC", and the way through is *More info*, then *Run anyway*; macOS refuses the first launch, so open the binary once from Finder's right-click menu.

Do not `cargo install` this. The client is built by bun before the binary and embedded into it, and a plain cargo build produces a binary whose page is blank. To build it yourself, read [`CONTRIBUTING.md`](CONTRIBUTING.md) and use `just dist`.

## 2 Raise a city, and open it

```bash
sprawling up ~/cities/first
```

That raises the city if the directory does not hold one yet, serves it on `127.0.0.1:8787`, and opens the page in the browser your operating system opens links with. The terminal you ran it in becomes the city's console: `/help` lists what it takes, `/serving` repeats where the city listens, and `Ctrl-C` stops the city. `sprawling` with no arguments at all asks one question first — it shows the folder it would start a city in, and Enter starts it there.

A city is one directory and everything is inside it: the genesis record, which also keeps the city's name, and the Ledger, under `<city>/.sprawling/ledger`, which is the city's whole history.

Every city is raised with one building already standing: `hall`, City Hall, which holds no project of its own. Two residents live there.

- `hall/mayor` — the Mayor, the city's planner. It writes Markdown and nothing else: it has no `exec`, no `delegate` and no `workshop`, because a planner that can run code stops reading the buildings' evidence and starts producing its own.
- `hall/clerk` — the clerk, which answers the questions you delegated to it, in the same three parts a refusal uses, with its reason written into the Ledger.

Who each of them is lives in `<city>/.sprawling/MAYOR.md` and `<city>/.sprawling/CLERK.md`. That subtree is outside every write domain, so the Mayor cannot edit who the Mayor is. Both files are laid down from the templates in [`templates/`](templates/) and are never overwritten, so what you write there stays. The tool list and the prohibitions of each seat are compiled into the city and appended after the file, so a persona you write cannot remove them. **Edit the two files in your own editor: no page edits them.**

To reach the city from another machine on your network, give it an address to bind:

```bash
sprawling serve ~/cities/first 0.0.0.0:8787
```

An address that reaches past this machine needs a key. If `SPRAWLING_PAIRING_TOKEN` is set, the city adopts that value and never prints it. If it is not, the city mints a key for this serve alone and prints it once in the banner, together with an address that carries it; nothing writes it down, and the next start replaces it.

## 3 Register a provider

A city with no model to call opens on **welcome**, whose first card, **connect a provider**, leads to **settings** → **accounts and providers**. The same group is always one click away under **settings** in the rail.

**with a key** is a form of three boxes:

| Field | What it wants |
|---|---|
| **base_url** | the base URL from the provider's own documentation. With no scheme it is read as `https://`, or as `http://` for an address on this machine, and a path you leave out is filled from the city's table of known hosts; the form shows the URL it will actually call |
| **wire_api** | which face the endpoint answers in: `chat` for the OpenAI shape, `messages` for the Anthropic shape. `responses` is listed and refused, because this city does not call that face |
| **key** | the provider's key, or empty for a local server |

The id and the display name are derived from the URL and wait under **advanced**, with the extra headers and the request-body overrides. **list models** asks the endpoint what it serves and shows each model it found, and **attach** registers the endpoint with the models you ticked. Beside the form, the page shows the `config.toml` this would write and the request it would send.

The key never becomes part of a command or of a frame. It goes to your operating system's credential service by its own route, and what comes back to the page is a reference of the form `secret:realm/name`. From then on that reference is what appears in configuration, in events and in logs.

If you pay Anthropic by subscription rather than by token, choose **with a subscription** instead. **start the login** gives you an approval page to open; you approve in your browser, paste back the code the provider shows, and **finish the login** puts the token in the credential service with its endpoint behind it. The token is renewed before it expires rather than after a call comes back refused.

Last, choose who does what. The table of models under the form has a role column, so a model can take its role as it is attached, and **which model thinks** holds one box per role afterwards:

- **main · thinks** is the model that thinks. With no model chosen for it, a dispatch is refused with `E_MODEL_UNCHOSEN`.
- **digest · reads** reads long documents on `main`'s behalf, and follows `main` until you point it elsewhere.
- **transcribe · hears** turns a recording into text. It needs a speech-recognition model, and a city with none draws no microphone.

One model may hold several roles. The table has a column for the context window and one for the output ceiling, and you need not fill either: an empty ceiling is filled by the city, from the figure the provider stated, then from the city's own table of known models, then from its policy default.

A local server can also be named before the city starts. With `SPRAWLING_MODEL_URL` and `SPRAWLING_MODEL` set in the environment of `sprawling serve`, a city that has nothing registered attaches that server as an OpenAI-dialect endpoint with no key and chooses the named model for `main`. It writes the same two records the settings page writes.

## 4 Tell the Mayor an idea

**the Mayor**, first in the rail, is the conversation with `hall/mayor`, and it is the page the city opens on once a model is chosen. The box at the bottom takes a message: Enter sends it, Shift+Enter starts a new line.

Under the box, three pills say where the message goes and how it runs: the **model** that answers, the **workspace** — the room that hears it — and the **effort**, how hard the model thinks before it answers. Click a pill to change it. A session keeps the model it started on, so choosing another model opens a new session.

1. Write the idea. One sentence is enough: *rewrite the ledger reader so a cold read of a million events stays under a second, and write down the numbers you measured.*
2. Leave the workspace at `hall/mayor`.
3. Press Enter.

A line that begins with `/` is a command rather than a message, and the menu over the box lists them: `/steer` speaks into the run that is going, `/stop` cancels it, `/new` starts a fresh session in the same room, `/raise <addr>` raises a building from a template, and `/help` lists the rest.

The box never asks for a budget. Nobody can price a piece of work before it runs, a subscription has no unit price at all, and what one run cost is reported afterwards from the record.

What the Mayor does with the idea is compiled into the city beside `MAYOR.md`:

- It reads before it writes: every building's `Roadmap.md`, `Memo.md` and `Handoff.md`, and the hall's own.
- It writes the city's plan into `<city>/hall/Roadmap.md` through its `plan` tool, one row per line of work, weighted by what each row is worth to you.
- It raises a building through its `city` tool when no existing building should hold the work, and adopts a directory you point it at. It never raises two buildings for one project.
- It hands each building its part through `plan`, and `pursue` keeps that building working until the part runs out.
- It records what it decided, and why, in `<city>/hall/Memo.md` before it reports back.

## 5 Watch the city work

The rail on the left holds the rest of the city: **city**, **the record**, **cost** and **settings**, and a count of the items waiting on you when there are any.

**city** draws one block per building: a lit window is a run at work, a lamp by the door is something stuck, a flag on the roof is a standing goal, and the plinth is how much of the plan is done. Pick a building to see its progress, what is ready, what is stuck and which runs worked there, then **open the building**.

A building's page shows **the plan**, drawn from that building's `Roadmap.md` and holding no state of its own. A row reads **not started**, **ready**, **working**, **stuck**, **awaiting approval** or **done**, and only leaves count towards the figure, because a branch's work is its children and counting both would count the same effort twice. The same page lists its **rooms**, its **files**, its **commits**, its **changes** and its **skills**. The box at the top gives the building a standing goal: **set as standing goal** keeps it handing out ready work by itself, and it stops when nothing is ready and nothing is in flight.

A run has its own page, reached from the conversation or from the city. Its lenses are **time** (where the time went, turn by turn), **turns**, **monitor** (the terminal and the files the run is working on, followed live), **prompt** (exactly what went to the model), **context**, **changes** and **evidence**.

**waiting on you** is every design question a resident asked that nothing can move without, grouped so that identical questions are one card; **allow** and **deny** answer it. Settings → **run** → **approvals** says who answers: you, or the clerk. A question that began with content from outside the city is marked **outside content** and is never grouped, so whoever answers it reads it knowing where it came from.

**the record** is the one history read through four lenses — **the ledger**, **the archive**, **the recycle bin** and **the log** — and **cost** is what was spent, cut by run, by resident, by segment, by skill and by tool, each cut summing to the same total. Where a provider reported no price, the page says how many calls that was and how many tokens they used, instead of printing `$0.00`.

## 6 Read the diff, and the merge that landed

Work that is meant to land goes through a pull request inside the city, and the rule that matters is not a rule anybody has to remember: **the resident who wrote the work cannot verify it.** A request that has not been verified has no method that merges it, so an unreviewed merge cannot be written in the code at all.

What you read, in order:

1. The run's **changes** lens: one row per file that moved, in path order, with the patch of one file when you open that row. A line that matched the credential scan is reported by its line number and its reason rather than echoed, because printing the bytes to prove a leak is the leak.
2. `git log` in the building. The fences the city writes before each wave of tool calls are commits nobody's `HEAD` points at, kept under `refs/sprawling/runs/`, so your own history keeps the shape you left it.
3. The merge commit's trailers, which `git interpret-trailers --parse` reads with no help from this city: `Sprawling-Run`, `Sprawling-Actor`, `Sprawling-Model`, `Sprawling-Effort` and `Sprawling-City`, plus `Sprawling-Predecessor` for a run that replaced another. A merge you reviewed also carries `Reviewed-by`, and only when this repository's git config holds `user.name` and `user.email`: the city does not invent a person's name.
4. The same question backwards: `sprawling whose ~/cities/first <commit>` answers which run wrote a commit, out of the Ledger rather than out of git, so a city exported to another machine with no `.git` beside it still answers. It wants the full commit id, not an abbreviation. Exit code 1 means this city has no record of writing that commit, which is a different answer from "nothing changed".

**No page merges for you, and none rejects work already merged.** Your recourse is git, which holds every commit the city made, and **the recycle bin**, where every discarded file states the way back and a file git still holds has a **put back** button. Your brake is `/halt --all`, on the **city** page or typed into any box, which stops every building until `/release --all`.

## 7 Stop, and start again

`Ctrl-C` in the console stops the city. `/halt --all` stops the work and leaves the city serving; the halt is itself recorded.

After a crash:

```bash
sprawling resume ~/cities/first
```

That verifies the chain, closes the tool calls whose outcome was lost when the process died, and reports what is waiting for a person. A city that was killed mid-call comes back saying what it does not know rather than pretending. `sprawling up --supervise` or `sprawling serve --supervise` does this for you: the city is served in a child process, and after a crash it is resumed and served again, until crashes come too close together, when the supervisor stops and waits for Enter.

## If something does not work

| What you see | What it means |
|---|---|
| The page says it speaks one wire version and the server another | The binary and the page in your browser were built apart. Reload. |
| The model list is empty after **list models** | The provider answered, but with nothing this build could read. The report under the form says what came back and at which step: name, connection or answer. |
| A dispatch is refused before anything runs | No model is chosen for `main`: `E_MODEL_UNCHOSEN`. Give a model that role in the model table. |
| A run stopped and asks for something | It is on **waiting on you**. |
| A row on **the plan** says **stuck** | Something it needs has not finished, or a check failed. The row names what it waits for. |
| A merge was refused | The trunk moved after that work branched. The building rebuilds the work on the trunk as it stands and has it verified again. |
| A file is missing | **the recycle bin**. Every row states how to get that file back. |

More of these, with what to do about each, are in [`operating.md`](operating.md).
