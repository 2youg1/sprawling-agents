# Changelog

Every release is a tag of the form `v<version>-Pre-alpha-<YYMMDD>`. The date is
part of the name because a pre-alpha version number says almost nothing about
how old the tree is, and how old the tree is, is what a reader of a pre-alpha
release most needs to know.

Each entry records what changed and, where a number is claimed, the class of
machine that produced it. Wall-clock figures are readings from a four-core
laptop with 16 GB of memory and never gates — a slow runner is not a defect. Byte counts are gated, because a byte count does not depend on how
busy the machine was.

The three releases before this file existed are reconstructed here from their
release notes and their commits.

---

## Unreleased

### A binary knows which release it is, and will say so when asked

`sprawling status` printed `0.0.5`, npm carried the same release as
`0.0.5-pre.260912`, and semver ranks the first above the second — so the
naive comparison a person or a script would write reported the newest
published release as the older one. Neither spelling could be compared
with the other because nothing decoded both.

- `kernel::Release` is the one authority for how a release is spelled
  and how two of them order. `xtask channel` converts through it to
  publish and a running binary converts through it to read the registry
  back, so the version this project publishes and the version it
  recognises cannot drift apart. `Ord` is derived over version then
  date, which is the order npm itself would put the same two strings in.
- The release workflow passes its tag to the build. A binary built any
  other way reports itself as built from source rather than guessing at
  a release it is not, and `status` says which of the two it is.
- `sprawling version` answers, as do `--version` and `-V`; all three
  used to land in `unknown subcommand`. The version line now carries the
  day the release was cut, read from what is compiled in and costing no
  network.

### Checking for a newer release is manual, and updating is not offered

`sprawling status --check` is the only command in this binary that
reaches the internet, and `Query::Release` is the only query that does.
Both run because somebody asked: no timer, no probe on connect, no check
folded into another command. `QUICKSTART.md` opens by promising that
nothing was installed and nothing outside the folder was written, and a
binary that polled a registry on its own schedule would be spending that
sentence on a question nobody asked.

- The source is npm's `latest` dist-tag, not GitHub. Every release here
  is a pre-release and `GET /releases/latest` excludes those by design —
  it answers 404 for this repository — so the endpoint that looks right
  is the one that would have been wrong.
- Three answers, not two: where a release stands, that this binary is
  not a release at all, or that the registry could not be read. The
  third carries the staged reading `kernel::reach` already defines, so a
  person behind a proxy is told where the call stopped rather than that
  it failed. The exit code reports whether the question was answered,
  never what the answer was.
- Nothing updates anything. `sprawling install` owns the archive path
  and npm owns its own, so both the terminal and the **machine** page
  print the command and stop.
- WIRE_V 31.

---

## v0.0.5-Pre-alpha-260912

The shape of the work was: make the first ten minutes work, and make the
screen a person judges this product by say what it means.

### One press connects an outside application

The MCP page drew Composio as three identifiers a person had to fetch
from somebody else's console — a key, a server id, a user id — and the
connect button said the city had no such command. It has one now, and
nobody types an id: the city connects under its own name, the directory
comes from the broker instead of from a hand-written list that went
stale weekly, and the consent tab opens inside the click so a browser
cannot mistake it for a popup.

- `Query::Toolkits` reads the shelf and where each application stands;
  `Command::ConnectToolkit` opens the consent session. Four standings,
  each a different next action, and every one of them the broker's
  reading rather than something this city remembered.
- Nothing polls. The page re-reads when it opens, when the button is
  pressed, and when the person comes back to the window from the consent
  page — returning is the event.
- What reaches the Ledger is the request, never the standing and never
  the consent url: a standing is a fact about now, and a consent url is a
  capability nobody should hold by replaying a log.
- `protocol::mcp` still connects to any tool server and still has never
  heard of this one. Exactly one module knows which broker holds an
  application's OAuth, and it brokers OAuth and does nothing else.

### A tool call says what it was asked for, and a turn says when

A row in the thread showed which tool ran and what came back, and
nothing about what went in — so "the tool failed" and "the tool was
asked for the wrong path" read identically. `Call` now carries its
arguments, bounded by the same rule and cut at the same limit as its
result, and `Turn` carries the moment the Ledger wrote the event that
opened it, read from that record rather than from a second clock.

### Linux is built, and kani runs

`install.sh` had recognised `x86_64-unknown-linux-musl` for a while and
nothing ever produced one. The release matrix has a third row, and the
question that had been blocking it — where a Linux install keeps an API
key, when a static musl binary and a D-Bus secret service cannot both be
true — is answered by the kernel keyring reached through keyutils: a
syscall, needing no bus, no dynamic library and no desktop session. What
it costs is what `Persistence::ThisBoot` already said it costs, and the
key that does not survive a reboot now says so where a person reads it.
kani verification runs on a Linux runner rather than nowhere.

### A model that was never asked to answer

Every model the built-in catalogue did not know was registered with an
output ceiling of zero, and zero was sent on the wire. A provider answers
`max_tokens: 0` with no content at all, the run then froze as `done`, and
the thread showed a run that thought for three and a half minutes and
said nothing. Two independent paths produced that false history, so both
are closed.

- `kernel::Ceiling` wraps a non-zero count, and a model's ceiling is
  `Option<Ceiling>` from the catalogue row to the wire: absent is absent,
  and zero cannot be spelled. The OpenAI wire omits the field and takes
  the provider's default; the Anthropic wire, which requires it, refuses
  with a recovery naming where to register the number.
- A reply with nothing in it, or one the provider cut off at the ceiling,
  freezes as `limit` rather than `done`. `Completion::Done` has to cite a
  `model_returned` as its evidence, and an empty one is not evidence.
- Usage is asked for on OpenAI-shaped streams, which never reported it
  before, so every streamed call billed something and accounted nothing.

### The machine's own proxy, and never for itself

reqwest was built with default features off, which dropped the system
proxy along with them: on Windows and macOS this was the one tool on the
machine that could not see the proxy its owner had configured, and the
symptom was a fifteen-second timeout naming nothing. SOCKS comes with it
and costs no package. Found by running the suite afterwards: with a
system proxy compiled in, a machine whose owner runs one sent loopback
through it, and a local model server answered 502 through somebody else's
gateway — so every client that can reach this machine refuses to proxy
it. `kernel::reach` says where a call stops, stage by stage, so a refusal
can name the stage instead of quoting an error chain.

### The screens

One implementation per control instead of a class string per view. The
composer opens in the middle of an empty room and rides down to the bar
on the first send. A `/` in the box opens the same command table the
palette reads. Keys are data: an action table, platform rendering,
rebinding and conflict detection. The provider form can hold a second
key — the reference now carries the name it was minted from, which is
what let the first provider's key reach the second. A model table can
state the ceilings the wire could not carry before. A browser is a
family rather than a brand, so a machine with Zen or Brave on it is
ready. Both install scripts show version, platform, percent and the
checksum result. Paths a page prints can be opened where a person keeps
their files.

Geist Sans and Geist Mono ship with the client, which reverses a recorded
decision: a type stack that begins with whatever the machine happens to
have is a different product on every machine.

### Numbers

- The client the browser downloads: 282.1 KiB gzipped, half of it the two
  faces. The register was re-priced with that reason.
- A durable write costs one disk barrier, and a barrier costs the same
  for fifty records as for one: 585 microseconds per record at one per
  barrier against 13 at fifty on one windows-x86_64 NVMe machine, of
  which about 2.5 is the write. The ledger port grew a batch face and the
  relay hands over everything already waiting at once.
- npm's platform packages moved into the `@sprawling` scope; the root
  package keeps its bare name, because `bunx sprawling` is what the
  channel exists for.

---

## v0.0.4-Pre-alpha-260911

The shape of the work was: give the city a planner to talk to, replace the
client with one anybody can rewrite, and make a resident able to finish a piece
of work on its own — read a file in parts, search for a symbol, run the build,
hand over to its successor when the window runs out.

### The city has a city hall

- Every city is raised with one building already standing: `hall`, which holds
  no project of its own. The Mayor plans and writes Markdown; the clerk answers
  the approvals a person delegated, in the same three parts a Gate uses. The
  Mayor has no `exec`, no `delegate` and no `workshop`, because a planner that
  can run code stops reading the buildings' evidence and starts producing its
  own.
- `WriteDomain::Documents` lets a resident write the city's Markdown and
  nothing else; the reserved subtree and every `Roadmap.md` stay out of reach,
  so a planner cannot rewrite the plan it is being measured against.
- The `city` tool raises, adopts and lists buildings, so a plan can grow the
  city it describes.

### The client is TypeScript, and the WebAssembly one is gone

- `client/` is Solid and Effect, built by bun, and the page it produces is
  embedded in the binary at build time. The first screen is a conversation with
  the Mayor rather than a dashboard.
- The city is drawn as a map; a building opens as a file tree; a run reads
  through four lenses — rounds, changes, evidence, cost — each answered by the
  city rather than folded in the browser.
- A building's git history is a page: commits, the files one commit changed,
  the patch itself, and the session that wrote it as a link into that room's
  conversation. Lines matching a credential shape report their line number
  rather than their bytes.
- Voice input where a transcription endpoint is registered: the recording goes
  to `POST /transcribe` and the text lands in the composer rather than being
  sent, because a machine that mishears must be correctable before it spends a
  run.
- The wasm client and its 112 files left the tree. One client, one build
  command, one bundle: 108.0 KiB.

### A resident can finish a piece of work

- `read` takes an offset and a limit, and a new `search` tool finds a symbol
  without a shell. Both are capped at 64 KiB cut on a line boundary, so a
  generated file cannot spend a window; the envelope says which line to resume
  from and that answer is the only one.
- `exec` inherits exactly the environment variables a building declares, so a
  resident can run this repository's own toolchain. A command that outlives a
  ten-second window returns a handle and keeps running; `halt` terminates what
  it started, including delegated runs.
- `runtime::sieve` compresses a tool result without a model: the same seed and
  the same filter table replay a byte-identical window, and every compressed
  result carries the way back to the original.
- A frozen run exports the messages the model actually saw beside its room, and
  a successor inherits the same depth and the same tools. Provenance records the
  predecessor, so a lineage reads back as a chain.
- Context is reported at 25% and 65% of the window, once each, off the token
  count the provider returns rather than an estimate of the bytes.
- The spending ceiling is deleted. Nothing here prices a piece of work before it
  runs; the brake is `halt`, and the cost page still reports what was spent.
- A provider that runs out either freezes the run or moves to a named
  endpoint — never a silent substitution, because that is the one decision a
  default must not make.

### Concurrency, and who owns the Ledger

- One thread owns the Ledger and the books; a pool of driving threads owns
  nothing but the drive in flight. A building working towards a goal takes its
  whole ready set at once, four runs at a time, each in a lane of its own. Runs
  are driven in parallel and accounted for in series.
- A wave fence no longer moves `HEAD`: checkpoints are written under
  `refs/sprawling/runs/`, so a city's own history does not grow a commit per
  tool wave.
- Every commit the city makes carries who ran it, under which model and effort,
  in which city; `sprawling whose <city> <oid>` reads it back from the Ledger
  rather than from git.

### Sight, and the desktop

- An image is a content block on the wire, translated into both dialects, with
  the bytes fetched from the content store at the last moment.
- The `browser` tool drives a real browser over BiDi: open, snapshot, act,
  screenshot, measure, console, viewport, close. A screenshot lands in the
  content store and comes back as evidence.
- `sprawling-desktop` is a separate MCP server for Windows: windows, snapshot,
  act, screenshot, record, clipboard — each refused on any platform without an
  implementation rather than faked. A building reaches the desktop only by
  saying so, and the allowlist lands where no resident can widen it.

### Doctor, platforms, and the first run

- `sprawling doctor` is the single authority on what this machine has. The first
  screen shows its answer, each item a state rather than a sentence, and a city
  that has never been probed says so instead of guessing.
- A static `x86_64-unknown-linux-musl` archive was built for the release matrix
  and is **not** in this release. Its first real build found the reason: the
  Linux credential store is D-Bus secret-service, and a static binary cannot
  link one. What that build needs settled is where a Linux install keeps a
  person's API key, which is not a decision a release workflow makes on its way
  past. Windows and macOS archives are what this tag carries, as before.
  Everything else the row needs is built and stays — `just package` takes a
  triple, the archive name carries it, and `install.sh` knows that name.
- A flake derives its toolchain from `rust-toolchain.toml` rather than
  restating it.
- Every CI job declares how long it may take, so "fifteen minutes" is a
  checkable promise rather than a wish.

### Numbers

- 1,415 tests across the workspace, 79 more in the desktop server, 29 in the
  client. A full run takes 83 s on a sixteen-core laptop with a warm cache.
- The wire is at version 21 with 24 queries and 24 commands; the client's types
  are generated from it and a gate refuses a tree where the two disagree.
- The binary is 9.47 MiB on Windows; the bill of materials lists 286 packages.

---

## v0.0.3-Pre-alpha-260903

The shape of the
work was: make the ledger fast enough
that the interface could be judged, give the city a plan it can walk on its own,
then open the client in a real browser and look at it.

### The city can now walk a plan without being spoken to

- `kernel::plan` — a plan is a tree of addressed nodes, and five malformed
  shapes are refused at the point of construction rather than reported later. A
  cycle is refused as a walk, not as a set, because the person fixing it needs
  to see which edge to cut.
- `kernel::share` — weight is conserved by construction. `Share` has no
  constructor, no arithmetic and no `Deserialize`; the only way to make one is
  `split`, which consumes its input. Conservation is not a rule that is checked,
  it is the only thing the type can express.
- `kernel::pursuit` — a standing goal. The stopping condition is exactly one
  thing: the ready set is empty **and** nothing is in flight. Money is
  deliberately not part of it.
- `kernel::blockage` — a red node propagates up the tree and forward along
  dependency edges, and comes out as one sentence naming the source rather than
  as seventeen red dots.
- The claim tool gained `split` and `block`, reaching six actions, and both new
  ones must carry a reason. The whole six-action schema costs 547 bytes of
  prompt, one byte less than the four-action version did.
- The board page — ready, waiting on dependencies, in progress, blocked, done.
  It holds no state of its own and has no dragging: a node moves because a run
  reported that it moved.

### The interface

- The stylesheet became a file the client and the settled screens both link, so
  a screen is settled against the stylesheet the product ships rather than
  against a copy of it.
- Dispatch collapsed to a single box. Everything else is inferred, the inference
  is one sentence, and every word in that sentence can be clicked and changed —
  so a wrong inference costs one click rather than one re-entry.
- Streaming, end to end. `gateway::endpoint` reads server-sent events; a token
  delta travels as its own frame class and never enters the event stream,
  because a token increment is not an effect. When the settled text and the
  streamed text disagree, the page draws the settled text.
- A session page now answers what a person actually asks: what this session
  cost, which gate it is waiting at, how much context is left, and when the
  handoff was written.
- The prompt the city sent is visible, segment by segment, with the hash of each
  segment — read out of the `prompt_assembled` event rather than recomputed, so
  the page is not a second authority on its own contents.
- A skill whose bytes moved since the last run that mentioned it says so. It
  says *this changed*; it never says *this is safe*.
- Typography and layout: one left edge per page, a spine that is a column rather
  than a margin each child sets for itself, and a sans stack led by open-licensed
  faces. Lato was measured and dropped — a stack entry that changes what a reader
  sees without proving it is better is worse than none.
- The cost page, the approval page and the record table speak the reader's
  language. Eleven phrases in two languages, 1.2 KB, and the gate that now
  refuses the defect they fixed.

### Speed, all measured on one machine

| Reading | Before | After |
|---|---:|---:|
| One `Query::RunHistory`, 50,000-record ledger, end to end | 2,823.5 ms | **1.1 ms** |
| The run-history read itself | 27.6 ms | **3.311 ms** |
| Reading one ledger line at a random offset | 742 µs | **~7 µs** |
| Ledger append, p50 / p99 | 1.047 / 1.879 ms | **0.562 / 0.838 ms** |
| A tool wave that changed one file, whole cycle | 262.8 ms of scan alone | **37.8 ms** |

The four causes were each measured before they were fixed: a byte-at-a-time
read, an index rebuilt under every question, a scan proportional to the worktree
instead of to the change, and a segment file reopened for every single write.
None of them was a guess, and the biggest of them was on the write path where
nobody had looked.

### Four new gates, and one deleted

`cargo xtask gates` now runs sixteen.

- **`ax`** — every role, accessible name and landmark a settled screen wrote
  down is offered by the client too.
- **`wiring`** — a verb the city can carry out is reachable from the client, or
  it is classified on the wire seam with a reason a person may not ask for it.
  It caught the direction nobody watches: `Pursue` and `SetAutonomy` were on the
  wire, matched by the worker, covered by tests, and **no control could reach
  them**. A drawn button that fails is loud; a button that does not exist is
  silent.
- **`render`** — the settled screens are opened in a real engine and the boxes
  are measured where they landed. A conflict inside the cascade exists in
  neither source file, so no gate that reads source can see it.
- **`wording`** — every word a reader is given comes from `web::lang`. Judged by
  position: a literal sitting in a text node or a spoken attribute is an
  English sentence on a Chinese page.
- **`zerojs` deleted.** It asserted that this repository's own commands never
  invoke npm or node. That excluded an architecture rather than a defect: the
  client shipped here is WebAssembly, and a second client in any language is
  supported. Three documents that read as a ban on JavaScript were corrected in
  the same change.

`length` gained two more units — a file may not pass 1,000 lines including its
tests, and a function may not take more than four parameters. `guard` learned to
tell a loosening from a tightening, so striking an exemption no longer costs a
ruling.

### Maintainability, with the debt written down

Eleven files stood above the new 1,000-line limit; all eleven are back under it
and the register of what predates the rule is **empty**, which is that rule's
completed state. Every cut was taken at a seam the architecture or the crate's
own SPEC had already named, never at a line count: `crates/sprawling/src/assembly.rs`
went from 11,461 lines to a module tree, and moving `impl RunWorker` into sixteen
submodules changed the visibility of not one field and added not one public item.

Forty-two over-long signatures are down to twenty-four. An exemption is keyed by
path and function name and **may not travel with the function**, so a signature
being moved is either fixed or left where it is.

### V10: an adversarial property checker, outside the tree

`adversary/` is a third client — Lean, outside the workspace, driving the
shipped binary over the wire, written to attack rather than to use. Thirteen
properties in 2 min 36 s, four cores. It is never a gate: on a machine without
Lean, `just check` behaves byte for byte as it does where the directory is
absent.

It has found three things a specific trace would not have:

1. `sprawling call` exits 0 when a refusal does not arrive inside the quiet
   window, while its own documentation promises that exit 1 means the city
   refused. An agent branching on that code reads a failure as a success.
   **Open — the ruling belongs to the Rust side.**
2. A dispatch the city was going to refuse wrote `JOB.md` to disk first.
   **Fixed**: every refusable judgement now runs before the first
   byte is written, in one new phase, and no caller sees a different error code.
3. The wire makes all twenty-three state-changing commands carry an `IdemKey`,
   `kernel::gate::dedup` implements the check as a pure function, and nothing
   calls it — so the same command replayed under the same key happens twice.
   **Open.** A key that must be carried and is never read is a promise the door
   makes and does not keep.

Getting the suite from *does not finish in 1,800 seconds* to *30.8 seconds* was
four measured causes, of which the instructive one is that parallelism was
swapping cities: when two runs contended for a port, the loser's city exited and
the loser's own liveness probe reached the winner's city on the same port, so a
whole trace ran against somebody else's history — **and it reported a defect that
was open at the time as green.** Ports are now lent from a pool and the tree runs
serially.

### Repository

Renamed to `sprawling-agents`. Both READMEs now cross-link
[kusanagi](https://github.com/2youg1/kusanagi) and state the division: one chain
inside a city, one chain per pair between cities.

### Fixed

- A dispatch to an address that was never raised left a directory tree behind
  and nothing in the ledger (found by the adversary suite).
- A dispatch into the reserved subtree wrote `JOB.md` before being refused. The
  checker's generator always supplied a session name, so this had never been hit.
- Startup no longer treats an unreadable plan as a plan somebody emptied.
- `#3` — `replay` on a directory holding no ledger reported success. It now asks
  whether there are segments at all, and answers `E_PATH_NOT_FOUND`.
- `#5` — one gate's internal failure discarded the verdicts of every gate after
  it, including `guard`. All sixteen now run to the end and the exit code takes
  the heaviest state, because *not judged* and *judged and clean* were reading
  the same.
- `#2` — `kernel::trybuild` failed on any machine with the `rust-src` component
  installed.

### Known and unfixed

Issue [#1](https://github.com/2youg1/sprawling-agents/issues/1) stays open. The
front end is better and it is not yet worth a working day.

Three defects found by opening the client in a real browser against a real
provider are recorded in the client's own SPEC and are
not all fixed: a turn panel whose heading denied the eight turns listed
underneath it, a fold labelled with the wrong speaker, and a page that showed
every part of the answer except the question the person typed. **None of the
three was caught by a gate**, and the reason is the same for all of them: a gate
compares what two sides wrote down, and nothing a page leaves out leaves a trace.

`#/setup` and `#/cost` have no settled screen, which is why the defects keep
landing there.

---

## v0.0.2-Pre-alpha-260827

Cut twice on the same day under the same version number, deliberately: the
second cut changed nothing about what the program does, only whether a person
could watch it do it.

- **A turn says what happened in it.** The client folded three of fifty-eight
  event kinds and discarded the rest, so a refusal and a successful file read
  reached the screen as the same grey line. The message, the token usage, the
  stop reason and the billed amount had been on the wire the whole time.
- **Opening yesterday's session showed an empty page.** History could not be
  asked for by session, so four concurrent sessions split one slice of five
  hundred records.
- **A session lists the files it changed**, with `+` and `−` counts taken from
  git between two real checkpoints. The list cannot contain a file an agent
  merely read, because the fence is the write domain.
- **Dropping a file on the composer** now reaches the composer and never presses
  the button, and a drop target lights up during the drag.
- **A building's documents** come apart by weight and slant rather than by
  colour. This interface has two colours; a syntax-highlighted document would
  have spent both on something that means neither.
- Three features existed and could not be reached: pairing a browser from
  another machine (a four-link chain with three links cut), the console
  answering a query, and the `exec` refusal naming a build that did not exist.
- Four failures reported as something else: a run under review writing to the
  building's shelf, an unreadable file reported as an empty one, a change
  outrunning its own record, and a ceiling lost when work was handed on.
- The dispatch path went from one 1,069-line function to 158 lines across named
  phases, and a gate started failing the build on any production function past
  200 lines.

## v0.0.1-Pre-alpha-260824

The first cut. Two capabilities landed just before it, both verified end to end.

- **Residents find each other.** A run asks for its neighbours and gets every
  address it can reach inside its building, each carrying the line that
  resident's own `URBANITE.md` offers about what to bring them. Detail decays
  with distance: the rest of the city comes back as building names.
- **A message reaches its reader whether or not they are working.** Speaking to
  a busy resident slips the message under the door, where it lands at the end of
  their next tool result; speaking to an idle one starts a run for them. Either
  way it arrives labelled with the sender's address, which is also the address
  that answers it — and a resident cannot render as you, which is a property of
  the type rather than a convention.

Two residents in one building negotiated six hours of kiln time to a written
agreement against a real provider. That is the evidence behind both claims.

---

## Thanks

- **[@Ameshika](https://github.com/Ameshika)** — PR #4, which brought dependabot
  to both ecosystems this repository has.
