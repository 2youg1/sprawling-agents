# Getting started

This guide takes you from nothing installed to a city that does work for you, and explains every idea you meet on the way. It is written for two kinds of reader: someone who already uses a terminal agent such as Claude Code or Codex CLI, and someone who has only used a chat window and has never handed a model a folder to work in. Part 1 explains the ideas, Part 2 walks one complete loop from install to a reviewed merge on your branch, and Part 3 covers what you do with a city once it is running.

For the meaning of a single word, [`glossary.md`](glossary.md) is the reference; for day-to-day operation and recovery, [`operating.md`](operating.md); for the design, [`../ARCHITECTURE.md`](../ARCHITECTURE.md). 中文版：[`getting-started.zh-CN.md`](getting-started.zh-CN.md).

---

# Part 1 — The ideas

## From a chat window to an agent

In a chat window you write, the model answers, and you copy what it said into your own files. The model sees only what you pasted, and it cannot do anything except write text.

An **agent** is a model given **tools**: functions it may call to read a file, search a folder, run a command, edit a file, or open a web page. The model decides which tool to call, the program around it runs the call and hands the result back, and the model decides again. This repeats until the model says the work is done. One pass of the model is a **turn**; a whole piece of work is many turns.

The program around the model is the **harness**. It decides which tools the model gets, what goes into its context, where it may write, what it must ask about first, and what is recorded. Claude Code, Codex CLI and Cursor's agent are harnesses for one agent at a time. **sprawling is a harness for many agents at once**, organised as a city on your own machine.

Two consequences follow for a chat user. First, you describe the result and what counts as done, not the steps: *"make the importer read files larger than 2 GB, and add a test that proves it"* is a good request, and a list of edits is not. Second, the work takes minutes, not seconds, and you do not have to watch it: the city tells you when it needs you.

## Models, providers and endpoints

A **model** is the thing that thinks — `claude-sonnet-…`, `gpt-…`, `qwen…`, whatever you pay for or run. A **provider** is who serves it: Anthropic, OpenAI, a relay service, or a server on your own machine. An **endpoint** is one provider as the city records it: a base URL, the **format** it speaks, and a credential.

The format decides the shape of each request. sprawling speaks two: the OpenAI chat format (`chat` on the settings page), which most providers, relays and local servers accept, and the Anthropic messages format (`messages`). A third shape, OpenAI's `responses`, is listed and refused, because the city does not call it.

There are two ways to pay for a model, and the city takes both: an **API key**, billed per token, and a **local model**, served on this machine with no key at all. The city signs in to no subscription.

The city gives models **roles**, so that the expensive model is not used for cheap work:

| Role | What it does |
|---|---|
| `main` | thinks: every run uses it unless the run names another model |
| `digest` | reads long documents and search results on `main`'s behalf, and follows `main` until you point it elsewhere |
| `transcribe` | turns a recording of speech into text; optional, and a city with none draws no microphone |

**Effort** is how long a model reasons before it answers, on the scale `none`, `low`, `medium`, `high`, `xhigh`, `max`, or **unstated**, which leaves the field out and lets the provider decide. More effort costs more per answer and helps planning and work that must be right the first time.

## The city

A **city** is one directory on your machine, and everything else is inside it. Copy the directory to another machine and it is the same city; delete it and nothing outside it changes.

| Word | What it is |
|---|---|
| **building** | One project, as a subdirectory of the city. Its rules, its configuration and the areas its agents may write are scoped to it. |
| **room** | A subdirectory of a building where one agent works. Its address, such as `lab/parser`, decides which files that agent may write, which documents it starts with, and whom it reports to. |
| **resident** | A standing identity at an address, with a file `URBANITE.md` that says who it is and what to bring it. It survives across runs. |
| **run** | One piece of work with a start and an end: an address, a task, a definition of done, a model. A resident is an identity; a run is the cost. |
| **session** | A stretch of conversation in one room. Dispatching into a room continues its session; `/new` starts a fresh one at the same address. |
| **Ledger** | The city's one history, under `<city>/.sprawling/ledger`. Every effect is written there as an event before it happens, and every page you read is rebuilt from it. |

Every city is raised with one building already standing, `hall` (City Hall), where two residents serve every other building. `hall/mayor`, **the Mayor**, is the city's planner: it turns an idea into a plan, raises buildings for the work, and hands each building its part. It writes Markdown and nothing else, because a planner that can run code stops reading the buildings' evidence and starts producing its own. `hall/clerk`, **the clerk**, answers questions on your behalf when you delegate that to it, and writes its reason into the Ledger.

## Scaffolding: the forms a building is raised with

A harness's *scaffolding* is the structure it puts around a model so that long work stays coherent: the files an agent reads before it starts, the files it writes before it stops, and the rules it cannot change. In sprawling that scaffolding is a set of Markdown and TOML forms that land in a building when it is raised (the templates are in [`crates/city/templates/`](../crates/city/templates/)). A form teaches by its shape: a blank form with the right headings makes even a small local model fill it in correctly.

| File | Where | Written by | What it is for |
|---|---|---|---|
| `SPEC.md` | building root | you or an agent | what the project is and the decisions it holds, before the code |
| `RULES.toml` | `<building>/.sprawling/` | you | what the building may do: where agents may write, whether work is reviewed, which skills are admitted, network egress, browser and desktop access, confidentiality |
| `Roadmap.md` | building root | agents, through the `plan` tool | the one task table, and the denominator for every progress figure |
| `Memo.md` | building root | agents | notes that have no other home |
| `Handoff.md` | building root | agents | what the next session needs to continue |
| `JOB.md` | room | you or the dispatcher | the task of one session; the agent reads it and leaves it unchanged |
| `URBANITE.md` | with the resident | you | who a resident is and how it works |
| `MAYOR.md`, `CLERK.md` | `<city>/.sprawling/` | you | who the Mayor and the clerk are |

`RULES.toml` and everything under a `.sprawling/` directory sit outside every area an agent may write. An agent reads its own rules and cannot edit them; it may propose a change, and you answer.

## Tools, skills and MCP

**Tools** are built in and given by the building's rules: `read`, `search`, `edit`, `exec` (run a program or a shell line), `plan`, `status`, `neighbours`, `delegate`, and more. A building with `browser = true` in its rules also gets a browser the city starts and owns.

A **skill** is a folder with a `SKILL.md` file in it: a set of instructions, and sometimes scripts, that teaches an agent how to do one kind of work. If you have used skills in Claude Code, the file format is the same. The difference is admission. A city keeps skills on **shelves** — the building's own shelf at `<building>/.sprawling/skills/`, the city's library, and any outside folder the city mounts read-only, such as a skills folder another harness already keeps — but an agent sees only the skills its building admits by name in `RULES.toml`:

```toml
reading_room = ["tutor", "blast-radius"]
```

A name on that list that no shelf holds is left out and reported, rather than promised. This keeps a building's context from growing with every skill on the disk. An outside folder is mounted in the city's own `<city>/.sprawling/CONFIG.toml`:

```toml
[skills]
shelves = ["~/.claude/skills"]
```

The **skills** group under **settings** shows every shelf, what each building admits, and how many runs used each skill. This repository ships nine skills under [`../crates/city/skills/`](../crates/city/skills/); the binary carries them, and a new city finds every one on its library shelf.

**MCP** (Model Context Protocol) is how a building reaches outside applications — mail, GitHub, Figma, a document converter — through a tool server. A server is one entry in the building's `CONFIG.toml`, a `command` for a program on this machine or a `url` for a hosted one, and the **MCP** page writes the same entries for you. A confidential building starts none.

## Rules, questions and the brake

The city decides most things by itself from the rules you wrote. Each action an agent takes passes a **gate** that allows it or refuses it, and a refusal always says three things: what was refused, why, and what the agent can do instead. An agent reads that and carries on.

What reaches you is a **question**: a design decision a resident cannot settle from the rules. It waits on the **waiting on you** page with **allow** and **deny**, and it is the one kind of thing the city raises a browser notification for.

To stop work, you have three verbs. **`/steer <text>`** adds an instruction to a running run without stopping it; the text lands at the end of its next tool result. **`/stop`** cancels the run in front of you. **`/halt`** stops a building, or with `--all` the whole city, until `/release`. There is no spend ceiling and no turn limit behind these: a run goes on until it concludes or somebody says stop.

## Coming from Claude Code

| In Claude Code | In sprawling |
|---|---|
| the terminal session | a page in your browser, served by the binary; the terminal becomes the city's console |
| the working folder | a room in a building; its address sets where the agent may write |
| `CLAUDE.md` | a building's `SPEC.md` and `RULES.toml`, a resident's `URBANITE.md`, and a project's own `AGENTS.md`, which the city hands to agents working in that project |
| `/clear`, `/compact` | `/new` starts a fresh session; `/compact`, the same as `/new --carry`, carries the room's `Handoff.md` across |
| `/model` | `/model <id>`, the **model** pill under the box, or the model table in settings |
| thinking budget | the **effort** pill, from `none` to `max` |
| plan mode | asking the Mayor, whose job is planning |
| subagents | other residents: a run can `delegate`, speak to a neighbour, or wake one, and every one of them is a run you can open |
| permission prompts | `RULES.toml` decides; only design questions reach you, on **waiting on you** |
| skills in `~/.claude/skills` | the same folders, mounted as a shelf and admitted per building |
| MCP servers | the same servers, per building, in `CONFIG.toml` or on the **MCP** page |
| `git` history of what the agent did | the Ledger, plus git trailers on every commit the city makes |
| Esc to interrupt | `/stop`, or Ctrl+. |

The largest difference is that nothing here is one conversation. The Mayor plans, buildings work in parallel, and you move between them.

---

# Part 2 — The first hour, end to end

## What you need

You need a desktop browser and a callable model: a compatible provider or a local model server. The standalone release binary needs no JavaScript runtime or database. npm needs Node.js; the Bun channel uses Bun. Building from crates.io also needs the compiler and native build tools required by the published package.

## 1 Install

This guide describes sprawling <!-- xtask:begin workspace_version -->0.0.10<!-- xtask:end -->, in <!-- xtask:begin maturity:word -->alpha<!-- xtask:end -->. Data formats, configuration, the wire and the interface may change between versions; keep a recoverable backup before an update. Commands below install the version actually available through that channel, which may differ from the source version described here.

Choose one channel:

| Channel | Install | Prerequisites and result |
|---|---|---|
| npm | `npm install --global sprawling@latest` | Node.js and npm; downloads a prebuilt binary. |
| Bun | `bun install --global sprawling@latest` | Bun; downloads a prebuilt binary. Put Bun's global bin directory, reported by `bun pm bin --global`, on PATH. |
| crates.io | `cargo install sprawling --locked` | Rust supported by the published package and native build tools for the platform; compiles locally. The package includes the built browser client. |
| cargo-binstall | `cargo binstall sprawling` | [cargo-binstall](https://github.com/cargo-bins/cargo-binstall); downloads the release archive using the published crate's metadata. |
| Manual archive | Select your platform in the [release list](https://github.com/2youg1/sprawling-agents/releases) | Windows x86-64, macOS on Apple silicon or Linux x86-64; unpack and run the included binary. |

The shell installers download an archive without needing a JavaScript or Rust toolchain. macOS/Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/2youg1/sprawling-agents/main/install.sh | sh
```

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/2youg1/sprawling-agents/main/install.ps1 | iex
```

The scripts print the selected tag, platform and download size, check the archive's SHA256 against the release's published digest and refuse a mismatch. Their last step, `sprawling install`, copies the binary to your program directory and adds that directory to PATH; `sprawling install --uninstall` reverses that installation. npm/Bun and Cargo manage their own bin directories. Open a new terminal if a PATH change has not reached the current one, then check:

```sh
sprawling version
sprawling help
```

### Select and verify a version

The [release list](https://github.com/2youg1/sprawling-agents/releases) includes prereleases; GitHub's `releases/latest` endpoint excludes them. Choose an actual published tag and use its asset names, changelog and checksums. Placeholders such as `<release-tag>`, `<npm-version>`, `<crate-version>` and `<archive.zip>` below must be replaced before running a command. A Git tag, npm version and crate version have different spellings: the release workflow derives them through [kernel::Release](../crates/kernel/src/release.rs).

Pin npm/Bun with `sprawling@<npm-version>`, Cargo with `cargo install sprawling --locked --version <crate-version>`, or binstall with `cargo binstall sprawling --version <crate-version>`. For a shell installer, set the full tag only for this invocation:

```sh
SPRAWLING_VERSION='<release-tag>' sh -c 'curl -fsSL https://raw.githubusercontent.com/2youg1/sprawling-agents/main/install.sh | sh'
```

In PowerShell, preserve any existing setting:

```powershell
$previousVersion = $env:SPRAWLING_VERSION
try {
    $env:SPRAWLING_VERSION = '<release-tag>'
    irm https://raw.githubusercontent.com/2youg1/sprawling-agents/main/install.ps1 | iex
} finally {
    if ($null -eq $previousVersion) {
        Remove-Item Env:SPRAWLING_VERSION -ErrorAction SilentlyContinue
    } else {
        $env:SPRAWLING_VERSION = $previousVersion
    }
}
```

For a manual download, compare SHA256 with the digest published for that exact asset. SHA256 verifies bytes against that digest; it does not identify the builder. The release workflow also supplies a Sigstore build-provenance attestation, checked with [GitHub CLI](https://cli.github.com):

```sh
gh attestation verify <archive.zip> --repo 2youg1/sprawling-agents
```

Operating-system code signing is separate from both checks. A signing application or workflow configuration does not establish that a downloaded binary is signed; inspect the selected asset's signature and release notes. None of these checks guarantees that an antivirus product will accept the file. Treat an OS warning as a reason to verify provenance before deciding whether to run it.

A checkout builds the complete deliverable with `just dist`. A plain `cargo build` before `just build-web` embeds a page explaining that the client bundle is missing. Follow the [contribution prerequisites](CONTRIBUTING.md) for source builds.

### With AUR

The AUR package is not published yet. Release builds generate and validate the package on Arch Linux; publishing requires an AUR account and the `AUR_SSH_KEY` secret. Missing credentials skip publication with a notice.

Once published, Arch Linux x86-64 users with `base-devel`, Git and unzip can inspect the PKGBUILD and build it as an ordinary user:

```sh
git clone https://aur.archlinux.org/sprawling-bin.git
cd sprawling-bin
makepkg -si
```

The package verifies the release archive's SHA256 and installs its binary, resources and licences, without calling `sprawling install` or changing shell configuration. The generated PKGBUILD defines the installation directory and links the binary into `/usr/bin`. Remove the package with `sudo pacman -R sprawling-bin`. The version check identifies the package's layout and displays its update command; a newer GitHub archive can precede its AUR update, so check the package version before updating.

### With Nix

On Linux x86-64, Nix can build the complete application from this repository. sprawling is not packaged in nixpkgs, so the repository flake is the Nix path. Enable the Nix features `nix-command` and `flakes`, and check out a tag or commit whose `flake.nix` builds the client before Rust and includes a committed `flake.lock` before building:

```bash
nix build .#default --no-update-lock-file
nix run .#default --no-update-lock-file -- status
nix run .#default --no-update-lock-file -- up ./cities/first
```

[The flake](../flake.nix) reads the pinned compiler from `rust-toolchain.toml`, Rust dependencies from `Cargo.lock`, and client dependencies from `client/bun.lock`; `flake.lock` fixes the Nix builders. Nix fetches the locked dependencies before the application build runs in its network sandbox. The build runs the existing client command and embeds its output, so the application carries the page as well as the shipped skills, templates and licences. The first build may compile dependencies before it builds the application.

The version line comes from the binary. A checkout built without release provenance reports `built from source`; a version number alone does not identify a published Git tag. To update, choose another actual tag or commit and run the commands again. `nix develop` opens the development shell; it does not start a city.

## Updating

Updates are manual. Check in Settings or with `sprawling status --check`, and use the channel that installed this copy. Verify the reported channel before following its command; a copied binary or ambiguous bin directory may not identify the installer. Read the selected release's CHANGELOG on its tag or release page, especially changes to data formats, configuration and the wire. The main branch's prepared changelog is not evidence that that version has been published.

Before replacing the binary, finish or cancel active work and stop the city with `Ctrl-C` in its console. Keep the old binary or enough package-version information to reinstall it, and write down `sprawling version`. Export with that old version, into a new directory outside the city, then restore into a separate unused directory:

```sh
sprawling version
sprawling export ./cities/first ./backups/before-update.bundle
sprawling restore ./backups/before-update.bundle ./cities/restore-check
sprawling replay ./cities/restore-check/.sprawling/ledger
```

Each command must succeed before updating. A bundle is a directory; the existence of that directory alone proves nothing. Restore verifies the chain, manifest counts and head, plus carried git history; replay independently verifies the restored Ledger. Inspect important project files and history in the restored copy as well. Use new destination names for subsequent backups and checks, and keep the original city stopped during export.

Export carries the Ledger, content objects, city/project files and supported git history; it omits credential plaintext, derived views, git hooks/configuration and the city root's reserved configuration. Preserve host configuration and any external project repositories separately through your own protected backup process. Credentials remain in the host vault; on another host you must enroll them again. Remote pairing also depends on the host's city key. Keep backups private because they contain project work and conversation history. [Bundle](../crates/storage/spec/Bundle.lean) defines the exact contents and verification.

Update through the original channel:

| Installed through | Update | Reinstall a selected version |
|---|---|---|
| npm | `npm install --global sprawling@latest` | `npm install --global sprawling@<npm-version>` |
| Bun | `bun install --global sprawling@latest` | `bun install --global sprawling@<npm-version>` |
| Cargo | `cargo install sprawling --locked` | `cargo install sprawling --locked --version <crate-version>` |
| cargo-binstall | `cargo binstall sprawling` | `cargo binstall sprawling --version <crate-version>` |
| shell/PowerShell installer | Run that installer again after stopping the city. | Set `SPRAWLING_VERSION` to the exact published tag as described above. |
| Manual archive | Download and verify the selected archive, then run its binary's `install` command if you previously installed it. | Keep or download the archive from the selected tag. |
| AUR sprawling-bin | In the original checkout: `git pull --ff-only && makepkg -si` | Inspect and build the PKGBUILD revision for the selected release. |
| Repository Nix flake | Check out the selected tag or commit, then run the [Nix commands](#with-nix). | Use the original pinned checkout and lockfile. A fixed commit does not follow newer releases. |

After updating, run `sprawling version` and `sprawling help`, check the intended executable is the one on PATH, then reopen the city with `sprawling up ./cities/first`. Confirm the page connects, configuration loads, project files and history are present and a small task works. Keep the old version and backup until this check succeeds.

### Roll back safely

Stop the updated city. Restore the previous binary through its original channel, verify its version, then use that previous binary to restore the pre-update bundle into a fresh city directory and run `replay` on its Ledger. Reapply any separately backed-up host configuration, enroll credentials where needed, and start this restored city. Do not ask an older binary to read a city that the newer version migrated unless the selected release explicitly documents that compatibility. Preserve the updated city separately for inspection; rollback to the backup does not retain work performed after it was taken.

## 2 Raise a city, and open it

```bash
sprawling up ./cities/first
```

This raises the city if the directory does not hold one, serves it on `127.0.0.1:8787`, and opens the page in the browser your operating system opens links with. The terminal becomes the city's console: `/help` lists what it takes, `/serving` repeats where the city listens, and `Ctrl-C` stops the city. The console prints one line for each record the city commits — its position, its kind and its address — and never what you typed or what a model answered, because a terminal is seen by whoever stands nearby; `sprawling up --whole-records` prints each record whole. It prints the same on Windows, macOS and Linux. `sprawling` with no arguments shows the folder it would start a city in, and Enter starts it there.

To work on projects you already have, raise the city in the folder that holds them, or move a project into the city's folder, and take it in as a building with `sprawling adopt ./cities/first myproject`. Adopting overwrites no file; it adds the city's forms beside your work and a `.gitignore` entry that keeps the city's notes out of your project's history.

`sprawling doctor` checks this machine in two tiers. The first is what a city uses, such as a browser engine for the browser tool. The second is what changing this code takes, and it is the list `just prereqs` reads: on a new machine without administrator rights, the development setup is complete when every required row of that tier answers. On Windows one of those rows is bash, which every `just` recipe runs in. Run `just` from Git Bash, because the `bash` another terminal finds first can be `C:\Windows\System32\bash.exe`, which starts WSL rather than a shell. A contributor can check that walk without a new machine: `gh workflow run on-demand.yml -f job=fresh` unpacks this tree's release archive on Windows, macOS and Linux runners, runs `sprawling doctor` with the environment a new account has, walks from install to the first dispatch, and uploads the checklist and logs as `fresh-<os>-<tree>`. A runner's account is an administrator, so the job checks the environment a new account has, not an account without administrator rights.

## 3 Connect a provider

A city with no model to call opens on **welcome**, whose first card, **connect a provider**, leads to **settings** → **accounts and providers**.

**official harnesses**, the next group, is where a subscription comes in: the city never signs in to one itself. The page lists Claude Code, Codex, Grok Build, Kimi Code and Pi, whether this computer can run the command that starts each, that command, and a link to the vendor's own sign-in instructions; you sign in inside the harness.

The form above the list of what is attached takes a key. Its first control lists the providers the city knows by host: pick one and the base URL and the format are filled in for you. The three boxes below it are:

| Field | What it wants |
|---|---|
| **base_url** | the base URL from the provider's documentation. With no scheme it is read as `https://`, or as `http://` for an address on this machine, and a path you leave out is filled from the city's table of known hosts; the form shows the URL it will actually call |
| **wire_api** | the format: `chat` for the OpenAI shape, `messages` for the Anthropic shape |
| **key** | the provider's key, or empty for a local server |

**list models** asks the endpoint what it serves and shows each model it found; **attach** registers the endpoint with the models you ticked. Beside the form the page shows the `config.toml` this would write and the request it would send. The key never becomes part of a command or a frame: it goes to your operating system's credential service, and from then on configuration, events and logs hold only a reference of the form `secret:realm/name`.

Then give models their roles. The model table has a role column, so a model can take its role as it is attached, and **which model thinks** holds one box per role afterwards. A dispatch with no model chosen for `main` is refused with `E_MODEL_UNCHOSEN`. The context window and output ceiling columns may stay empty: an empty ceiling is filled from what the provider stated, then from the city's table of known models, then from its default.

A local server can also be named before the city starts: with `SPRAWLING_MODEL_URL` and `SPRAWLING_MODEL` set in the environment of `sprawling serve`, a city with nothing registered attaches that server with no key and chooses the named model for `main`.

## 4 Talk to the Mayor

**the Mayor** is the conversation with `hall/mayor`, and the page the city opens on once a model is chosen. Enter sends a message; Shift+Enter starts a new line.

Under the box, four pills say how the message runs, and a click changes each:

| Pill | What it sets |
|---|---|
| **model** | the model that answers; a session keeps the model it started on, so choosing another opens a new session |
| **workspace** | the room that hears the message |
| **effort** | how long the model reasons first |
| **mode** | what the run does with your message: **chat** answers what you say and changes nothing else; **work** carries out the task towards the goal you state |

Write the idea in one sentence — *rewrite the ledger reader so a cold read of a million events stays under a second, and write down the numbers you measured* — leave the workspace at `hall/mayor`, and press Enter.

The Mayor reads every building's `Roadmap.md`, `Memo.md` and `Handoff.md` first. It writes the city's plan into `<city>/hall/Roadmap.md` through its `plan` tool, one row per line of work. It raises a building through its `city` tool when no existing building should hold the work, or adopts a directory you point it at, and never raises two buildings for one project. It hands each building its part, keeps each building working through `pursue` until the part runs out, and records what it decided in `<city>/hall/Memo.md` before it reports back.

You can also skip the Mayor. `/raise lab` raises a building named `lab` from the `minimal` template (`/raise vault confidential` raises a confidential one), and switching the **workspace** pill to a room of that building sends your message straight to it.

## 5 Watch the city work

**city** draws one block per building: a lit window is a run at work, a lamp by the door is something stuck, a flag on the roof is a standing goal, and the plinth is how much of the plan is done. Pick a building to see its progress, what is ready, what is stuck and which runs worked there, then **open the building**.

A building's page shows **the plan**, drawn from its `Roadmap.md`, whose rows read **not started**, **ready**, **working**, **stuck**, **awaiting approval** or **done**, and lists its **rooms**, **files**, **commits**, **changes** and **skills**. The box at the top gives the building a standing goal: **set as standing goal** keeps it handing out ready work by itself until nothing is ready and nothing is in flight.

A run has its own page, with seven lenses: **time** (where the time went, turn by turn), **turns**, **monitor** (the terminal, the changed files and the file the run is working on, followed live), **prompt** (exactly what went to the model), **context**, **changes** and **evidence**. In **monitor**, a hunk can be reverted, or commented on, which reaches the run as a steer.

**When residents write to each other.** A resident's message to another room is a **letter**. In the receiving room's conversation a letter is a card on the left that names the room it came from, links that room's session, and gives its kind, time and text; your own words are never drawn as a letter. In the sender's tool rows a send reads "send to @room: …" and says where the letter landed: delivered to a run at work, queued, or knocked for a new run. A run that waits for a reply says which room it waits on and how the wait ended. A letter carries the standing of the resident that wrote it and never yours, even when it quotes you, and every resident's `City.md` tells it to check a decision a letter reports against the hall's `Memo.md` or the plan before acting on it.

**Open a file in your own editor.** Under **settings** → **advanced** → **Open in my editor**, choose the editor this computer has — VS Code, VS Code Insiders, VSCodium, Cursor, Windsurf or Zed — and enter the city's folder as an absolute path. From then on every file the monitor shows is a link that opens it at its line. The browser hands the link to the editor; the city starts nothing.

## 6 Answer what residents ask

**waiting on you**, behind the mailbox key at the foot of the page's first column, holds every design question a resident asked that nothing can move without. Identical questions are grouped into one card, and **allow** or **deny** answers the whole group; what was blocked is dispatched again with your answer settled. A question that began with content from outside the city — a web page, a tool result — is marked **outside content** and never grouped, so you answer it knowing where it came from.

**settings** → **run** → **approvals** chooses who answers: **me**, or **the clerk**, whose decisions are listed under **answered for you**.

## 7 Read the diff, and the merge that landed

Work that is meant to land goes through a pull request inside the city, and the resident who wrote the work cannot verify it: a request that nobody else verified has no method that merges it. Read, in order:

1. The run's **changes** lens: one row per file that moved, and the patch of a file when you open its row. A line the credential scan matched is reported by its line number and reason, never echoed.
2. `git log` in the building. The checkpoints the city writes before each wave of tool calls are commits no `HEAD` points at, kept under `refs/sprawling/runs/`, so your history keeps the shape you left it.
3. The merge commit's trailers — `Sprawling-Run`, `Sprawling-Actor`, `Sprawling-Model`, `Sprawling-Effort`, `Sprawling-City`, and `Sprawling-Predecessor` for a run that replaced another. A merge you reviewed also carries `Reviewed-by`, when this repository's git config holds `user.name` and `user.email`.
4. `sprawling whose ./cities/first <commit>` answers the same question backwards from the Ledger, given the full forty-digit commit id. Exit code 1 means this city has no record of writing that commit. Add `--trace` to list the calls the run made since its previous commit, each with its time, tool and outcome; other runs that called tools in the same building in that span are counted beside them as candidates.

No page merges for you, and none rejects work already merged. Your recourse is git and **the recycle bin**, where every discarded file states the way back; your brake is `/halt --all`.

## 8 Read what it cost, and what happened

**cost** is money and tokens cut by run, by resident, by prefix segment, by skill and by tool, each cut summing to the same total. Where a provider reported no price, as with a local model, the page counts the calls and tokens instead of printing `$0.00`.

**the record** is the one history read through four lenses: **the ledger** (every event, with filters that say how many rows they hid), **the archive** (a search across what every building keeps, at the moment you ask), **the recycle bin** and **the log** (this process's diagnostic log). From a terminal, `sprawling view ./cities/first` reads the same Ledger without a served city, and `--runs` prints the run tree. `--since 2026-05-14T09:00:00Z --until 2026-05-14T10:00:00Z` keeps the lines whose own time falls in that hour: UTC, to the second, ending in `Z`, the end left out.

## 9 Stop, and start again

`Ctrl-C` in the console stops the city. `/halt --all` stops the work and leaves the city serving. After a crash:

```bash
sprawling resume ./cities/first
```

This verifies the chain, closes the tool calls whose outcome was lost, and reports what waits for you. `sprawling up --supervise` does it for you after every crash, until crashes come too close together.

---

# Part 3 — Living with a city

## The box, the commands and the keyboard

A line that begins with `/` is a command, and the menu above the box lists them:

| Command | What it does |
|---|---|
| `/dispatch <task>` | opens a run in the room the box speaks to |
| `/steer <text>` | adds an instruction to the running run without stopping it |
| `/stop` | cancels the run in front of you |
| `/halt [addr\|--all]`, `/release [addr\|--all]` | stop a building or the city, and let it go on |
| `/raise <addr> [minimal\|confidential\|hall]` | raises a building from a template |
| `/new [--carry]` | start a fresh session in this room; `--carry` brings the room's `Handoff.md` |
| `/fork [addr]` | starts a second line of conversation from the newest run in a room |
| `/model <id>`, `/effort <level>` | point `main` at another model; set the effort |
| `/diff`, `/go <page>`, `/mcp`, `/doctor`, `/help` | open changes, a page, the MCP page, the machine check, the list |

Ctrl+K (⌘K on a Mac) offers every command with every page, building and room beside it. The keys that ship: Ctrl+1 to Ctrl+6 for the Mayor, city, MCP, the record, cost and the registry; Ctrl+, for settings; Ctrl+Shift+A for **waiting on you**; Ctrl+B for the mailbox; Ctrl+J for the inspector; Ctrl+P to find a file; Ctrl+\ to change how much of the city the page draws; Ctrl+. to stop the run in front of you, which on a page with no run going stops nothing and names `/halt --all`, the verb that stops the whole city; Ctrl+/ for the key list; Ctrl+Shift+F to fork from the entry under the pointer; Ctrl+Shift+Y, Ctrl+Shift+E and Ctrl+Shift+X to answer the decide card that holds the focus; and `/` outside a text box to focus the box. Every key that changes something holds Ctrl, so a letter typed while the focus is outside the box changes nothing. **settings** → **keybindings** changes any of them.

## Sessions

A room keeps its session until you start a new one, so a second message to the same room continues the same work with the same model and effort. `/new` forgets both, so you may choose again, and carries nothing; `/new --carry` carries the room's `Handoff.md`, the five-section summary the last session wrote for whoever comes next. `/fork` opens a new session that begins with another run's conversation up to a line of it, which is how you try a second approach without losing the first. A run whose context window is filling up is told so, with enough budget left to write a handoff and hand the work to a successor at the same address; every link in that chain is a run you can open.

## A building of your own

A building raised from `minimal` is ordinary: its agents may write every file under the building, work is not reviewed, and no skill is admitted. Open `<building>/.sprawling/RULES.toml` in your editor to change that; each key is explained in the file, and a key the city does not know is refused rather than ignored. The keys people change first:

```toml
does = "The importer for the lab's instrument logs."   # one paragraph for an agent arriving here
conventions = "Rust 2024; every change passes cargo test."
review = true                  # work is offered for review before it lands
reading_room = ["tutor"]       # skills admitted into this building
egress = ["api.github.com"]    # domains work here may reach
browser = true                 # the browser tool
```

`sprawling check ./cities/first` reads every TOML file in the city and prints each error as `path:line:column`.

**A confidential building** (`/raise vault confidential`) is for data that must not leave: a run there is stopped before any call to a remote provider, it starts no MCP server, and writes to other buildings are refused. Pair it with a local model.

## Outside tools

The **MCP** page adds a tool server to a building by command, by URL, or from a pasted JSON block, and shows whether each server answers. The same entry in `<building>/.sprawling/CONFIG.toml`:

```toml
[[mcp]]
label = "docs"
command = "markitdown-mcp"
```

[`operating.md`](operating.md) walks through this server in full, and through a second city whose building drives a browser to watch the first.

## Driving a city from a terminal or a script

```bash
sprawling dispatch lab "add a --json flag to the report command"   # prints events until the run ends
sprawling dispatch lab "…" --detach                                # prints the run id and returns
sprawling top                                                      # the monitor, in the terminal
sprawling call '{"ask":{"ask_id":1,"query":"city_view"}}'          # one frame of the wire
```

`sprawling call` with no frame lists every command and query the wire carries. Its exit code is the answer — 0 answered, 1 refused, 2 your command line, 3 nothing came back in time, 4 no city at that address — so a script branches on it without parsing JSON. [`wire.md`](wire.md) is the whole wire, written for an agent that drives a city from outside.

## Another machine on your network

```bash
sprawling serve ./cities/first 0.0.0.0:8787
```

An address that reaches past this machine needs a pairing key. If `SPRAWLING_PAIRING_TOKEN` is set, the city adopts it and never prints it; otherwise the city mints a key for this serve alone and prints it once in the banner, with an address that carries it. The next start replaces it. From outside your network, a device reaches the city through the remote door and a route you choose. Settings > Remote opens the door for a time you pick, closes it, and replaces the city key; opening and replacing the key print a code on the terminal that runs `sprawling serve`, which you type into the page within two minutes, the same on Windows, macOS and Linux. Pairing a device stays at the console: [operating.md](operating.md), *Reaching the city from another device*, says how, and what the route is trusted with.

## Moving a city

```bash
sprawling export ./cities/first city.bundle
sprawling restore city.bundle ./cities/copy
sprawling replay ./cities/copy/.sprawling/ledger
```

Restore compares the chain with the bundle's manifest before it reports success, and `replay` verifies the chain offline, read-only.

## If something does not work

| What you see | What it means |
|---|---|
| The page says it speaks one wire version and the server another | The binary and the page in your browser were built apart. Reload. |
| The model list is empty after **list models** | The provider answered with nothing this build could read. The report under the form says at which step: name, connection or answer. |
| A dispatch is refused before anything runs | No model is chosen for `main`: `E_MODEL_UNCHOSEN`. Give a model that role. |
| A run stopped and asks for something | It is on **waiting on you**. |
| A row on **the plan** says **stuck** | Something it needs has not finished, or a check failed; the row names what it waits for. |
| A merge was refused | The trunk moved after the work branched. The building rebuilds the work on the trunk as it stands and has it verified again. |
| A file is missing | **the recycle bin**: every row states how to get that file back. |
| A setting does not take effect | `sprawling check <city>` prints each error in the city's TOML files. |
| An editor link does nothing | The editor chosen under **settings** → **advanced** is not the one registered on this computer, or the city's folder there is not this machine's absolute path. |
| This machine lacks something the city needs | `sprawling doctor` lists it, and `--install` offers each missing item, one at a time. |

More of these, with what to do about each, are in [`operating.md`](operating.md).
