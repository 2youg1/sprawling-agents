# Changelog

Every release is a tag of the form `v<version>-Pre-alpha-<YYMMDD>`. The date is
part of the name because a pre-alpha version number says almost nothing about
how old the tree is, and how old the tree is, is what a reader of a pre-alpha
release most needs to know. A section written before its release is cut is
headed by the version the workspace manifest carries and the release's name,
and takes its tag when the release is cut.

Each entry records what changed. A wall-clock figure names the class of
machine that produced it, because the same figure from a busier or slower
machine is a different reading, not a regression; wall-clock figures are
readings and never gates. Byte counts are gated, because a byte count does not
depend on how busy the machine was.

The three releases before this file existed are reconstructed here from their
release notes and their commits.

---

## <!-- xtask:begin workspace_version -->0.0.7<!-- xtask:end --> citior

Pre-alpha. Nothing in this section has been published yet: it records what
landed in the repository after `v0.0.6-Pre-alpha-260922`. WIRE_V
<!-- xtask:begin wire_v -->42<!-- xtask:end -->, recounted from
`channels::WIRE_V` while this section is unreleased.

This section quotes no wall-clock figure. The measurements that would price
this release's changes are taken after it, so the entries below say what the
code does and leave how fast it does it to those readings.

### The client stands on Svelte

The page has now stood on three frameworks, and both moves had one cause. Each
time, the client had become hard to change, and the framework under it was
about to ask for a rewrite anyway: Dioxus's next line is in pre-release
(`0.8.0-alpha.1`, with `0.7.10` the newest stable), and Solid 2.0 is in release
candidates (`2.0.0-rc.6`) that rebuild the reactive core the 1.9 client was
written against. A rewrite owed to the next major version was coming either
way, so it went to a line that is not about to move: Svelte 5, the stable line
since October 2024. Between Svelte 5 and Vue, Svelte was chosen for what it
leaves in the built page: reactivity compiled into the bundle, no virtual DOM
and no runtime diff, so a streamed token re-evaluates the signals it touched
rather than a component tree. The client speaks only the WebSocket protocol in
`crates/channels`, which is what keeps replacing it a job for the client alone.

- The client's runtime dependencies are `svelte`, `effect`, `@lezer/highlight`
  and nine lezer grammars. Effect still has the one job of decoding the wire;
  the lezer packages colour code and load only when a page shows code.
  `cargo xtask npm` holds the list in both directions, so a deleted package
  fails the gate as loudly as an added one. (xtask/src/npm.rs:63)
- The eslint bridge kept for `eslint-plugin-solid`'s type gap left with the
  plugin; `eslint-plugin-svelte` types through `defineConfig()`. (client/eslint.config.ts:8)
- `AGENTS.md`, `ARCHITECTURE.md`, `README.md` and `README.zh-CN.md` name the
  framework the manifest names. (README.md:47)

### One writer, and a history that survives a crash

- A city has one writer across processes. `JsonlLedger::open` takes
  `.sprawling/ledger.lock` before it reads or repairs a segment, and a second
  `serve`, `resume` or `up` on the same city is refused with `E_LEDGER_HELD`
  and a recovery that says how to stop the holder. `serve` binds its port
  before it opens the writer, and a serve refused by the lock hands the port
  back. (crates/memory/src/jsonl/ledger.rs:84)
- Reopening a city after two writers continued the same line no longer
  deletes the second writer's records: a break on a later line of the last
  segment is refused as a broken chain instead of truncated as a torn tail. (crates/memory/src/jsonl/tail.rs:11)
- A wave whose write fails partway leaves the handle refusing every later
  wave, so nothing is written behind torn bytes. A Lean model
  (`adversary/design/Durability.lean`) proves that every record a handle
  answered `Ok` for survives tail recovery on reopen. (crates/memory/src/jsonl/barrier.rs:44)
- Open and replay judge a line through one per-line check, so a correctly
  chained line of a kind this build ignores is accepted by both. (crates/memory/src/chain_audit.rs:64)
- A ledger payload may nest 126 levels. The writer refuses deeper, where it
  used to write a line that replay then refused, and depth is judged before the
  float scan walks the value. (crates/kernel/src/event/payload.rs:22)
- The whole chain is audited segment by segment in constant memory, and a
  broken chain stops the writer: every later command is refused with the
  audit's own error. (crates/memory/src/chain_audit.rs:68)
- A served city runs that audit in the background, beside the writer rather
  than in front of it; an audit that cannot read the ledger stops the writer
  the same way. (crates/sprawling/src/assembly/chain_watch.rs:27)
- The views start from a snapshot of themselves. The snapshot is written whole
  (staged, synced, renamed), fits only the line it was cut at, and a damaged or
  mismatched one falls back to a fold from the first line, so losing it costs
  time and never history. A served city folds only the lines after it; a
  one-shot read, and a worker opened by `fork` or `adopt`, audit the whole
  chain before they trust one. (crates/memory/src/snapshot/start.rs:59)
- Startup folds stream the history one segment at a time and build the ledger
  index in the same pass, where they used to hold every raw line and every
  record at once. (crates/sprawling/src/assembly/folds.rs:93)
- The ledger index holds one 64-bit word per line, the seq implied by its
  place. (crates/memory/src/index/fold/entries.rs:12)
- What opening a ledger cut from a torn tail reaches the person: `resume`
  prints it and `serve` writes it to stderr. (crates/sprawling/src/assembly/lifetime.rs:56)

### A run reads and writes only where it may

- The read bound has one judgement, `kernel::address::may_read`: a run's own
  building is open, another building is open unless its rules say
  confidential, and a building whose rules do not read is closed. `read`,
  `search` and the transcripts they reach all ask it. (crates/kernel/src/address.rs:184)
- A link or junction is judged where it lands, so a link from an open building
  into a confidential one, into `.sprawling`, or out of the city reads
  nothing. A search that meets a link it cannot resolve says so, and a search
  from the city root names a building whose rules could not be read instead of
  passing it over as empty. (crates/runtime/src/tools/chosen_path.rs:102)
- `.git` is protected metadata beside `.sprawling`, matched in any letter
  case, and no write passes through a link. (crates/kernel/src/address.rs:40)
- A run that took in outside content — a web page, an MCP answer, the
  person's own browser — carries that taint to the command door, and `exec`
  from it is refused with `E_TAINTED_ACTION`. The taint is labelled with where
  it came from. (crates/kernel/src/error/code.rs:160)
- A provider key pasted into a dispatch, or written by a resident through
  `edit`, goes to the vault; the text, the request, the ledger and `JOB.md`
  keep a `secret:pasted/...` reference. (crates/sprawling/src/assembly/dispatching/custody.rs:19)
- The same custody covers the arguments and results of every tool on the
  bench. (crates/sprawling/src/assembly/workbench/tools/kept.rs:63)
- A credential written into an endpoint's `base_url`, as `user:key@` or in the
  query, is refused before it reaches the ledger. (crates/gateway/src/router/normalise.rs:191)
- A pull request merge refuses with `E_VERSION_CONFLICT`, naming every path,
  when it would overwrite an uncommitted change in the city folder. It used to
  replace the person's edit without a record. (crates/memory/src/error.rs:85)
- A background command's output returns only to the run that started it; the
  next run to call `exec` no longer receives another run's stdout. When a run
  ends, the commands it left running are terminated. (crates/runtime/src/backlog.rs:107)
- A run's `RULES.toml` is read once while the file's stamp holds, instead of
  once per path per call. (crates/city/src/policy/cache.rs:6)
- An export carries the city's git history as a pack, and a restore rebuilds
  the repository from it. A v0.0.6 bundle, which copied `.git` whole, restores
  as history too; a bundle whose `.git` borrows objects through `alternates` or
  a `commondir` is refused. The manifest counts the history's packs and refs,
  and restore checks them before it writes anything, so a restore is still all
  or nothing. Every bundle write is staged, synced and renamed. (crates/memory/src/bundle/history.rs:6)
- Restoring a discarded file refuses a link on its path, a file the person
  made after the discard, and a reserved address. (crates/memory/src/checkpoint/fence.rs:217)
- A fence is pinned by a reference named for its own commit, so `git gc` can
  no longer prune a commit a discard's restoration points into. (crates/memory/src/checkpoint/fence.rs:30)

### Providers fail the way they failed

- 408, 429 and every 5xx are asked again; other refusals are final. The
  recovery sentence is derived from the same classification. (crates/gateway/src/endpoint/failure.rs:153)
- A retriable failure waits before the next try: 500 ms, doubled for each
  failure in a row, capped at 60 s, and reset by the first answered turn. A
  halt reaches a run while it waits. (crates/runtime/src/watchdog.rs:165)
- A provider that names its wait (`retry-after`) is not asked sooner, and each
  run backs off on its own jittered schedule, so runs cut by one outage do not
  all ask again in step. (crates/gateway/src/endpoint/failure.rs:200)
- An error has three retry answers: yes, no, and unknown — the request left
  and its answer was lost, so the provider may have run and billed it. A cut
  stream and a silence timeout are unknown. (crates/gateway/src/endpoint/failure.rs:162)
- An error frame in the middle of an Anthropic stream, an OpenAI error chunk
  and a Responses error event are the failure they report, where they used to
  be dropped and read as a cut. (crates/gateway/src/anthropic/stream.rs:178)
- An MCP answer lost after the call left is marked as an effect nobody saw;
  a call that never left stays retriable. (crates/protocol/src/mcp/broker.rs:353)
- A refusal that names the context window is `Overflow`, not retriable, with
  `/new --carry` or a wider model as the way out. (crates/gateway/src/endpoint/failure.rs:61)
- A failure reaches the page with its kind, so a Chinese page words it in
  Chinese instead of showing the city's English sentence. (client/src/core/provider_failure.ts:7)
- A city with no model chosen answers `E_MODEL_UNCHOSEN`, whose one recovery
  is the settings page. (crates/kernel/src/error/code.rs:168)
- A local model streams through the same endpoint as any other, where it used
  to answer in one burst at the end; a stream request answered with a whole
  JSON body is read as that body. (crates/gateway/src/endpoint/adapter.rs:20)
- One HTTP client per endpoint, where every call built its own. (crates/gateway/src/endpoint/transport.rs:28)
- An MCP server's SSE stream is bounded by its opening instead of ended by it,
  so it no longer drops 15 s after connecting. (crates/protocol/src/mcp/sse.rs:18)
- A subscription token stays a bearer token when the settings are saved again. (crates/sprawling/src/assembly/credentials/subscription.rs:51)
- `input_tokens` means the whole prompt in every compatible format, and a
  cached token is no longer billed at the input price as well as the cache
  price. (crates/kernel/src/model/usage.rs:28)
- A price row with no figure leaves a call unbilled instead of billing it 0,
  and the cost answer carries how many calls no provider priced, so a city on a
  subscription or a local model reads as unpriced rather than idle. (crates/memory/src/attribution/report.rs:68)
- The Codex login row redirects to `127.0.0.1`, following upstream. (crates/gateway/src/oauth_profiles.rs:106)

### What a run's conversation carries

- Cancel and Steer reach a run that a lane is driving, and a steer inside a
  tool wave is recorded. (crates/sprawling/src/assembly/driving/flight.rs:306)
- A steer that arrives after a request was assembled waits for the next tool
  results instead of rewriting the message already sent, and a branch rebuilds
  it at the same place. (crates/runtime/src/conversation.rs:56)
- The person's words reach the request once. (crates/runtime/src/conversation.rs:15)
- A cut tool result names its rest file by its address in the room, which the
  `read` tool accepts. (crates/runtime/src/sieve.rs:25)
- A branch of a branch opens with the grandmother's conversation, and a
  carried session names the previous run's transcript. (crates/sprawling/src/assembly/freezing.rs:186)
- A frozen handoff is the room's own `Handoff.md`, pinned; the docs now say
  that `/new` without `--carry` discards the only copy of it. (crates/city/src/spine_files.rs:10)
- `status` reports the token count of the call that asked. (crates/runtime/src/turn/wave.rs:133)
- A turn's exchange is compacted only after the whole tool wave has landed,
  once per turn, and `compaction::plan` alone decides whether it fits. (crates/runtime/src/compaction.rs:205)
- A failed stream is asked once more through the blocking door when it failed
  as a wire mismatch; each recovery step answers recovered, failed or skipped,
  and a skip hands the same error on. (crates/runtime/src/turn/recovery.rs:33)
- A model's note, `.sprawling/models/<endpoint>/<model>.md`, ends the system
  prompt sent to that model. (crates/sprawling/src/assembly/freezing/model_note.rs:41)
- A room is named by rule from the task's first four ASCII words, never by a
  model, so the first provider call of a dispatch is the run itself. (crates/sprawling/src/assembly/dispatching/session.rs:68)

### Tools, fences and the prompt cache

- A read the model hands over while it is still generating starts during the
  generation. A Lean model (`adversary/design/Speculating.lean`) proves this
  leaves the ledger as serial execution writes it. (crates/runtime/src/turn/speculation.rs:20)
- The leading reads of a tool wave run at once, and the ledger holds the same
  lines a serial wave writes. (crates/runtime/src/turn/wave.rs:39)
- A fence is skipped for an empty or read-only wave, stages the paths the wave
  wrote rather than the whole scope, and lists in its payload only the paths
  it changed. A failed call widens the next fence to the write domain, and a
  building's first fence is written at adoption as one pack. (crates/runtime/src/run/fence.rs:6)
- Cache breakpoints have one author: the three segment edges and the tail
  message. The Anthropic request now marks the tail message, and
  `prompt_assembled` records the breakpoints a request really carried. (crates/runtime/src/prefix/breakpoint.rs:24)
- Each assembled request writes `prompt_shape_compared`: the tool table's and
  the conversation's shape, the region that changed since the request before
  it, and the most bytes the request can be billed as input. The four segment
  hashes stay on `prompt_assembled`. (crates/kernel/src/event/record/turn.rs:197)
- `prompt_assembled` is written once per run and again only when the payload
  differs. (crates/runtime/src/turn.rs:98)
- Keep-warm is a setting, off by default: `[cache] keep_warm` in a layer's
  `CONFIG.toml`. When on, a prefix a real request used is renewed once, just
  before the provider's five-minute cache would expire, by resending its last
  request with one output token; each renewal's usage or refusal is a
  `cache_renewed` line. (crates/city/src/config_layers.rs:242)
- A run cannot rewrite the rules that judge it: the `rules` and `city` tools
  keep `Effect::Govern`, which is refused on every call. (crates/city/src/city_tool.rs:18)
- Every dispatch books the documents it stands under. The city's
  `CONFIG.toml`, and the building's `CONFIG.toml` and `RULES.toml` when the
  building exists, are hashed, and one that differs from what was last booked
  gets a `rules_changed` line with the before and after digests before the run
  starts. (crates/sprawling/src/assembly/dispatching/agreeing.rs:184)
- A screenshot can cover a rectangle, an element, or several elements, and
  one function reads an element's box for both `measure` and the survey. A
  capture larger than the cap is taken once more at a lower pixel density
  instead of being refused. A quality outside 0–100 is refused by the browser
  and the desktop tools alike, where the browser clamped it and the desktop
  replaced it with 85. (crates/browser/src/shot.rs:181)
- A limit that refuses writes its own refusal in four parts, and no looser
  limit can be built outside the kernel. (crates/kernel/src/policy_limit.rs:7)
- An MCP answer whose text passes 16 KiB reaches the model the way a long
  command output does: stored whole, with a substitute in the window. A
  document converter joins a building as one more stdio MCP server, and
  `docs/operating.md` shows the entry. (crates/runtime/src/pipeline/connector.rs:31)

### Plans, claims and the people in a building

- A resident writes `Roadmap.md` and the other spine documents through the
  city. `PutSpine` carries the text the sender started from, a stale base is
  refused with `E_VERSION_CONFLICT`, and the write lands as
  `SpineDocumentWritten` before anything else. (crates/channels/src/command/kind.rs:341)
- A plan is measured in two figures: the count of leaves, which says how many
  pieces the plan turned out to have, and the weighted share, which says how
  much of the whole those pieces stand for. Shares are counted in integer
  billionths, so no float reaches a plan. (crates/kernel/src/share.rs:35)
- `plan` asks whether a node must be expanded, and a split reports what is
  left. (crates/collab/src/claim_tool/tool.rs:75)
- A run that ends holding a plan row turns it Blocked instead of leaving it In
  progress. (crates/sprawling/src/assembly/plans/held.rs:97)
- A claim on a plan node is decided by the accounting thread when the run
  asks, first asker winning, and `roadmap_claimed` is written then. Every way a
  run comes home closes the claims it holds, node by node as each closing line
  reaches the ledger; a split closes its parent. (crates/collab/src/claim_tool.rs:56)
- A workshop hands down only the nodes that are ready, keeps its graph per
  room, and hands down the next ready set when a node comes back. A knock at an
  occupied room waits for it, a handback wakes the resident who asked, and one
  conversation stops at 64 woken runs across its branches. A pursuit moves on
  when its runs land, without holding the desk while they drive. (crates/collab/src/workshop.rs:199)
- A standing goal on a building with no plan is refused with
  `E_PLAN_MISSING` and a button that puts a request for a plan in the
  composer; a plan that cannot be read keeps its own error. (crates/sprawling/src/assembly/plans.rs:161)
- A verification run checks each citation against the version pinned for
  review: a quote of an earlier draft does not hold even when its words
  match. `citysim` compares a red team's conclusions with and without that run. (crates/collab/src/citation.rs:61)
- After a restart, a building the person cleared no longer pursues again, a
  rejected pull request is not offered for review again, and a taken signal
  leaves the inbox. Each came from a fold that skipped a line it could not
  read; those folds now refuse it. (crates/sprawling/src/views/holding.rs:52)

### The city answers without waiting on its writer

- The views are folded on their own thread, and readers read a published
  snapshot of them without a lock; a query that reads disk, git or the network
  does it after the snapshot is let go. (crates/sprawling/src/serving/folding.rs:6)
- The accounting thread sleeps on one queue and wakes for what it serves.
  A Lean model (`adversary/design/Attending.lean`) proves a waiting message
  is served by the next wake. (crates/sprawling/src/assembly/driving/flight.rs:46)
- Each committed record is spelled as an event frame once, before the
  broadcast. A seq gap is owed to a socket as `Lagged`. (crates/channels/src/server/committed.rs:8)
- The city view carries the active runs and the 32 newest frozen ones; an
  older run still answers its own view and its cost from the ledger. An
  8,000-run city view falls from 1,234,534 bytes to under 16 KiB. (crates/memory/src/hot.rs:105)
- A question travels with an `ask_id` and its answer comes back under it,
  dated by the first seq the answer does not reflect. (crates/channels/src/wire/ask.rs:29)
- A reconnect fetches what it missed from the last seq the page folded, and a
  welcome from another ledger (a new epoch) makes the page rebuild. (client/src/core/socket.ts:165)
- An MCP server an earlier run started is reached again rather than restarted
  for every dispatch; a stdio server that has exited is started again. (crates/sprawling/src/assembly/workbench/servers.rs:6)
- A room under review keeps one worktree across its runs, checked out to its
  scope only. (crates/memory/src/worktree/trees/kept.rs:6)
- The view thread and the socket's workers stand one step above normal
  priority, and every command `exec` dispatches starts one step below: the
  below-normal class on Windows, `nice` on Unix, and `ionice` as well on
  Linux. `[core] priority = "normal"` in `~/.sprawling/config.toml` keeps the
  core at normal, and the doctor and the machine page say where the core
  stands. (crates/runtime/src/tools/exec/yielding.rs:26)
- A plan's next row waits while another run drives and less than a tenth of
  physical memory is available. (crates/sprawling/src/monitor/memory.rs:15)
- A dispatch on a volume close to full is refused before anything is written,
  and the refusal says how many bytes to free. (crates/sprawling/src/assembly/commanding/shedding.rs:20)

### The terminal

- One command table and a pure parser stand in front of every verb: `--help`
  no longer runs `up` or `install`, an unknown flag is refused, and a
  misspelled verb is answered with the nearest name. (crates/sprawling/src/main/grammar.rs:62)
- Exit codes are one table: 0 done, 1 refused, 2 a command line (or a `call`
  frame) that cannot be read, 3 quiet before the awaited answer or event, 4
  nothing at the address answered as a city. A refusal is a failure line, a
  recovery line and the nearest names to a person, and one JSON line to a
  program (`--json`). (crates/sprawling/src/main/exit.rs:21)
- `call` ends on its answer instead of waiting out the quiet window, and
  `call --until <kind>` ends on the event it names. (crates/sprawling/src/wire_client.rs:24)
- `doctor` writes its heading at once and asks every item in parallel. (crates/sprawling/src/doctor.rs:242)
- `sprawling dispatch` mints its own key, runs on the model `-m` names, waits
  for its own run, and exits 3 when silence ends the wait early. A run naming
  no model continues on its room's frozen one. (crates/sprawling/src/main/dispatch.rs:25)
- The console gives each typed line its own key and offers only the verbs the
  wire carries. `Query::Release` is renamed `Query::NewestRelease`. (crates/sprawling/src/console/language.rs:11)
- `sprawling view` prints ledger lines filtered by kind, address and text,
  and runs with their parents. (crates/sprawling/src/main/view.rs:12)
- A person viewing a whole city gets a raw-terminal face: it opens on the run
  waiting for the person, reads its first screen from the ledger's tail,
  follows the ledger while open, and unfolds a run into rounds and calls. (crates/sprawling/src/main/view.rs:253)
- `sprawling check <city>` reads every city TOML file and places each refusal
  at its line and column. (crates/city/src/check.rs:63)
- `sprawling top` watches a city's counters over the wire, one screen when the
  output is a terminal and one JSON line per sample otherwise. (crates/sprawling/src/monitor/top.rs:6)
- `sprawling up --supervise` serves in a child and resumes it after a crash,
  until three crashes in a minute. (crates/sprawling/src/supervising.rs:23)
- A serve that failed says so in the room's handoff, with its cause,
  instead of recording that the person closed the city.
  (crates/sprawling/src/assembly/lifetime.rs:72)

### The screens

- A reading thread stays where the reader is; it follows the foot only when
  the reader is already there. A room opens at its newest words. (client/src/views/talk.svelte:202)
- A streaming reply lays out each block as it closes, instead of jumping into
  shape at the end. (client/src/core/prose.ts:158)
- A notice's way out acts on the composer's room; a refused line goes back
  into the composer. (client/src/views/notice_recovery.ts:82)
- Without a main model, the welcome leads with the provider. A dispatch to a
  building is followed to the room its run started in. (client/src/views/welcome.svelte:38)
- The address bar decodes a room named in any script; a fork shows where it
  went and whom it came from. (client/src/core/route.ts:143)
- A question the city could not answer is drawn as unavailable, with a way to
  ask again, instead of loading for ever. (client/src/core/answered.ts:22)
- A hidden tab keeps draining its queue; a wire mismatch offers a reload
  rather than a reconnect; the tab icon tells its states apart by shape; words
  typed while the link is down wait for it. (client/src/core/socket.ts:82)
- Browser notifications for approvals, behind a switch that starts off. (client/src/core/prefs.ts:118)
- Code is coloured by its grammar from a lazily loaded chunk; a patch shows
  both line numbers; an opened tool call draws its arguments; an approval card
  draws what it asks about. (client/src/views/parts/code.ts:28)
- A path inside the city opens in VS Code or VS Code Insiders. (client/src/core/editor.ts:34)
- The city page draws its runs as a lineage tree with a time bar each, waiting
  runs first. (client/src/views/runs/lineage.ts:42)
- The run page opens on a time lens — the model speaking, tools running, the
  person being waited on — under a summary line, and names who dispatched the
  run. (client/src/views/run/lanes.ts:26)
- A monitor follows a run's code and terminal, with comment, revert and
  open-in-editor on every hunk; a running command's output reaches the page
  while it runs. (client/src/views/monitor/ansi.ts:30)
- A results-only mode for a room and for the city. (client/src/core/results.ts:25)
- A performance panel at `#/monitor`; the city samples only while somebody
  watches. (crates/sprawling/src/monitor.rs:6)
- A discarded file comes back from the recycle bin. (crates/memory/src/checkpoint/fence.rs:217)
- A building can be removed from its page: its directory moves under
  `.sprawling/removed/` with every file kept, and a busy building says why it
  cannot be. (crates/city/src/building/removal.rs:59)
- Slash verbs: `/stop` cancels, `/halt` and `/release` pair, `/clear` starts
  a new session, `/diff` opens the run; a mistyped scope halts nothing. (client/src/core/slash.ts:76)
- The model pill names the model the session answers with; choosing another
  opens a new session. (client/src/views/palette.svelte:96)
- Grey text a person reads takes the faint ink or darker; the disabled ink
  appears only on a disabled control. (xtask/src/color.rs:37)
- The cost page tells an unpriced city from an idle one. (crates/memory/src/attribution/report.rs:68)
- A timeout's heading names the question the page asked. (client/src/views/parts/combobox.svelte:48)
- A narrow screen keeps the risk readings, the palette and the rail in bounds. (client/src/views/palette.svelte:96)
- The Mayor is 市长 on a Chinese screen; an empty list says where its contents
  come from; an idle building can still be halted. (client/src/lang.json:287)
- The tuning defaults a page shows are answered with the layer that stated
  them, and the endpoint form leaves its tuning to the city instead of
  spelling figures of its own. (crates/channels/src/answer/config.rs:85)

### Skills

- `skills/` carries seven skills — `sdd`, `tutor`, `translation`, `why`,
  `how`, `blast-radius` and `authority-review` — as a shelf in the layout
  other agent harnesses read, mounted read-only through `[skills] shelves`
  and shipped in the release archive. (xtask/src/package/contents.rs:74)
- `sdd`, `tutor` and `translation` carry MPL-2.0. The other four are modified
  adaptations of MIT-licensed skills from `cursor/plugins` and keep that
  licence; `skills/LICENSES.md` travels with them. (skills/LICENSES.md:1)
- A skill may be a directory, and `read` opens its files by `<name>/<path>`.
  A read that misses lists the nearest existing entries and points at
  `search`. (crates/runtime/src/tools/read/miss.rs:6)
- `read` opens `cas:` and `file:` locators, judged at the building the bytes
  were stored for. (crates/runtime/src/tools/read/locator.rs:6)

### What the machines check

- Gates run in parallel and report in roster order; an unknown gate name is
  refused. (xtask/src/gates.rs:159)
- The `features` and `slices` gates and the history half of `guard` are
  gone; `apisync` left the roster, and the public surfaces of `kernel` and
  `channels` are checked nightly. The file budget counts production lines. (.github/workflows/ci.yml:195)
- A commit that changes gate machinery together with the source it judges
  carries the trailer `Verdict: user-approved`, spelled exactly so. No gate
  reads history any more; `cargo xtask commits <base>..<tip>` judges every
  subject and trailer in a range, and CI runs it on the change-set.
  (xtask/src/commits.rs:16)
- Every CI job calls a recipe of `just check`, and `just check` builds the
  Lean design models. (justfile:29)
- The handshake hash covers the event kind names, so a renamed kind is
  refused at the handshake. (crates/channels/src/wire.rs:73)
- `lexicon` reads `lang.json`. (xtask/src/lexicon.rs:6)
- The render gate measures crushed text and clipped popovers. (xtask/src/render/room.rs:9)
- The colour gate admits the disabled ink only behind a disabled variant. (xtask/src/color.rs:37)
- The secret gate reports a labelled hex key and no longer reads a whole
  PascalCase identifier as one. (crates/kernel/src/secret/hex_run.rs:11)
- The release walker does not enter nested worktrees. (xtask/src/walk.rs:14)
- Every text file is LF in the index. (.gitattributes:1)
- `SPRAWLING_E2E_REQUIRED=1` turns an absent end-to-end variable red. Both
  test suites judge addresses against one table. `SECURITY.md` names the one
  channel for reporting a vulnerability. (crates/sprawling/tests/e2e.rs:18)
- Every SPEC names section 12 Decisions. (crates/accounting/accounting-SPEC.md:372)
- `just check-all` runs every phase to the end, and `just check-branch`
  judges one branch. (justfile:37)
- `xtask` refuses to judge a checkout other than the one it was built from. (xtask/src/root.rs:6)
- `just mem` reads private, peak private and working set of a named process or
  of an empty city it serves itself; every bench reading carries its floor. (xtask/src/mem.rs:6)
- The embedded client is the bundle the workspace built, wherever
  `CARGO_TARGET_DIR` points. (crates/sprawling/build.rs:9)

### Removed

- The Zig leaf and `crates/mem`: nothing called it, and safe Rust scanned as
  fast.
- The `eval` crate; the handoff probe moved under the assembly, the
  instruments into `citysim`.
- `Command::Takeover` and `Command::Rollback`, which nothing carried out, and
  their two event words.
- `gateway::Native`.
- `kernel::highlight`, which nothing called.
- Naming a room with a model.
- `SummaryProducer`: nothing produces a summary. (crates/runtime/runtime-SPEC.md:1014)

### Built, and not reachable yet

- `city::install_skill` installs a skill package whole, after a static
  precheck that walks directory handles, refuses links and reparse points,
  and bounds a package at 32 MiB. No command, tool or page calls it. (crates/city/src/lib.rs:65)
- Going back opens a new tree at an earlier point and restores one file from
  it, recorded as `went_back` and `file_restored`. No command reaches it. (crates/memory/src/worktree/back.rs:6)

### A release says where it was built

- Every archive a release attaches carries a build-provenance attestation,
  signed through Sigstore by the workflow run that built it, before the
  archive is attached; a failed attestation stops the release. `gh
  attestation verify <archive> --repo 2youg1/sprawling` checks one, and
  `docs/getting-started.md` shows how. (.github/workflows/release.yml:231)
- The installers still check only the archive's sha256, which shows that the
  download arrived whole and not who built it: `install.sh` and
  `install.ps1` verify no signature and no attestation. (install.sh:152)

### Known and unfixed

- The page draws nothing for `log_truncated`. (client/src/core/belief/fold.ts:101)
- A dispatched command has no memory cap: the one safe interface on Windows
  is refused to an ordinary account. (crates/runtime/src/tools/exec/yielding.rs:26)
- The memory wait holds only a plan's next row; other entrances start a run
  without it. (crates/sprawling/src/monitor/memory.rs:15)
- A kept worktree of a closed room is never reclaimed. (crates/memory/src/worktree/trees/kept.rs:6)
- On Windows and macOS the monitor reads 0 for the process's own I/O bytes.
  (crates/sprawling/src/monitor/counters/own_process.rs:53)

---

## v0.0.6-Pre-alpha-260922

Reconstructed from the record the tag carried under *Unreleased*, checked
against the code at the tag, and from the commits that landed after that
record was last written. WIRE_V 34.

### A binary knows which release it is, and will say so when asked

`sprawling status` printed `0.0.5`, npm carried the same release as
`0.0.5-pre.260912`, and semver ranks the first above the second — so the
naive comparison a person or a script would write reported the newest
published release as the older one. Neither spelling could be compared
with the other because nothing decoded both.

- `kernel::Release` is the one authority for how a release is spelled and how
  two of them order. `xtask channel` converts through it to publish and a
  running binary converts through it to read the registry back. `Ord` is
  derived over version then date, the order npm itself would put the same two
  strings in.
- The release workflow passes its tag to the build. A binary built any other
  way reports itself as built from source, and `status` says which of the two
  it is.
- `sprawling version` answers, as do `--version` and `-V`; all three used to
  land in `unknown subcommand`. The version line carries the day the release
  was cut.

### Checking for a newer release is manual, and updating is not offered

`sprawling status --check` is the only command in this binary that reaches the
internet, and `Query::Release` is the only query that does. Both run because
somebody asked: no timer, no probe on connect, no check folded into another
command.

- The source is npm's `latest` dist-tag, not GitHub: every release here is a
  pre-release, and `GET /releases/latest` excludes those.
- Three answers: where a release stands, that this binary is not a release,
  or that the registry could not be read, with the stage `kernel::reach`
  names so a person behind a proxy is told where the call stopped. The exit
  code reports whether the question was answered, never what the answer was.
- Nothing updates anything. The terminal and the **machine** page print the
  command and stop.

### An error with nothing to do about it can no longer be written

`AxError::failure` returns a draft, and only `with_recovery` turns a draft into
an error.

- 172 errors carried an empty recovery line. Each now names a key to press, a
  file to edit, or a command to run; six whose subject could not carry a
  recovery had the subject corrected as well.

### A new variant is a compile error at every reader

- 44 `#[non_exhaustive]` attributes are gone, and with them 45 wildcard arms
  the compiler had already proved unreachable.
- Four clippy lints join the deny list, and the 37 discarded `Result`s and the
  boolean parameters they turned up are gone with them.
- The approval, governance and signal payloads are serde structs read and
  written through `Payload::of` and `Payload::read`, and a verdict this build
  cannot read is a read failure instead of a default.

### Where a city keeps things, and a document reaches disk whole

- The directories and the files every layer shares — `ledger`, `cas`,
  `library`, `CONFIG.toml`, `FILTERS.toml`, `JOB.md`, `Handoff.md`,
  `URBANITE.md` among them — are read off `kernel::layout`. The document
  names only the city reads, such as `RULES.toml`, `Memo.md` and
  `SCHEDULE.toml`, stay in `city`.
- A document is written through one door that renames it into place, one
  writer at a time, so a power loss mid-write leaves the old file.
- `config_layers::Ladder` makes the configuration layers a value, and a
  directory the city cannot read says so instead of reading as empty.
- The on-disk `views/`, `memory::projection` and the `redb` dependency are
  gone: a city's views live in the process and rebuild from the Ledger.

### How an endpoint is connected is decided once, at attach

- `ConnectionKind` is resolved after the pasted URL is normalised, written
  into `endpoint_attached`, and read back by `Query::Config`; the responses
  format a person pasted is no longer dropped.
- One output ceiling ladder answers for every model: the person, the
  provider's model list, a preset row citing its source, then the city's
  default, and the decision says which rung answered.
- A provider failure that completed no exchange is asked again, where the
  default retry setting used to mean no retry at all.
- A device-code login finishes a subscription sign-in, and xAI and Kimi
  attach through the same command steps as the other two families.
- Provider intelligence is followed from four vendors' own harnesses; which
  repository, path and commit was read stays in
  [`docs/third-party.md`](docs/third-party.md) section 1.

### Defects a person would have met on the default path

- A thinking block's signature survives the Anthropic stream, so the next
  turn is not refused.
- A tool call cut in half is refused instead of sent with no arguments.
- An MCP server can no longer hand the city a message larger than 8 MiB.
- The built-in price table reports its own failure, and an OAuth callback is
  split with its `state` checked.
- A bundle export compares its file count field by field against the source.
- A question the city never answers says so after fifteen seconds, and the
  run table forgets runs the city stopped listing.
- `/transcribe` and `/enroll` ask for the pairing token: an exposed city used
  to let anyone who reached the port write into a vault route and spend money
  transcribing.

### The screens

- A conversation has a second column: the artefact the last tool produced.
  Tool calls fold to one sentence.
- Each part states the WAI-ARIA pattern it implements, its key table, and
  where focus returns.
- Navigation is Ctrl/Cmd and a digit, and the registry screen has a route.
- Six numbers the city already counted reach the city page, four empty
  screens say what to do next, a field says it is wrong while it is typed,
  and an endpoint says when it is local.
- 26 `outline-none` are gone, so the focus ring is visible again; 33 `title`
  attributes become a hint a keyboard and a touch screen both reach.

### A door decides, and a question is a person's

A Gate answers `Allow` or `Deny`, and one door, `attach`, answers `Ask`: what
it would grant is the reading right over every login the person's own browser
holds, so no rule of the city can answer it. The approval queue holds what
only a person can answer — that question, and a design question a resident
asked. `Autonomy` has two values, `Owner` and `Delegate`.

### What another harness on the same computer already knows

- Codex's `[model_providers.*]` and pi's `providers` are read once into rows
  the city can attach: name, address, wire and the models listed.
- One direction, and no credential: nothing is written back, and a row says
  where the other harness keeps its key, never what the key is.
- An entry the city cannot attach is reported by name with the reason.

### The adversary asks what a saved setting reads back as

The out-of-tree property checker drives arbitrary sequences of
`configure_building` and holds three relations after each: the answer states
the last figure written, the building's `CONFIG.toml` states that figure, and
the layer above states none of them. 21 checks; a run of the new one takes
26 s on a four-core Windows machine with a debug binary.

### The adversary asks what a person settled, and says which seed it used

After any sequence of `PutPreferences`, the file on disk, the answer
`Query::Preferences` gives and the city's own configuration agree, and the
city states none of it. The served city points at a throwaway home
directory. `SPRAWLING_SEED` is read as stated, unstated or unreadable, and an
unreadable one ends the run with its own exit code; every run prints the seed
it used.

### What the machines check

- `cargo xtask docnum` recounts every number `ARCHITECTURE.md` and `LLM.md`
  quote.
- A budget row carries a unit; `dependency_count`, how many packages
  `Cargo.lock` resolves, is the first row not counted in bytes.
- `specalign` reconciles every kernel enum variant by variant.
- Three kani harnesses, three proofs.
- Six fuzz targets, three of them on the surfaces a stranger reaches: an
  inbound wire frame, a configuration file, an MCP answer.
- `just check-desktop` compiles, lints, tests and licence-checks the desktop
  connector.
- `just prereqs` is the one list of what a development machine needs. Two
  builds of one tree are compared nightly.
- The release binary is built at `opt-level = "z"`: 6,927,360 B against
  11,150,336 B at level 3.

### Carried, and not selected yet

- A third vault backend, one encrypted file opened by a passphrase
  (ChaCha20-Poly1305 per entry, the key derived by Argon2id). No probe
  selects it.

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
