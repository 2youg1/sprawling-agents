# Getting started — from one install command to a merge on your trunk

> **For someone installing this for the first time.** It walks one closed loop and skips no step: install the binary, raise a city, register a provider, hand the Mayor an idea, watch the buildings work, read the diff, and read the merge that landed on your branch.
>
> It does not explain the vocabulary ([`glossary.md`](glossary.md)), day-to-day operation ([`operating.md`](operating.md)), or the design ([`../ARCHITECTURE.md`](../ARCHITECTURE.md)). 中文版：[`getting-started.zh-CN.md`](getting-started.zh-CN.md).

## What you need

A desktop browser, and one model to call — an API key for a provider that speaks the OpenAI or the Anthropic dialect, a subscription you can sign in to, or a local server that speaks one of those two dialects.

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

The script fetches the newest release archive for your platform and unpacks it; where the binary finally lives, and what happens to `PATH`, is decided by `sprawling install`, which the script runs at the end and which you can run again later. A platform the release workflow does not build is reported with the list of platforms it does build, rather than guessed at.

While it runs it tells you four things: the release tag, the platform it matched, the download as it arrives — megabytes and percent, on one line that rewrites itself — and the result of checking the archive against the sha256 the release publishes. An archive that does not match is not unpacked and nothing is installed. Last, before it hands over to `sprawling install`, it prints the version the binary itself reports, which is what you can compare against the tag you asked for.

Set `SPRAWLING_VERSION` to a release tag to install that release rather than the newest one.

Neither runtime is needed to run this, but if you already have [bun](https://bun.sh) or node, one command fetches the same binary:

```sh
bunx sprawling help      # or: npx sprawling help
```

The npm packages carry the archives' own binaries, so what arrives is byte for byte what a download gives you. One platform package is fetched and the rest are skipped, and nothing is written outside the package directory: `PATH` stays as it was until you run `sprawling install` yourself.

Neither channel updates itself, and neither asks about a newer release until you do. `sprawling version` prints which release this binary is and the day it was cut, which is the reading you can take offline; `sprawling status --check` is the one command here that reaches the internet, and it asks npm's `latest` dist-tag what the newest release is. The **machine** page carries the same check behind a button, asked when you press it and at no other time. Both report and stop: an installed binary's path belongs to whoever installed it, so replacing it is `bunx sprawling@latest` or a download plus `sprawling install`, and it stays your command to run.

The binaries are not code-signed, so a first run trips a warning: Windows says "Windows protected your PC", where the way through is *More info*, then *Run anyway*; macOS refuses the first launch, so open the binary once from Finder's right-click menu.

Do not `cargo install` this. The client is built by [bun](https://bun.sh) before the binary and embedded into it, and a plain cargo build produces a binary whose page is blank. To build it yourself, read [`CONTRIBUTING.md`](CONTRIBUTING.md) and use `just dist`.

## 2 Raise a city, and open it

```bash
sprawling up ~/cities/first
```

That raises the city if the directory does not hold one yet, serves it on `127.0.0.1:8787`, and opens the page. A city is one directory and everything is inside it: the genesis record, which is also where the city's name is kept, and the Ledger, which is its whole history.

Every city is raised with one building already standing: `hall`, City Hall, which holds no project of its own. Two residents live there.

- `hall/mayor` — the Mayor, the city's planner. It writes Markdown and nothing else: no `exec`, no `delegate`, no `workshop`, because a planner that can run code stops reading the buildings' evidence and starts producing its own.
- `hall/clerk` — the clerk, which answers approvals you delegated, in the same three parts a Gate uses, with its reason written into the Ledger.

Who each of them is lives in `<city>/.sprawling/MAYOR.md` and `<city>/.sprawling/CLERK.md`. That subtree is outside every write domain, so the Mayor cannot edit who the Mayor is. Both files are laid down from the templates in [`templates/`](templates/) and neither is ever overwritten, so what you write there stays. **Edit them in your own editor: this build has no screen for either file.**

To reach the city from another machine on your network, give an address to bind and set a pairing token first:

```bash
SPRAWLING_PAIRING_TOKEN=<a token you choose> sprawling serve ~/cities/first 0.0.0.0:8787
```

Binding a non-loopback address with no token refuses to start. That is a binding decision rather than a request-time one: a city that starts and then rejects everyone looks like a network fault, and a city that starts and accepts everyone is worse.

## 3 Register a provider

Open **settings** in the left nav, and fill in **Attach a provider**.

| Field | What it wants |
|---|---|
| **call it** | the name you will recognise this provider by later |
| **base URL** | the base URL from the provider's own documentation. `https` anywhere; `http` only to this machine |
| **which wire does it speak** | OpenAI-shaped or Anthropic-shaped |
| **key** | the provider's key, or empty for a local server |

The key never becomes part of a command or of a frame. **put the key in the vault** sends it to the credential service of your operating system by its own route, and what comes back to the page is a reference of the form `secret:realm/name`. From then on that reference is what appears in configuration, in events, and in logs. Then press **attach this provider**, and the city asks the provider which models it serves.

If you pay by subscription rather than by token, use **Sign in with a subscription** on the same page instead: **start the login** gives you a URL to open, you approve it in your browser, you paste back the code the provider shows, and **finish the login** puts the token in the credential service with its endpoint behind it, renewed before it expires rather than after a call comes back refused. Only the Anthropic row of that table is complete today; a provider whose row is empty refuses to begin rather than sending you to an empty page.

Last, under **choose a model for a job**, point each job at a model with **point this job at that model**. Two jobs matter at the start:

- `main` — the model that thinks. Without it a dispatch is refused.
- `digest` — the model that reads long documents on `main`'s behalf.

They may be the same model. The context window and the output limit are not asked for: those are facts about the model, which the city reads from the list it just fetched.

## 4 Tell the Mayor an idea

Go to **what's happening**. The box at the top takes one sentence, and the grey line under it says where that sentence goes and how it will run — a dotted word is one the city guessed, a solid one you set yourself, and clicking any word changes it.

1. Write the idea. One sentence is enough: *rewrite the ledger reader so a cold read of a million events stays under a second, and write down the numbers you measured.*
2. Click the room word and type `hall/mayor`. The part before the slash is the building, the part after it is the session, so this opens a session called `mayor` in City Hall. The list that drops down offers the buildings this city already has; a room it has never seen is the one field you may type into freely.
3. Leave the mode word at `build`, which is the mode that plans first and then works.
4. Press **send it**, or Enter.

The box never asks for a budget. Nobody can price a piece of work before it runs, a subscription has no unit price at all, and what one run cost is reported afterwards from the record. What stops a run that should not go on is cancelling it; what stops everything is **stop the city**.

What the Mayor does with the idea:

- It reads before it writes — every building's `Roadmap.md`, `Memo.md` and `Handoff.md`, and `SPEC.md` where a building has one.
- It writes the city's plan into `<city>/hall/Roadmap.md` through its `plan` tool, one row per line of work, weighted by what each row is worth to you.
- It raises a building where no existing building should hold the work, and adopts a directory you point it at. It never raises two buildings for one project.
- It hands each building its part, then leaves that building pursuing it.
- It records what it decided, and why, in `<city>/hall/Memo.md` before it reports back.

## 5 Watch the city work

**what's happening** lists the sessions that are running and the ones that stopped waiting for you; what has ended is in the section below them. Open one and you get that session's own page, in five tabs: **turns** is what happened, **changes** is what moved on disk, **spend** is what it cost, **documents** opens the building it works in, and **prompt** is exactly what went to the model.

That building page is also reached from the city drawing. It shows **the plan**, drawn from that building's `Roadmap.md` and holding no state of its own: a row is **ready**, **waiting**, **working**, **stuck** or **done**, and only leaves count towards the figure, because a branch's work is its children and counting both would count the same effort twice. The **standing goal** panel is the other half: **pursue this** keeps the building handing out ready work by itself, and it stops when nothing is ready and nothing is in flight.

**waiting on you** is everything that cannot move until somebody answers. A new city is raised with approvals delegated to the clerk, so most items are answered there and their reasons land in the Ledger; what a policy marks as yours, and anything carrying content from outside the city, stays for you.

**the record** is the one history in three lenses — **the ledger**, **the archive**, **the recycle bin** — and **cost** is what was spent, in five cuts that each sum to the same total. Where a provider reported no price, the page reports tokens and says why there is no amount, instead of printing `$0.00`.

## 6 Read the diff, and the merge that landed

Work that is meant to land goes through a pull request inside the city, and the rule that matters is not a rule anybody has to remember: **the resident who wrote the work cannot merge it.** A request that has not been verified has no method that merges it, so an unreviewed merge is a compile error rather than a policy. Verifying and merging are one action, because a verified request nobody merged would be a third state for you to chase.

What you read, in order:

1. The session's **changes** tab: one row per file that moved, in path order, with the patch of one file when you open that row. A line that matched the credential scan is reported by its line number and its reason rather than echoed, because printing the bytes to prove a leak is the leak.
2. `git log` in the building. Only two things ever reach a branch: the first commit of a repository that had none, and the merge that lands a reviewed piece of work. The fences the city writes before each wave of tool calls are commits nobody's `HEAD` points at, kept under `refs/sprawling/runs/`, so your own history keeps the shape you left it.
3. The merge commit's trailers, which `git interpret-trailers --parse` reads with no help from us: `Sprawling-Run`, `Sprawling-Actor`, `Sprawling-Model`, `Sprawling-Effort`, `Sprawling-City`, plus one `Reviewed-by` when this repository's git config carries `user.name` and `user.email`. The city does not invent a person's name.
4. The same question backwards: `sprawling whose ~/cities/first <commit>` answers which run wrote a commit, out of the Ledger rather than out of git, so a city exported to another machine with no `.git` beside it still answers. Exit code 1 means this city has no record of writing that commit, which is a different answer from "nothing changed".

**This build has no button that merges for you and none that rejects work already merged.** Your recourse is git, which holds every commit the city made, and **the recycle bin**, where every discarded file states the way back. Your brake is **stop the city**, which shuts every scope to new work.

## 7 Stop, and start again

`Ctrl-C` on the server, or **stop the city** on the page, which halts every run and is itself recorded.

After a restart:

```bash
sprawling resume ~/cities/first
```

That verifies the chain, closes tool calls whose outcome was lost when the process died, and reports what is waiting for a person. A city that was killed mid-call comes back saying what it does not know rather than pretending.

## If something does not work

| What you see | What it means |
|---|---|
| The page says the schema does not match | The binary and the page in your browser are different versions. Reload. |
| The model list is empty after attaching | The provider answered, but with nothing this build could read. The refusal names what it got. |
| A dispatch is refused before anything runs | No model answers for `main`. **settings** says so at the top of the page. |
| A run stopped and asks for something | It is on **waiting on you**, grouped so one answer covers one question. |
| A row on **the plan** says **stuck** | Something it needs has not finished, or a check failed. The row names what it waits for. |
| A merge was refused | The trunk moved after that work branched. The building rebuilds the work on the trunk as it stands and has it verified again. |
| A file is missing | **the recycle bin**. Every row states how to get that file back. |

More of these, with what to do about each, are in [`operating.md`](operating.md).
