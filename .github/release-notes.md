# v0.0.4 — pre-alpha

## Please do not download this release

**It is published to test the release pipeline, not to be used.** This tag
exists so that a machine can build the archives, sign nothing, publish them, and
let me check that the whole path from a green tree to a downloadable file still
works. Treat anything it produces as scaffolding.

If you want to look at the project, read the source. If you want to run
something, wait for a release that does not carry this notice.

## 请不要下载这一版

**这个版本是为了测试发布流程而发出来的，不是给人用的。** 打这个标签，是为了让机器把
归档构建出来、发布出去，好让我确认从一棵绿树到一个可下载文件的整条路还走得通。它产出
的一切都请当作脚手架看待。

想了解这个项目，请读源码；想跑起来用，请等一个不带这条提示的版本。

---

Everything below is this release's entry in
[`CHANGELOG.md`](../blob/main/CHANGELOG.md), unchanged.

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
