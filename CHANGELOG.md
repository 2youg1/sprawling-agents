# Changelog

Every release is a tag of the form `v<version>-<maturity>-<YYMMDD>`, and the
releases this tree cuts are <!-- xtask:begin maturity:word -->alpha<!-- xtask:end -->; `kernel::release::MATURITY`
is the one place that is decided. The date is part of the name because an
early version number says almost nothing about how old the tree is, and how
old the tree is, is what a reader of an early release most needs to know. A
section written before its release is cut is headed by the version the
workspace manifest carries and the release's name, and takes its tag when the
release is cut.

Each entry records what changed. A wall-clock figure names the class of
machine that produced it, because the same figure from a busier or slower
machine is a different reading, not a regression. Wall-clock figures and byte
counts are both readings and never gates, because a faster binary that is
larger is the better binary.

The three releases before this file existed are reconstructed here from their
release notes and their commits.

---

## v0.0.10-Alpha-261007

**sprawling 0.0.10 innocent**

Alpha, cut on 2026-10-07 (UTC) as `v0.0.10-Alpha-261007`. The workspace manifest carries <!-- xtask:begin workspace_version -->0.0.10<!-- xtask:end -->; `kernel::release::MATURITY` owns maturity, and the release workflow supplies the tag to a released binary. It records what landed after `v0.0.9-Alpha-261004`. The wire moved from WIRE_V 56 to 64, once for each push in which its shape changed (`crates/wire/Spec.lean` D1): 57 when a doctor report's sandbox arm gained `macos_seatbelt`, 58 when a provider endpoint began to carry an ordered account list, 59 when the person's preferences gained the performance group and the release answer lost the Homebrew origin, 60 when the settings page began to read an endpoint's stored keys, tuning and the city's `[search]` back, 61 with the `ForgetSecret` command, 62 when the body size lost its upper bound, 63 with the privacy query and operation frames, and 64 when the appearance preference gained the reading face. `EVENT_LOG_V` stays at 2: a line written by 0.0.9 reads unchanged. A city written by 0.0.9 opens without conversion: its views and standing are folded again from the start of the Ledger on the first open, because the snapshot format changed; provider keys 0.0.9 stored in the vault are used as they are; and the personal configuration is read without being rewritten ([updating](docs/getting-started.md#updating)).

### Behaviour changes

- `/clear` is removed. `/new` is the only verb that replaces the session in a room, and `/new --carry` brings the room's `Handoff.md`; branching is a separate feature and keeps `/fork`, the fork key on each entry and the mailbox's fork entry (`client/Spec.lean` D44).
- There is no default per-run memory ceiling. A run's commands get one only when the person enters a number of bytes under Settings → Performance and chooses `[core] placement = "soft_shares"` ("soft with memory ceiling"); an empty field means no ceiling. All commands of one run share it; Windows jobs and delegated Linux cgroups enforce it, and macOS does not. When a command reaches the ceiling, or the ceiling could not be applied or read back, the command's result and the monitor and inspector terminals say so instead of passing silently (`crates/runtime/spec/Tools/Exec.lean` D95).
- Action shortcuts that were single keys now hold the accelerator (Ctrl, or ⌘ on a Mac): Ctrl+Shift+F forks from the entry under the pointer (was `f`); Ctrl+Shift+Y, Ctrl+Shift+E and Ctrl+Shift+X answer the decide card that holds the focus (were `y`, `e` and `n`; refusing moved to X because Ctrl+N opens a browser window before the page hears it); Ctrl+\ changes how much of the city the page draws (was `\`); and Ctrl+/ opens the key list (was `?`). `/` stays a single key, because it only moves the focus into the box. A letter typed while the focus is outside the box therefore changes nothing (`client/src/core/keys.ts`).
- The read-only view of an earlier session no longer draws a "continue from its end" button. Going on from an earlier session is a branch: use the fork key on one of its entries or the mailbox's fork entry.
- sprawling is not distributed through Homebrew: no tap is published, and the update check recognises no Homebrew install. On macOS, install through npm, Bun, crates.io, the shell installer or the release archive.
- A plaintext credential written by hand into an `[[mcp]]` row of `CONFIG.toml` is now refused when the file is read; see Configuration below for the rule and the recovery.
- The default body size is 15 px instead of 14 px. The smallest accepted size stays 12 px, and there is no upper bound any more (`wire::BODY_PX_MIN`, `crates/wire/spec/Preference.lean` D51).

### Main input area and settings

Before a session starts, the row under the text box holds three controls: the workspace chip, one model entry (provider, then model, with thinking beside it) and one permissions entry with two switches, the mode and the write limit. A control with nothing to choose is hidden. Once the session has a run, no control stays under the box; the model, effort, mode, write limit and, where a real restriction exists, the sandbox are recorded on the session's first message head. The placeholder names the agent the box speaks to and disappears as soon as the person types. Fork buttons stand at the end of their entry's own line and no longer cover text.

Settings gains a Performance group: CPU placement (`none`, `soft`, `soft_shares`, `pinned`) and core priority (`raised`, `normal`), each with a note on what the platform does, and the per-run memory ceiling above. The values are kept in `[core]` of `~/.sprawling/config.toml` and apply after the city server restarts. Every settable field of the personal configuration and of a building's `CONFIG.toml` now records in its crate's specification either the control that edits it or the reason it has none, and `xtask wiring` turns red for a new field without that record.

The client's stylesheet `client/src/theme.css` is now an entry that imports its parts from `client/src/theme/`, one part for each owner: the token tables, the light block and the person's choices, the surfaces, the motion vocabulary, the base layer, and the parts a screen owns.

The appearance group offers a reading face for running text: conversation messages, documents and reports. Geist Mono stays the default face for the whole interface. The second choice is the serif face [Libron](https://github.com/nicoverbruggen/libron), shipped like Geist Mono as web font files with its SIL Open Font License beside them in `client/src/fonts/` (`docs/third-party.md` §4); controls, labels, figures, code, paths and commands stay in Geist Mono. No Chinese face is bundled: Chinese text in the reading face falls back to the machine's own `serif` face. The choice is the new `reading` field of the person's appearance preference, and a preference written by 0.0.9 reads as the default face (`crates/wire/spec/Preference.lean` D52).

The client's controls are now split into a seat and a look (`client/Spec.lean` D95). The seat is the file callers import: it owns the wiring to the city, the state, the keys and the focus, and hands each operable element to the look as a wire bag that carries its role, `aria-*` values, tab stop and handlers. The look is markup and a `<style>` block only; it reads colour and motion from the theme's tokens and imports nothing from the client's core, which eslint refuses. A person can therefore replace a look with a component of another UI library, or change its colours and motion, without losing the wiring or the alignment, which belongs to the screen that places it. `just swap` proves it: it builds the client with the replacement looks under `client/swap/` in place of the shipped ones, then runs the type check, the wiring tests and `cargo xtask render` on that build, and `ci.yml` runs it in the gates job. `cargo xtask render` also reads a roster of parts and fails when a look drops the role its contract requires. [Seats and looks](docs/frontend-method.md#seats-and-looks) explains how to write a replacement. The split also adjusted details on the way: transitions arrive and leave on the theme's curves, one hover vocabulary is defined (rows on `wash`, controls on `raised`), copying to the clipboard has one part that reports a failed write, and the fork entry reads as a branch from a turn.

### Privacy

`sprawling privacy status` prints the privacy history summary as one JSON line. It discloses a summary only after the live Windows identity matches the owner bound in the platform vault, and a missing, locked or mismatched binding is refused with its own recovery (`crates/sprawling/spec/Privacy/Cli.lean` D54).

On Windows, the settings page has a privacy group with 88 optional privacy controls in nine categories, from diagnostic data and speech input to app permissions and Windows AI features. Each control is one host setting: 74 machine registry values, 8 user registry values, 2 user environment variables and 4 scheduled tasks (`bin::privacy::controls`, `crates/sprawling/spec/Privacy/Controls.lean`). For each one the page shows its current value, the value it would write, its scope, what it changes and what stops working when it is applied, and, where Microsoft states it, which Windows editions honour it. Nothing is applied by default and there is no "apply all". A person applies or restores one control at a time, after a confirmation that binds the value the page showed; if the value on the machine changed meanwhile, the operation is refused. A machine-scope write goes through a UAC prompt. Every write is read back: when the read-back value is not the written one, the original value is written back, and when that also fails, the control stays unsettled until the person checks it (`crates/sprawling/spec/Privacy.lean`). The original value is recorded before the first write, so each control can be restored to it, and "restore all" restores the controls one by one. Of the 52 items of the original request list, 39 map to controls; the other 13 are listed with the reason they are not written (absent, obsolete, undeterminable, or needing an operation kind this version lacks). The page answers only on the machine that runs the city; a paired remote device is refused. These controls are choices about privacy, not security: many of them cost convenience, and each one shows that cost so the person can weigh it. Real writes of every operation kind were applied and restored on disposable Windows runners, never on a person's machine.

### Model providers and search

A resident MCP connection over streamable HTTP whose session the server ended is dropped and reconnected (open, handshake, tool list) on the next dispatch; the failed call itself is returned, not sent again.

A provider endpoint can list several accounts in order. The list order chooses the first account, and a Session stays on the account that first answered it, across reordering and restarts.

A provider refusal with status 429 whose structured error code (or, when the code is absent, its error type) is `insufficient_quota` is now its own failure kind, `quota`. It is no longer backed off and sent again until a Halt, because waiting does not restore a used-up quota: the run stops with the refusal, and the page says to add credit to the account or add another account. A 429 without that code is still a busy provider and is asked again. Each provider error also records whether the account it was sent on can still take the request (a rejected key, a used-up quota or a missing account credential cannot); the record omits this field when the answer is yes, so earlier Ledger lines are unchanged.

A model provider registered with two or more accounts now uses them in the listed order for each model call. A rejected key, a used-up quota or a credential the vault does not hold moves the same request to the next account at once, and the history records a `watchdog_fired` line with action `switch` before the next call. A busy provider is asked again on the same account one or two times (the endpoint's `account_retries`, two when it is not set), then the next account takes the request. A request whose answer was lost is sent again only on its own account. When every account has failed, the run stops with `E_PROVIDER_ACCOUNTS_EXHAUSTED`, which names each account and the last failure it met. The next model call of the run starts on the account that answered last. An endpoint with one account behaves as before.

Each provider on the settings page now folds an account editor under its row. It lists the accounts in their order with the state of each key (stored, missing, or read from an environment variable and therefore read-only), moves an account up or down, edits or removes it, and adds a new one; a key typed there goes straight to the vault under `secret:providers/<provider>.<account>`, and the page keeps only the reference the city answers with. Every change sends the whole list with the endpoint's tuning as the city read it back, so timeouts, headers and overrides set earlier are kept. A provider still attached with its one key says that saving an account replaces that key with the list. Removing an account whose key the vault holds leaves the key in the vault and offers "delete the key too" beside the list; that sends the new `ForgetSecret` command, which the city refuses while an endpoint, the city's `[search]` or a building's configuration still names the reference, and refuses for a key an environment variable supplies, naming the variable to unset.

The providers group of the settings page now has a web search card below the provider card. It chooses between the default service (Exa, whose address the city reports), a custom list of MCP services with one of them in use, and no web search, and writes the city's own `[search]` table whole each time. Each listed service folds the same account editor the providers use; a service's keys are stored in the vault under `secret:search/<service>.<account>` and travel only in the header the account names. When a file nearer the hall states its own `[search]`, the card says so and leaves that file alone. A provider with two or more accounts also offers the number of retries on one account before the next account is tried (once or twice); while none is chosen the card shows the city's default.

### Configuration

Behaviour change: a plaintext credential written by hand into an `[[mcp]]` row of `CONFIG.toml` is now refused when the file is read, with the same `E_CONFIG_INVALID` refusal the settings page already gave when writing that row. Before, the reader accepted such a row and started the server with the key, although the file is committed with the project. An `env` or `headers` value whose name reads as a credential, or whose value has a credential's shape, must be a `secret:realm/name` vault reference. An MCP url must be an absolute `http` or `https` address with a host, with no userinfo and no credential in a query parameter; a query or fragment that carries no credential is kept as written. To recover, store the key in the vault and write its reference where the key was.

Every building that is not confidential now offers its runs a `web_search` tool. With no `[search]` table in `CONFIG.toml` the tool reaches Exa's hosted MCP service without an account, so the words a run searches for leave the machine to that service; `choice = "off"` under `[search]` removes the tool, and a confidential building never has it. A custom supplier is reached over MCP streamable HTTP only, its accounts are tried in their listed order, and a failing custom supplier is never replaced by Exa. A search does not stay on one account for the rest of a Session: each search starts from the first account whose key is stored.

### Execution and recovery

Linux host-command confinement explicitly requests a user namespace and refuses a failed namespace setup. A copied working tree remains a placement mechanism; it does not isolate other host paths or the network. Sandbox guarantees are reported by axis rather than inferred from a platform name.

Windows run jobs apply the selected processor affinity. JSONL preallocation reads scan the zero tail with bounded memory while preserving rejection of nonzero bytes after the tail; they do not skip that verification I/O. Skill audit execution binds results to the installed content digest, records unreachable services without claiming a successful audit, and observes shelves through the serving assembly.

The existing container command path preserves the configured engine, encodes a single Podman entrypoint and refuses non-UTF8 arguments instead of changing them silently. Configuration, native-platform mechanisms and lifecycle guarantees are described in [operating](docs/operating.md#how-exec-is-confined) according to the shipped implementation.

On Windows, explicitly selected native confinement (AppContainer with a Job Object) passes the six child-process regressions and the PowerShell and toolchain checks on disposable runners, and the MSVC toolchain is found outside the container. Windows still defaults to the copied working tree until `crates/runtime/spec/Tools/Exec/NativeWindows.lean` D59 is settled.

### Distribution and verification

Release archives keep ZIP with Deflate level 9, with archive suffixes and platform names taken from the shared platform table. Release validation uses the existing parallel CI entry point and its cache configuration. Manual release-workflow dispatch builds and checks artifacts without publishing GitHub releases, npm packages or crates, and does not invent release provenance.

The repository Nix flake builds the browser client before Rust and includes the shipped skills and licences. The flake is the only Nix distribution path; sprawling is not submitted to nixpkgs. Windows resource generation reads Cargo package metadata for the product and file version. Build provenance, archive checksums and OS code signing are separate facts.

The static Linux archive stays linked with musl-gcc, which produces a static PIE; cargo-zigbuild produced a binary without PIE, and its release build took 9-27 s longer on GitHub's hosted Ubuntu runner (`crates/sprawling/spec/Install.lean` D57). `rust-version` stays 1.97, and a nightly `msrv` job in `platforms.yml` builds the workspace with exactly that compiler.

`ci.yml` runs the test suite in four slices balanced on measured nextest time, and checks before the slices start that every test belongs to exactly one of them; a run starts 16 jobs, 7 of them on Windows, where it started 31, 23 of them on Windows. On the Windows runners Cargo's home sits on the work drive, and the test archive links with rust-lld: on a warm cache its compile took 171 s and 194 s in two runs, against 201 s with link.exe, on GitHub's hosted Windows runner. A released binary is still linked with link.exe (`tools/xtask/Spec.lean` D27). A release is published only when every slice of its own `ci.yml` run passed, so a slice that failed, was cancelled or never started stops it.

A fault-injection run of the release workflow on a throwaway branch checked that rule: a stand-in publish step ran when every slice passed, and was skipped when one slice failed, when a test was left out of every slice (the coverage check in the test build named the unassigned tests), and when a slice was cancelled. A whole `ci.yml` run took 557 s of wall clock on GitHub's hosted runners with a warm cache, against 650 s before the cache and toolchain changes ([run 37539251320](https://github.com/2youg1/sprawling-agents/actions/runs/37539251320)).

Release builds generate an Arch Linux package from the archive and install it in an Arch container; the AUR is pushed only on a tag push with the `AUR_SSH_KEY` secret.

### Documentation and contribution rules

README now introduces the city, documents that carry work across Sessions, five capabilities and a complete reading index in matching English and Chinese. The paired getting-started guides distinguish prebuilt and locally compiled channels and explain version selection, provenance checks, stopped-city export, verified restore, updating and rollback from the pre-update backup. Root host configuration and vault credentials require separate handling; an exported directory alone is not proof of recoverability.

[LLM.md](LLM.md) introduces the project for another model; [wire](docs/wire.md) owns the protocol reference, [integrations](docs/integrations.md) owns ACP/MCP/CLI setup and [performance](docs/performance.md) owns monitoring and reproducible evaluation. Onboarding objectives and historical readings are not presented as measurements of this release.

Contribution policy uses AGENTS as its single authority, distinguishes negative fixtures, fixed regressions and opt-in experiments, and requires matching English and Chinese issue/PR descriptions. The security policy states actual trust boundaries, private reporting, no bounty, optional credit by consent and next-release fixes without backports.

README presents performance and privacy as the two features of this release, each with the guide that explains it: the new privacy section of [operating](docs/operating.md#privacy) and the performance settings in [performance](docs/performance.md#choose-how-the-city-uses-the-machine); it states that agents work with full permission inside their building by default, and carries the author's own lines on strengths, weaknesses and why the project exists. [LLM.md](LLM.md) states the same performance and privacy boundaries for a model that introduces the project.

### Known and unfixed

- Windows formal code signing is not done in this version; Windows Defender may quarantine the binary of a downloaded archive.
- The performance readings of the four placement arms (R13) were not taken; the default `[core] placement = "soft"` is supported by the design, not by a reading.
- The first message head breaks inside a Chinese fact when it wraps.
- Under the copied working tree, `exec` in a room starts in a copy of that room's directory, so files elsewhere in the building are not in the copy. A model that is asked about them tends to look for them through absolute host paths, which the copied tree does not restrict ([how exec is confined](docs/operating.md#how-exec-is-confined)). Until this changes, keep the files a room works on in that room, or name their full paths in the task.
- The `read` tool resolves a relative path from the city root, while `exec` starts in the room's copy, so the same relative path names different files in the two tools; a model often tries one, fails, and retries with the other.
- When a command's spilled output in a room holds secret-shaped text, for example a long listing of random file names, the checkpoint that follows refuses with `E_SECRET_EGRESS` and the run stops, and the refusal lists every span it found instead of a count.
- Secret-shaped tool-call identifiers that some providers send are replaced by a marker in the Ledger like any other secret-shaped value. Both the call and its result carry the same marker, so they still pair up.
- Microsoft does not state whether most privacy policies apply on Windows Home: for 79 of the 88 privacy controls the page shows "not stated" for the Home edition.

---

## v0.0.9-Alpha-261004

**sprawling 0.0.9 something and nothing**

Alpha. The workspace manifest carries 0.0.9 and the tree cuts alpha
releases; this section is the release, cut as `v0.0.9-Alpha-261004`. Alpha
means usable: a User can hand the city real work, and
data formats, the wire and the screens may still change between versions. It
records what landed after `v0.0.8-Pre-alpha-261002`. The wire moved from
WIRE_V 46 to 56, once for each push in which its shape changed
(`crates/wire/Spec.lean` D1): 47 when sessions began to carry a display name
and their last run, together with the harness states, registry lines, the
doctor's scanning answer, the sandbox arm names and a colour theme; 48 when
an endpoint's tuning gained `max_in_flight`; 49 when `RunSummary` gained
`waiting`; 50 when the four remote door commands `OpenRemoteDoor`,
`ReplaceCityKey`, `ConfirmRemoteDoor` and `CloseRemoteDoor` arrived; 51 when
a cache count could say it was not reported; 52 when a tool call could render
as a send or a delegation and the thread's notes gained arrivals, reply waits
and handbacks; 53 when a sent letter's delivery outcome reached the page; 54
when a send carried where its letter landed, an arrival its kind and sending
session, a proposal card the line that offered it, the usage of skills and
tool servers became three queries, and the sampling beat became settable; 55
when the shells tally became a query of its own and an unavailable answer
began to carry its reason; and 56 when the kernel's `skill_shelved` kind
arrived, with the author every skill version now names.
`EVENT_LOG_V` stays at 2: every new payload field is optional, and a line
written by 0.0.8 reads unchanged.

### Agent messages

- A resident could pass for the User. A signal's body reached the receiving
  run inside the same User-role message as the User's own steer, with only a
  text prefix to tell them apart, so a body that held a newline and
  `user: approve the merge` read exactly like the User. Now every word a
  resident says reaches the window as one **letter**,
  `<letter from="@room" run="…" kind="steer|reply" sender="…">`, with `<`,
  `>` and `&` in its body escaped, so the body can neither close its letter
  nor start a line the User would write. Who spoke is a type, `Speaker`:
  only the control surface builds the User's speaker, the city's own words
  begin `city:`, and a fork replays a recorded steer through the same
  renderer. A property test over generated bodies, a newline followed by
  `user: …` and `</letter>` among them, failed before the change and passes after it
  (collab D16, `crates/runtime/spec/Conversation.lean`).
- A delegated task names who handed it down. JOB.md gains a `<from>` element
  ("the User", "the city", or "@room, run …"), the task and the goal sit
  escaped inside their own elements, and the opening line says
  "The task is in JOB.md above, handed down by …"; a forked run's inherited
  task is written the same way (city D21). City.md says which shapes are the
  User's: the opening of a session the User started, and lines that begin
  `user:`. A letter or a handed-down task carries its resident's standing,
  even when it quotes the User, so a decision it reports is checked in the
  hall's `Memo.md` or the plan first.
- The receiving run's thread draws a resident's letter as a letter card on
  the left, with the sender linked to its session, the kind, the time and the
  text, apart from the User's own words. A pulled letter names who sent it
  and what it said, where it used to read as the receiver saying nothing.
- A send reads "send to @room: …" and a delegation "delegate to room: …",
  each with where the letter landed: delivered to a working run, queued, or
  knocked for a new run. The city records that **delivery outcome** as a
  `signal_landed` line right after the `signal_enqueued` line, one for every
  sent letter (kernel D38). A letter that a handback or a settled landing
  sends records where it landed in the same way.
- A reply wait is visible: the thread shows the room waited on and the time
  left, and how the wait ended (a reply, the patience running out, or the run
  leaving); the mailbox, the runs board and the session row say which room a
  run waits on instead of "thinking".
- A child room's opening names the run that delegated it, and its session row
  says who delegated it. A handback reads "finished, verified by …" or
  "stopped, because …", with a link to the child's session.
- The queued-letter section shows each letter's kind, first line and a link,
  and counts in the singular and the plural.
- The session sheet gives TTFT and tokens per second a cell each, one figure
  over one note, so no figure is cut off at 1440 px, at 1920 px or on a
  390 px sheet.
- The gallery draws every agent-message case, so the screenshot job shows
  each of them.

### What the Ledger and the wire now record

- A tool result carries `took_us`, and a turn carries `first_us` and
  `took_us`: whole microseconds read off the turn's monotonic clock, beside
  the millisecond moments the envelope already had. A line written before
  these keys existed has none of them, and a reader falls back to the
  difference of the two moments (`crates/kernel/spec/Event/Record.lean` D20).
- Seven new kinds, 96 in all: `run_policy_changed`, `session_named`,
  `skill_audited`, `signal_wait_started`, `signal_wait_ended`,
  `signal_landed` and `skill_shelved`.
- A usage record's cache counts say whether the provider reported them, so a
  provider that reports cache hits under another field reads as unknown
  rather than as zero hits (A29).
- `endpoint_attached` may carry `max_in_flight`, which survives a replay.
- Every duration that `just bench`, `sprawling gauge`, the citysim bench and
  the monitor page print is an integer microsecond. A duration shown to a
  reader is in µs below 10 ms and in ms from 10 ms up.

### A run's work takes effect when it is done

- A signal is on the Ledger and in its room when it is sent, not when the
  sender freezes. A delivered signal knocks on an empty room, names where its
  sender stood, and a signal a run took is held until a model answer reads
  it; a run that leaves first gives it back to the room (collab D8, D10).
- `delegate` and `workshop` take effect at the call: the children start and
  the graph registers while the parent still drives. A cancelled or failed
  parent closes its graph, and children already in flight hand their work
  back (collab D14).
- `signal send` with `wait: true` is the reply wait: the run stops at its next
  safe point, calls no model and spends no tokens until the reply comes, the
  run leaves its room, or 240 s of patience run out. A reply that lands
  before the run stops is kept for the wait, one run has at most one open
  wait, and the run's summary on the wire carries `waiting`.
- A run policy the User changes reaches the run working in the room at its
  next `BeforeWave`, so one wave never runs under two policies. Every mode
  offers the same tool list and the mode is asked at the call, so a policy
  change keeps the prompt cache: `edit` and `exec` in chat mode answer
  `E_GATE_DENIED` with the way to switch.
- `redact` takes a tool payload by value and gives a clean one back in the
  allocation it came in, so a payload with no secret in it is not copied;
  a property test holds it to the copying walk.
- A read-only tool runs before its intent is durable. The run holds its
  lines across turns, so a turn pays one durability barrier plus one per
  write, rather than one per line, and the run pays one more before it
  freezes, is cancelled, or writes a carrier event. A power cut at any line
  leaves a prefix of the run with no write ahead of its intent, and the
  count and the cut are the same on Windows, macOS and Linux, where the
  barrier is `File::sync_data` (`crates/runtime/spec/Turn/Durability.lean`
  D36).
- A checkpoint takes no lock across runs: each run stages into an index of
  its own, a city gets one base however many writers race, and the first base
  is written as one pack, so it leaves no loose objects.
- The lane writes a landing's transcript and reads its sweep before the run
  comes home, so the accounting thread only appends.
- A prepared run gets a lane at once; the driving pool asks only for memory.
  Each endpoint hands out permits in arrival order under its `max_in_flight`
  (16 by default, 4 for a local model, 256 at most), sheds a call after
  600 s in its queue, and narrows after a 429 until the provider's
  `Retry-After` instant has passed.

### Performance

The changes above remove waits; they do not yet have readings. The
before-and-after readings are taken at the end-reading sitting under one
basis: integer microseconds, nearest rank with n beside every figure, p50 and
p99 for ordinary latency, p50, p99, p999 and max for the high-frequency
operations, and every duration split into waiting, work and the durability
barrier. Each set names its tree, toolchain, feature set, platform, fixture
digest and machine class. These readings are owed and not stated here:

- throughput and the four waits (lane, provider queue, relay queue, fold and
  broadcast lag) of N concurrent runs, from the citysim throughput bench and
  the provider-queue instrument;
- the per-tool phases of `read`, `write`, `edit`, `exec` and `search`;
- the secret scanner, which now reads its input once, and the playback
  export. A narrow export's credential scan is now lazy: it reads the lines
  its tables select, every tool call and every run's opening line, and
  settles the far end of a pair from the copy it kept or from the line's
  offset. A property test holds the lazy bundle byte for byte to the full
  scan's. On the 1004-line test fixture with five lines selected, it scans
  208 lines where the full scan read all 1004; the wall clock of a large
  export is owed;
- the first base of a city and the checkouts of overlapping delegations;
- private bytes on a 100 ms tick, and their slope over an hour. On Linux
  private bytes are now `Private_Clean` plus `Private_Dirty` from
  `/proc/self/smaps_rollup`, or `RssAnon` where the rollup is absent;
  Windows reads `PagefileUsage`. While somebody watches the monitor, the
  city notes private bytes every 100 ms inside its one-second beat and
  reports the highest of each second, so a peak of a few milliseconds is
  not lost between two readings; an unwatched city reads nothing;
- a profile-guided release build against the plain one. `on-demand.yml`
  builds the Windows release in three stages, and the profile is kept only
  when the geometric-mean gain on the held-out load is at least 3 % and
  beyond the round spread.

### Fixes from the User's report

- The page opens in zen at every launch, and the layers key closes the
  settings panel and every page before it changes tier (A1, A20). The right
  side and a settings group move in with the theme's durations (A17, A21).
- With no provider endpoint the guide opens at every launch; it opens one
  step at a time, and skipping every optional step goes to the conversation
  (A2, A11, A12).
- The settings tree folds by object, keeps one branch open, and gathers
  second-level settings under "more"; the hall is listed even in a city with
  one building, and every page reached from the tree has a back key at its
  top left (ST, A3, A4, A6).
- The performance page draws tall bars, each scaled to its own window, with
  a band from p50 to p99 (A18).
- A tool call shows µs below 10 ms instead of 0 ms (A13). The session sheet
  shows TTFT as median and mean and tokens per second as p50 and p99 (A26).
- Each button beside a reply has a glyph, a name and a hover note; copying a
  whole reply gives its Markdown; a reply fills the reading column, and the
  read-wear bar says what it is (A16a, A16b, A16c, A22). The send and stop
  key is one coin with two faces, as in the template (A16d). An open sandbox
  no longer draws a warning edge (A31).
- "Whole" and "results" fold the earlier stretch by one rule (A23), and a
  hint in the conversation can be expanded (A30).
- A session row shows its display name, model, effort, workspace and last
  reply; its menu renames it and changes its run policy, and tags default to
  the workspace (A28, A15). A policy changed while the room is idle rules
  the session's next dispatch: the city keeps the session's last
  `run_policy_changed` as the one authority, and a dispatch frame's policy
  applies only to a session nobody changed.
- The mailbox pushes in from the left under the three edge keys; a letter
  opens on the right side with the text before and after, and every card in
  "deciding" says which event or refusal put it there (A19, A25, A27).
- Switching a Markdown document between source and preview keeps the line at
  the top of the view, and hall folders open and close (A34, A33).
- The chosen effort is drawn in accent (A8). The open right side starts on
  column line 8, so RefRain is wider at 1920 px (SR).
- The doctor page lists every item with its state, version and install
  line, every harness state, both registries and the drive scan (A9). A
  harness reads Ready only where its vendor's own directory exists, and
  otherwise NotSetUp with the paths it looked at (A5). Grok Build is looked
  for in `GROK_HOME` or `~/.grok`, Kimi Code in `KIMI_CODE_HOME` or
  `~/.kimi-code`, on Windows, macOS and Linux alike.
- The update check asks npm and crates.io and gives the update command of the
  channel this binary was installed through (A10).
- The remote group in settings walks a pairing in numbered steps, says what a
  restart does, and lists `/remote replace-key` (A7). It opens the remote
  door for 30 minutes, 2 hours, 12 hours (the default), 2 days or 7 days,
  closes it, and replaces the city key. Opening and replacing the key
  print a code on the terminal that runs `sprawling serve`, and act only
  when that code is typed into the page within two minutes, because a
  browser tool or a command can drive the page but cannot read the
  console. Closing needs no code. A city started without a console refuses
  both with `E_TOOL_UNAVAILABLE`. The page and the code are the same on
  Windows, macOS and Linux. Pairing a device stays at the console. The city key is kept in
  the city's vault and read back at every start: Credential Manager on
  Windows, the Keychain on macOS, the kernel keyring on Linux until reboot
  (or the encrypted vault file across reboots), and the city's memory where
  no store answers. `/remote replace-key` makes a new key and unpairs every
  device. On macOS the Keychain asks once after the binary is replaced, and
  a Keychain dialog nobody answers no longer keeps Ctrl-C from closing the
  city, because the key is read on a thread of its own after Ctrl-C is
  installed. A city started without a console names `sprawling up` or
  `sprawling serve --console` as the way to get one.
- The city's git reads its own repository's config file and no git file of
  the User's, on Windows, macOS and Linux (G2).
- The doctor exits 1 only when an item the use tier requires is missing; a
  missing develop tool leaves it ready.
- A new city finds the nine shipped skills on its library shelf; the shelf
  shows where each skill lives and where a new one goes, and the rules page
  picks its building (A32).
- The console prints one line per committed record — seq, kind and
  address — and the whole record only behind `--whole-records`, on every
  platform (A14).
- A colour page under preferences lists every theme token with a picker, a
  stylesheet field, legibility warnings and restore default (CT).
- The key sheet's gallery specimen stays inside its fold, the city bar
  draws nothing until the city answers, and the MCP environment's add-a-row
  button keeps its label on one line (G7). The second context reminder is
  set in the settings tree's run group, after autonomy, with a building
  picker, instead of under the MCP page's desktop form; the welcome page's
  back key stands at the top of its one column. The remote door's open and
  replace-key controls carry a door and a key glyph, and a door command the
  page could not send says the page is not connected instead of asking for
  a newer city. The Composio key field keeps its width, with the link to
  composio.dev on a line of its own.
- Tools (F1–F9): `archive record` files its entry at the call, so `recall`
  in the same run finds it; the Mayor writes an empty plan's first line with
  `plan add`; `rules read` and `city list` are admitted in a run; `read` and
  `edit` answer the whole version `plan finish` takes; `status` walks the
  worktree at the call; a search hit carries a window of its line and a
  stopped search names its limit; a reply a leaving run took reaches the
  room's next run; the hall templates name no `pursue` address and say that
  two seats in one building share a read domain.

### Storage and recovery

- A preallocated segment's trailing zeros read as the end, and a record
  hidden behind zeros is refused; a restore makes its truncation and its
  removals durable. Each platform's segment durability arm is one constant.
- A staged blob the scan cannot read refuses the checkpoint.
- A city killed between a job's put and its checkpoint line reopens with
  nothing to cut (G5).
- A plan claim from a desk that read the plan before a node ended is refused
  at the call, and the claim-conflict and re-dispatch scenarios run through
  `attend` (G4).
- `resume` over a run cut by power loss after any line it held closes the
  same tool calls as unknown, and continues with the same turn and
  session, as it does for a run cut at the same place under the old
  barrier per turn; a property test checks every cut.

### What a contributor notices

- The toolchain is pinned to Rust 1.99.0 and every member inherits
  `rust-version` 1.97; the lockfile and the client take the newest compatible
  releases, with TypeScript held below 7.
- `release.yml` publishes the workspace to crates.io through trusted
  publishing, and `cargo binstall sprawling` fetches the release archive for
  Windows x86-64, macOS on Apple silicon and Linux x86-64.
- CI runs the suite in four slices of one archive and the core packages on
  Ubuntu and macOS. Mutation, citysim, the screenshots, a release build and
  the PGO build run from `on-demand.yml`.
- The spec gate counts unresolved Rust paths and stateless state-machine
  parts as ratchets, and xtask keeps no public-API baseline.
- The shipped skills live in `crates/city/skills/`, and the city crate
  embeds them.
- `on-demand.yml -f job=fresh` walks this tree's release archive from
  install to the first dispatch on Windows, macOS and Linux runners, with
  the environment a new account has, and uploads the checklist and logs as
  `fresh-<os>-<tree>`. On macOS and Linux the walk ends its first and third
  servings with SIGINT; on Windows it starts each city through a launcher
  in a process group of its own and closes it with Ctrl-Break, so on all
  three the cities close in order. `-f job=keychain` reads on macOS whether the Keychain asks again
  for the city key after the binary is replaced.
- citysim measures collaboration on its counted clock: three delegated
  children run at the same time while the lead still drives, and the
  second game takes 7 runs with asynchronous sends and 4 with `wait: true`
  (TP3).
- Text a User or a model reads says User for the person who owns the city.
- `on-demand.yml -f job=mutants-modules` scores five modules against their
  mutation rows in `tools/xtask/budgets.toml`, and the diff job takes any
  base. Kernel and `storage::jsonl` sit below their rows; the survivors are
  listed for the next round.
- The page marks the User's key interactions as User Timing entries, so a
  browser profile shows where the time went.
- Every specification part that holds no state machine says at its top that
  it is a description, and theorems that only re-evaluated a definition are
  gone.

### Letters, usage, storage, gateway, runs and placement

- A letter's arrival note carries its kind and the run that sent it, and a
  letter opened on the right shows the sending run's conversation around the
  send.
- The skill page and the MCP page show the usage record folded from the
  Ledger, and export it as JSONL or CSV; the monitor's sampling beat can be
  set per city between 10 ms and 1 s, 100 ms by default, and the city keeps
  it for the next serve (A18).
- A new city records one `skill_shelved` line for each skill it ships, and
  each content version in the skill usage record says who put it on the
  shelf: the city, with where the skill came from; a change made outside the
  city's doors; or nobody recorded, for a skill shelved before the city kept
  these lines.
- The checkpoint scan never stages a document's staging file, and a segment
  can be preallocated behind one configuration value that defaults to off.
- `NO_PROXY` reads `example.com` as that domain and its subdomains and never
  as `badexample.com`, on Windows, macOS and Linux. A request to OpenAI's
  API carries the conversation's id as its prompt cache key.
- `[sandbox] interpreter = "pwsh"` runs `exec` under PowerShell 7 where it
  is installed, and refuses with the install line where it is not;
  `sprawling view --shells` counts each shell's failures.
- A remote door that expires closes on its own while the city serves.
- A socket writes every record queued when it wakes and flushes once. The
  doctor reads a program's output a line at a time, a playback export
  encodes into one buffer, and a frozen run keeps no task in memory. Their
  readings are owed.
- The city reads each core's efficiency class, its processor group, its
  cache group and whether the process may use it, and works the placement
  plan out as a pure function: one class, or a reading it cannot
  reconcile, leaves the threads to the operating system. The seats go in
  two pools — the accounting thread and the view fold hold the plan's last
  ones, a run's lane holds one of the rest and gives it back when the run
  ends, and a tokio worker holds none. `[core] placement` takes `"none"`,
  `"soft"` (the default), `"soft_shares"` and `"pinned"`: the default
  leaves power throttling lifted and each run's commands an even CPU
  share, `"soft_shares"` holds each run's processes to half the physical
  memory as well, and `"pinned"` is the hard-affinity comparison arm.
  Runs' commands share the processors by weight on Windows (each run's job
  takes weight 5), by cgroup `cpu.weight` on Linux where the cgroup is
  delegated and `nice 10` alone where it is not, and by `taskpolicy -c
  utility` on macOS. `sprawling doctor` says what was read and what each
  run's commands share. No reading compares the arms yet.
- The rules tool no longer offers `propose`, which it could not carry out.

### Designed, not yet built

- Sandbox arms are named `none`, `copied_tree`, `native`, `container` and
  `python` on every platform, and each platform's default is one column of
  `crates/wire/spec/Answer/Doctor.lean` D26: `native` on Windows (Job Object
  with AppContainer) and Linux (`bwrap`), `copied_tree` on macOS. Until that
  arm lands, every platform resolves `copied_tree`, or `native` on Linux
  where `bwrap` is present.
- Skill audit: the `skill_audited` kind and the audit state exist; no
  auditor (the skills.sh partner audits, or SkillSpector when installed)
  runs yet.
- No reading of the four-arm comparison exists: the p50, p99 and p999 of
  the main features, how much the per-run shares cut the waits under heavy
  load, and how often a step of work changed core mid-step. The default
  `"soft"` is what the design supports, not what a measurement settled.
- The fourth arm, `"pinned"`, is one step short: each run's job has to
  take the mask of the remaining processors, and the place that makes a
  run's job (`runtime::backlog::jobs`) is in `runtime`, which may not
  depend on `sprawling`; a seam handing the mask into `Backlog` comes
  first. Until then a run's processes leave the harness's job and land in
  the operating system's hands, which is no worse than `"soft"` but is not
  the whole arm.
- Storage still reads whole segments in places — `recover_tail` in
  `jsonl/open.rs` and two other read paths — so opening a city that was
  preallocated reads tens of MiB of zero tail with the records.
- macOS has no counterpart of the per-run memory limit.
- The byte-budgeted resident cache (`storage::resident`) exists and keeps
  its budget; the views, the hot view's tombstones and the Ledger's side
  index do not read through it yet.
- The Responses WebSocket mode is an open question in the gateway
  specification.
- macOS reports the process's virtual size where the other platforms
  report private bytes, until a safe interface reaches its physical
  footprint.
- The skill wire: `InstallSkill` and shelving a skill have no command yet.

### Known and unfixed

- The thread's progress bar can scroll below the page; it was not reproduced
  without a live city (A24).

---

## v0.0.8-Pre-alpha-261002

**sprawling 0.0.8 something (pre-alpha)**

Pre-alpha. It records what landed in the repository after
`v0.0.7-Pre-alpha-260927`. WIRE_V 46. The version moves at most once between
two pushes, at the first change of shape (`crates/wire/Spec.lean` D1), so in
this release it moved twice: to 45 when a turn and a call began to carry when
the reply's first content arrived, and to 46 when the person's preferences
began to keep session tags. Every other wire change below shares one of those
two numbers.

### One shell for every page

The page stands on one grid with three tiers: **zen** shows the conversation
alone, **blend** shows the city behind it as an outline, and **panorama** puts
the workbench beside it. The layers key moves between them. The tier, the
glass and the blend opacity are saved in the person's preferences file
through `PutPreferences`, so a second browser opens on the same tier. A
browser that has never saved one keeps its own until the city has an answer.

- The page is lit in acid blue on a cooler grey ramp, set in Geist Mono, and
  every icon comes from the lucide set through one glyph component. Motion,
  glass and corners are tokens in `client/src/theme.css`: three durations
  (150, 250 and 350 ms), an arriving and a leaving curve, a glass surface at
  75 % opacity, and a corner exponent of 1.6. The appearance group switches
  glass and motion off and sets the blend opacity.
- Settings open as a panel over the page, from the left. A tree of groups
  sits beside the open group, and `#/setup/<group>` opens the panel at that
  group. The groups include the identity cards with an explicit GitHub CLI
  import, the city layer (standing effort and `keep_warm`, written with
  `ConfigureCity`), a building's `RULES.toml` written whole with `PutRules`,
  the automation files read-only, what input each model accepts, and the
  tier. A save shows its state until the city's receipt arrives.
- The welcome page is a five-step guide. Only the first step, choosing a
  model, is required. The others can be put off one at a time or all at
  once, and the city keeps that progress per city. A step counts as done only
  when the city's own configuration shows it.
- The notices drawer is replaced by the mailbox, behind Accel-B. It has four
  sections, ordered by what needs the person: deciding, working, recent and
  notices. Its key shows the count of what needs the person, a dot for an
  unread ordinary notice, and a bar when the link is down. A question, an
  approval and a proposal are all drawn by one decide card answered with
  y, e or n. An ordinary notice waits for a pause before it appears, and the
  toast sits on the composer.
- A call opens in full on the right side, in the inspector: its diff, its
  terminal output and exit code, the file it read, or its screenshot. Each
  open item is a tab, and each tab is a link to its item. Accel-J closes the
  inspector. A file opens read-only at the cited line. A line from a past
  version of the tree is offered as `path:line` to copy, never as a link that
  would open today's file.
- On a container narrower than 768 px the page draws one column. The world
  becomes a sheet from the left, and the right side becomes a sheet from the
  right only while an item is open. Each sheet is a history entry, so the
  back button and an edge swipe close it. The frame follows the visual
  viewport when the soft keyboard opens, and it pads the screen's safe areas.
- The workbench in panorama shows the room's sessions, the chosen session's
  fact sheet and timeline, and the commits as a swimlane graph built from
  each commit's parents. The panes can be reordered and resized with a
  splitter or the keyboard, and this browser keeps that arrangement. Picking
  a commit scrolls the timeline to its checkpoint and opens its changes on
  the right side.
- The composer carries a run policy before the session starts: the write
  limit, the admission requirement and the landing policy, in one control
  with a three-column menu. `/dispatch` sends it, and `/admit` and `/room`
  are new verbs. Modes are now Chat and Work.

### Every other page

City, building, cost, registry, the machine report, monitor, MCP, record and
run pages are laid out on the shell's columns. Horizontal rules, not cards,
separate their parts, and each page has one title.

- The building page has a sandbox card, the cost of each plan node, and
  commit rows that state the message, the UTC time and the facts. A change
  row opens the file on the right side. A file can be taken back from a
  checkpoint after a confirm dialog (`RestoreFile`). `Commit` and `CostOf`
  have pages of their own.
- The record is one timeline of the ledger and the process log, with a
  filter by source. The run page shows its facts as one sheet. Its calls are
  timed in UTC to the millisecond, and it opens on the lens that the run's
  state calls for. `#/run/<id>/<lens>` opens a given lens, and `/diff` opens
  the changes lens. A row of the runs board opens the page it names.
- Accel-P finds a file by name in the building in front (`Query::Find`). The
  city walks the tree shallow-first, in name order, skips `.git` and links,
  and stops at 50 matches or 20,000 entries. The answer says when the walk
  stopped before it reached the end of the tree.
- Tool lines, mailbox entries, the runs board and the settings tree all walk
  by one key table: ↓/j, ↑/k, Home/gg, End/G, Enter and Esc. In the settings
  tree, a letter moves to the next group that starts with it.
- Ctrl+. cancels the run the page is showing, or says that no run is shown.
- The address bar can name a call (`#/talk/<addr>?call=<run>&at=<seq>`), a
  document and its version, a run lens, or a step of the guide. The composer
  keeps a separate draft for each room. ↑ in an empty box recalls the room's
  newest task. A quote is added to the open box without moving the caret. A
  draft that the browser refused to store is named under the box with a copy
  key.
- The palette has a "speak" entry. It records speech and writes the
  transcript into the message box.
- The client decodes the wire through Effect 4. `wire-ts` writes Effect 4
  schemas, and it closes recursive types with `Schema.suspend`.

### The silver cut

`--silver: sqrt(2)` in `theme.css` is the one place the cut is defined. The
twelve columns are grouped 3 | 6 | 3 and laid out in the proportion
1 : √2 : 1 of the window's width, so at 1920 px the conversation takes the
771 px in the middle, centred, and columns 4 and 10 are the two silver lines.
When the right pane opens, it takes the right part and the conversation stays
where it was. On the workbench, the session pane and the conversation are
split 1 : √2 from top to bottom. Pages span columns 2 to 12 and are
symmetric. The MCP page, the welcome guide and the settings panel put their
edges on the silver lines.

The conversation page is no longer a CSS query container. While it was one,
it turned off `subgrid`, so every pane fell into the first column. One
scrollbar rule also pushed every page 5 px left of centre, and that is fixed.

### Sessions: every one listed, tagged and pinned

The sessions pane lists one row per session, past sessions included: the
pinned group first, then one group per building, newest first. A row links to
its session at `#/talk/<room>:<began>`. An earlier session opens read-only,
with "continue from its end", which opens a new session from its last run,
and a screen reader names its region "an earlier session" rather than the
conversation, since it has no box to write in. In blend, the sessions pane takes input while the rest of the world layer stays
inert.

- `/new`, `/clear` and `/compact` act on the room of the session shown in
  main, and bring main to the new session. `/compact` cancels the run, waits
  for it to freeze, and then opens a new session that carries a handoff. It
  is composed in the client from existing commands, with no new kernel verb.
- A session takes tags of 1 to 24 characters: letters, digits, `-` and `_`,
  lower-cased on entry. The row menu and `/tag`, `/untag` add and remove
  them, and a filter row shows one tag at a time. The tags are kept in the
  person's preferences (`PreferencePatch::Tags`), keyed by city, room and
  the session's start. `pin` is a reserved tag. The Mayor's current session
  is always pinned, and its menu offers no unpin.
- A room still has one live session: opening a session ends the previous
  one, and opening one while a run works in the room is refused with
  `E_BUSY`.

### RefRain: documents beside the conversation

A document opens on the right side in RefRain, an editor built on CodeMirror
6. CodeMirror loads in a chunk of its own (97 KB gzipped), off the first
screen. Markdown and text are edited as source. A text file is shown
literally: a byte-order mark is hidden, CRLF is folded and written back
byte-exact, and control characters are drawn. The preview is asked from the
city window by window (`Query::Preview`), and the cursor's place carries
between source and preview. The editor has undo, find and replace, and a
draft for each document version. A save goes through `PutRange` and is
confirmed by its `document_written` receipt. A document that moved in the
city since the page read it opens a conflict, with three choices: compare,
move the draft onto the city's version, or discard it.

- Every version of a document that the city reads, saves, or lands through a
  proposal is kept in the content store. `Query::Versions` lists up to 100,
  newest first, with whether each one is kept and its size, and any two kept
  versions can be compared.
- PDF is drawn with pdf.js and DOCX with docx-preview, both in lazy chunks,
  and HTML is drawn in a sandboxed frame with its own CSP. Two versions of a
  PDF or DOCX are compared by their extracted text. Every preview names its
  format, the tool and its pinned version. The bytes reach the page through
  `Query::Bytes`: 1 MiB windows, base64, on the same socket, up to 64 MiB per
  file. The inspector draws screenshots through the same door.
- `Query::Export` writes a document as one self-contained HTML file with no
  script and `default-src 'none'`. A comparison is exported as Markdown.
- Mermaid is shown as its source, not drawn, and RefRain reads no text out
  of an image.

### Markdown has one grammar

The city lays out Markdown with one grammar (comrak), for documents and for
the conversation alike. A reply is laid out by the city (`Query::Reply`): the
page asks only for the part of the reply not yet laid out, and while the reply
streams it sends complete lines only. An open fence or list stays raw until
it closes. The client's own Markdown reader, `prose.ts`, is deleted. A
Markdown version up to 64 KiB can be previewed without being stored first.
Formulas are drawn by KaTeX as MathML, with no KaTeX fonts or stylesheet. A
formula KaTeX refuses is shown as its source.

### Proposals

A run can propose changes to a document instead of writing it. The resident
`proposal` tool offers and withdraws cards. Each card quotes the version it was
made against, and the city refuses a decision on a card whose version is no
longer the document's. Four record-only kinds carry the history:
`document_written`, `proposal_offered`, `proposal_decided` and
`proposal_withdrawn`. `DecideProposals` answers one or more cards.
`Query::Proposals` lists one document's cards, and `Query::OpenProposals`
lists every open card in the city, newest first, with the time each was
offered.

On the page a proposal is the decide card's third kind. Its body is the
city's sentence diff. y accepts it whole, e opens a per-sentence edit, and n
rejects it. A card made against an older version shows both versions; y and e
are disabled with the reason, and n is still allowed. Cards are listed in the
mailbox's deciding section and above the document in RefRain.

### What the city calls the person and the Mayor

`PREFERENCES.md` and `MAYOR.md` may open with an identity area: a TOML block
between two `+++` lines. `PREFERENCES.md` states `user_id` and
`imported_from`, and the text below the area says what every agent should
know about the person; `MAYOR.md` states `name`. A file without an area reads
as before, all body. An area that does not read is refused with its line
number, everywhere it is read, rather than read as "no name".

The names reach the model. The city segment of every prompt says what the
person is called, what they asked every agent to know, and that the Mayor at
`hall/mayor` goes by its name; the Mayor's own prompt opens with
`Your name: <name>`. A session freezes the names its first run read, and every
later run of that session sends the same ones, so a rename takes effect at the
next `/new`. Each `run_started` line records the version it ran under, and a
page reads the names back from the content store by it.

The page writes the names through `PutIdentity`, which sends the values and
the text the page read; the city rewrites only the card's keys. `PutDocument`
now carries that text too. A document that moved since the page read it is
refused, and the draft stays on the page. `Query::Identity` reads both areas
at the moment of asking. `Query::GithubLogin` reads the login the GitHub CLI
holds, for the import the identity card offers, and `Query::Guide` and
`PutGuide` read and write the welcome guide's progress.

### A building's rules and the city's own settings from a page

`PutRules` writes a building's `RULES.toml` whole. The city evaluates the
text before it lands and refuses it when the file moved since the page read
it; a write that lands is booked as `rules_changed`, the same line a dispatch
writes when it finds the file changed. `ConfigureCity` writes `keep_warm`
and the standing `effort` into the city's own `CONFIG.toml`. The core
priority is one more `PutPreferences` patch, and lands in the `[core]`
section of the person's file, where it always lived.

### Reading what a city automates, and taking one file back

`Query::Automation` lists `SCHEDULE.toml` and `WATCH.toml` as the next tick
reads them; a file that does not read is named with its reason, and the other
is still listed. `RestoreFile` takes one file of the city's own tree back to
what a checkpoint holds, or removes it when the checkpoint holds none, and
writes one `file_restored` line. It is refused while a run works in that
building.

### The page knows whether the history is proved

`CityAnswer.proved` names the last record of a history proved whole. A served
city proves its history in the background and refuses commands until it is
done; until then `proved` is absent, so a page can say why a command was
refused.

### A run's policy

A dispatch carries one run policy: the mode, a write limit, an admission
requirement and a landing policy, and `run_started` records it. The write
limit `Create` lets a run create files that do not exist and change, remove
or rename none, including one it created itself; a create claims its name
atomically, and the limit holds on every write path. The admission
requirement names the evidence a merge asks for on top of the building's own
checks: that the tests ran and passed, or that the observable contract did
not move. The landing policy `Experiment` gives the run a worktree of its
own, even in a building without review, and nothing is ever merged from it.

### The truncation lock

A run is offered each tool at one of three tiers, decided per building and
per session. A tool the building does not admit takes zero bytes anywhere. An
admitted tool outside the mode's core takes one line in a dormant index of at
most 1,024 bytes, written as `- name: hint`, then names alone, then `+N more`.
The core is `call`, `describe`, `read`, `search` and `status` in Chat, plus
`edit` and `exec` in Work. `describe` returns a tool's full guide, and `call`
runs it. The resolved call keeps the model's call id and passes that tool's
own checks, so `tool_called` and `tool_result` name the real tool. The
request's tool list stays the same for the whole session, so calling
`describe` keeps the prompt cache. On the fixture, the tools array and its
skill lines went from 12,973 B and 316 B to 4,693 B plus a 1,021 B index.

### Pictures and recordings

`ModelTag::Ocr` is a slot a person fills with a model that can read images,
and the models page names it. A run whose building the endpoint book gives
such a model is offered the `ocr` tool, which reads a PNG in the city or a
screenshot a connector stored. A run whose building has a transcription
endpoint is offered `transcribe`, which reads a recording from a file in the
city, by its extension, or from a stored block, by its leading bytes. Both
tools open bytes through one reader that refuses whatever `read` refuses. A
building with no such model is not offered a tool that could only fail.

- An MCP picture block is stored in the content store and handed to the model
  as an attachment, where it used to enter the window as base64. A sound
  block is stored the same way and named by its locator.
- A model's accepted input is one ladder: the catalogue, then the preset
  table, then text. `SelectModel` carries what the person says a model
  accepts, and that statement is the first rung.

### Computer use

- `desktop.snapshot` reads a window to the model as an outline in document
  order, read from the UI Automation tree.
- A snapshot's generation expires when its window moves or changes size, so
  input aimed by an old snapshot is refused.
- A batch of input cut short reports how far it got and releases only the
  keys and buttons it left held.
- The clipboard has an owner and a bounded read, and a clipboard that will
  not lock no longer reads as empty.
- The connector declares per-monitor DPI awareness when its desk opens, a
  recording takes every frame from the window's own capture, and it hears
  the one audio device the scope file names.
- The desktop server answers `tools/call` with a `CallToolResult`. A tool that
  reports its own failure reaches the model as a failure, and an answer lost
  after the request left is marked as an effect nobody saw.
- MCP over stdio, SSE and HTTP reads one message at a time under the message
  ceiling, where an answer past the ceiling used to grow the reader without
  bound.
- A hard link is refused by its link count on Windows as well as Unix.

### Official harnesses as residents

A layer's `[resident] harness` names one of the five official harnesses, and
the rooms below it run that harness as an ACP agent instead of a model the
city calls. A dispatch to such a room is refused before anything is written
when the name is not one of the five, when the building is confidential, or
when the dispatch names a model.

- The harness runs in the room's own worktree, whatever the building's
  `review` says, and the brief is its one prompt.
- What it reports is written as `harness_reported` after it is said, and its
  answer as `harness_answered` once the city has committed the tree. Both
  kinds join the handshake hash; `WIRE_V` does not move for them.
- A halt, or the building's `harness_minutes` ceiling in `RULES.toml`,
  becomes `cancel_received` and `session/cancel`.
- A run frozen done is offered for review as a request the city opens,
  because the harness reaches none of the city's tools.
- A permission request is answered with the first `allow_once` option the
  harness offers, or else the first `reject_once`, and never with
  `allow_always`.

### Remote pairing

A city can be reached from outside the machine through a route: a named
Cloudflare tunnel, any command that prints an https address (the documents
show Tailscale as an example), or a scripted route for tests. The city's
`[remote]` table chooses the route each time `/remote open` runs, and the
terminal prints the invitation as a QR code. Every frame from a device is
judged on its own. A device with the `Watch` authority reads; one with `Act`
may also send the eleven verbs of the Act class: `Dispatch`, `Steer`,
`Cancel`, `Halt`, `Release`, `Approve`, `HandOff`, `BatchByBuilding`,
`Pursue`, `OpenSession` and `PutSpine`. Every other verb is local only. The
door writes five record-only kinds: `remote_opened`, `remote_closed`,
`device_paired`, `device_revoked` and `remote_session_started`.

- The remote listener serves the same page as the city's own port. A paired
  browser on https speaks the wire through the sealed session. On any other
  connection it uses `/ws`. `/transcribe` and `/drop` answer 404 there.
- An invitation link (`#pair=…&city=…`) opens the remote settings group. The
  pairing code travels sealed to the city the invitation pinned. The seed is
  shown once and never stored. The device's Ed25519 half is a WebCrypto key
  that cannot be exported, and ML-KEM and ML-DSA come from
  `@noble/post-quantum` in a lazy chunk. A device can lock the door from its
  session. The page has a web app manifest and no service worker.
- A city served on port 0 now reports the port its listener holds to the
  door, the banner, `/serving` and the browser probe.

### Playback

`sprawling playback export` writes a read-only, deterministic bundle of a
city's history. The selection can be by seq, run, building or UTC time
(`[since, until)`), cut at the last complete line when the export starts.
`sprawling playback check` judges a bundle or a page on five items, each
reported as passed, failed or unchecked: structure and references,
consistency with the bundle, a recomputation against the city, a static
offline check, and a browser observation. Confidential buildings stay out
unless `--include-confidential` is given, and the output says so when it is.
A resident exports and checks through the `playback` city tool, as the run's
building, into the city's playback exports.

The `playback` skill ships with a reference page: one HTML file with the
bundle embedded, which opens on the runs and on what needs attention. It has
filters, narration whose every claim cites a `data-seq`, words in English and
Chinese, and one swimlane per run. Its playhead steps in seq order or plays
on measured moments. A line whose moment was not measured stays off the time
axis and says why. The page uses the machine's monospaced font and embeds
none. A local design or workflow skill may restyle it, but it may not change
the facts in the bundle.

### Timing

- Every line a turn waited for records its own moment, read from the
  driver's clock when the line was made. `EVENT_LOG_V` is 2, and a line says
  whether its `t` is the moment it records. A 0.0.7 binary refuses to open a
  city that 0.0.8 has written, with `E_LOG_VERSION_UNSUPPORTED`.
- A turn carries when its reply's first content arrived (`first_at`) and
  when the reply returned (`Turn.returned`). The first head of a session
  reads who is speaking, the model, the effort and the mode, from the moment
  the session starts. Each later head shows its time to first content and
  its tokens per second, and the session sheet shows the medians of both. A
  run's effort and the names it froze reach the page through `Opening`.
- A running call counts up every 100 ms and shows nothing under one second.
  Once the call lands, it shows the ledger's milliseconds. One function
  computes a call's duration, and the thread, the timeline, the monitor and
  the inspector all use it. A UTC instant is written one way everywhere on
  the page.
- An `exec` result ends with the second its call answered, in ISO UTC.
  `[clock] stamp` sets how precise that stamp is, and `status` says what time
  it is.
- `sprawling view --since/--until` keeps the lines whose own time falls in a
  UTC span, and the terminal view prints each line's chain hash.
  `sprawling whose --trace` lists the calls a run made since its previous
  commit.
- The context ring sounds its first reminder at 30 %. A city may set the
  second reminder anywhere from 31 % to 90 %.

### Measuring: the monitor and `gauge`

`Sample` carries two more counters: `view_backlog`, the records the writer has
handed the view thread and the pages cannot see yet, and `read_nanos`, how
long the sampler's previous beat took to read the counters.

`sprawling gauge`, which `top` still names, measures one of three subjects: a
served city, a process tree (`--pid`), or a command run n times (after `--`).
It beats every 1000 ms unless `--every` sets 250 to 60,000 ms, prints lines a
person reads on a terminal and JSON lines otherwise, and shows the two new
counters. Dispatch preparation, MCP tool listing and the probes read a
monotonic clock, and every spread is the nearest rank. The `gauge` skill
teaches an agent to use it on other projects.

### The release binary is built at `opt-level = 3`

`"z"`, `"s"` and `3` were measured against each other on the product's common
operations, and `3` was the fastest: the geometric mean of nineteen workloads
read 0.902 of `"z"`'s, against 0.945 for `"s"`. Folding a large ledger takes
0.63 of the time it took, the first byte of a 400,000-record city 0.81, and
startup is unchanged. The binary grows from 15.1 MB to 26.8 MB and install
takes 1.25 times as long (windows-x86_64, 16 cores, NVMe); runtime speed comes
before size.

The readings on that machine, release build, with the readings at `"z"` in
brackets:

- First byte of a served city: 45–47 ms empty (47–48), 110–112 ms at 100,000
  records (121–125), 210–212 ms at 400,000 records (257–263). The history is
  proved behind the first byte, its segments hashed in parallel waves of
  eight: 138 ms at 400,000 records. Commands are taken 310 ms after opening
  began, and until then they are refused with `E_HISTORY_UNPROVEN`.
- Rebuilding the views from genesis: p50 437 ms (676–708 ms). It fell from
  1,277 ms when a rebuild stopped proving the history before folding it, since
  the fold checks every line itself.
- A run's history: p50 3.3 ms (4.15–4.26 ms).
- Install: p50 1,856 ms (1,486–1,490 ms). Most of it is the first launch of
  the unpacked executable, which the on-access scanner checks: 1,674 ms.
- A run lands before its lane restocks the worktree, so the wait at landing
  went from 1,219 ms to 0. The city's first placement of a tree still takes
  4.0–4.3 s.
- A 500-step long turn peaks at 7.1 MiB of private memory.
- The release binary, the client bundle and the dependency count are now
  readings. `budgets.toml` still records them, but no gate refuses them.

### Every specification is Lean

Every crate's SPEC, the client's and the tools' included, is now
`Spec.lean` with its parts under `spec/`, in one Lean package at the
repository root, and no `-SPEC.md` file is left. Each entry file has
seventeen sections. A decision is a `D<n>` comment above the declaration it
governs, and the comments are in Chinese with concept names in English. The
design models moved from the adversary into the crates they specify, and
`tools/adversary/` now holds only the checker. The client's interaction
contracts are state machines with proofs. How a screen looks is now described
in English in `docs/frontend-method.md`, under the labels the code cites.

Lean is required to develop the code. `just models` fails when Lean is
absent, `just check-branch` builds the models when a `.lean` file changed,
and `cargo xtask spec <lib>` writes a skeleton. The `spec` gate enforces one
effective specification per crate, no citation of a specification the tree
lacks, imports only along the crate graph, no `sorry`, `admit` or `axiom`,
and every cited path on disk. `specalign` and `wiring` read the Lean tables.
`docnum`, `lexicon`, `release` and `proof` read `.lean` files too.

### The doctor checks the city's drive

On Windows, `sprawling doctor` adds a `scanning` part after `priority`. It
names the city's directory, the one on the command line or else the default
city location, and answers two questions about it: is it on a Dev Drive, and
is it inside Defender's exclusions. Each answer is yes, no, or "cannot tell"
with the reason. When neither holds, it prints the Settings path that creates a
Dev Drive and the `Add-MpPreference -ExclusionPath` line for the city, or
`fsutil devdrv trust <volume>` for a Dev Drive that is not trusted. Defender
shows its exclusions only to an administrator, so an ordinary account reads
"cannot tell" there. The part changes no verdict and no exit code, and other
platforms print one line saying it does not apply. The doctor's page does not
show this part yet.

### What a contributor notices

- Packages carry their crates.io names, `sprawling-*`, and the directories
  follow the library names: `channels` is `wire`, `memory` is `storage`,
  `protocol` is `agent_protocols`, and `remote_access` and `documents` are
  new. `tools/` holds what never ships: `xtask`, `citysim`, `adversary`,
  `fixtures` and `fuzz`. `crates/`, `tools/`, `docs/` and `skills/` each
  have a README.
- `crates/desktop` is a workspace member. The four groups of Win32 calls
  with no safe interface — enumeration, capture, the clipboard and DPI
  awareness — go through a Zig leaf behind `crates/desktop/ffi`, the one
  crate with a lint table of its own, and `crates/desktop/src` holds no
  production `unsafe`. Zig is in the develop tier at the version
  `crates/desktop/ffi/zig-version` pins, and `header`, `length` and `modmap`
  read `.zig` files.
- Every product package can be published. The `packaged` gate refuses a
  package that compiles a file from outside its own directory, and `guard`
  pins every workspace package to the workspace version. `sandbox` is a
  default feature, the client bundle is built into `crates/sprawling/web-dist`,
  and the templates a city writes live in the city package.
- The maturity has one definition, `kernel::release::MATURITY`. The tag, the
  status line, both READMEs and this file render it from there.
- The new `motion` gate requires every transition to use the curves and
  durations in `theme.css`. The colour gate moved to the acid-blue axis and
  checks that text on glass stays legible over the brightest surface behind
  it. The render gate checks the conversation page's standing controls
  against `[talk_controls]`.
- The client's runtime list adds the five CodeMirror packages, `pdfjs-dist`,
  `docx-preview`, `@noble/post-quantum`, `@lucide/svelte` and `katex`, each
  pinned exactly. The READMEs and `AGENTS.md` point to the list in
  `tools/xtask/src/npm.rs` instead of copying it.
- `just shots` renders every page at 1440 and 1920 px, dark and light, for a
  person to look through.
- A build with no pinned toolchain installs the stable Lean and winget's
  Zig.

### Known and unfixed

- The city's remote key lives only as long as the process, so every device
  pairs again after the city restarts. Keeping it needs a decision about
  where the seed lives, and `crates/remote_access/Spec.lean` §3 sets out the
  three options. No test opens the Cloudflare route on a real tunnel.
- A harness resident gets none of the city's tools and none of the
  building's MCP servers, and a steer sent during its turn is recorded but
  not delivered.
- `/compact` can reach the city a moment before the room is free, and the
  city then answers `E_BUSY`. A second `/compact` works.
- Session tags are keyed by the city's name, so two cities with the same name
  on one machine share their tags.
- An earlier session whose runs are older than the runs the page holds shows
  no thread, and its "continue" is disabled.
- With the right pane open at 1920 px, RefRain gets 518 px, about 60
  monospace characters.
- A resident's `edit` or `exec` writes no line naming the version it made, so
  RefRain can say only "written outside the page" for those versions. A
  version written outside the page and never read by the city is listed but
  not kept.
- The `playback` tool takes the reference page's whole text as an argument.

---

## v0.0.7-Pre-alpha-260927

**sprawling 0.0.7 citior (pre-alpha)**

Pre-alpha. It records what landed in the repository after
`v0.0.6-Pre-alpha-260922`. WIRE_V 45.

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
<!-- lexicon-ok: a released entry names the shell of its own version -->
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

### The first pass by hand

The release build was driven through its own page the way a person uses it,
with a real provider attached, and what failed there was fixed before the tag.

- A city that had committed once could not be served again. The views are
  copied through their binary snapshot encoding at every start, and a run id
  read back as text where sixteen bytes had been written, so `serve` stopped
  with `E_CAS_CORRUPT`. A run id now reads its bytes back from a binary
  format and keeps its one spelling in text. (crates/kernel/src/event/identity.rs:61)
- Listing a provider's models filed the key and cleared its box, and the
  attach that followed registered the endpoint without it, so every call
  answered 401. The attach now carries the reference the form filed. (client/src/views/setup/providers/form.svelte:194)
- The Mayor room is a chat page. A new chat mode, first in the list, sends
  what the person typed as they typed it; the model, workspace, effort and
  mode menus say in one line what each choice does; the strip under the box
  opens to show what the prompt carries; a refused or stopped dispatch is
  answered in the conversation with the way forward; a dropped file is
  inserted as a path the city can read.
- In the composer, Tab completes a command (`/ne` becomes `/new`), the arrow
  keys move through several matches and Enter takes one; a line that spells
  a known command runs it, and a message that starts with a path is sent as a
  message.
- Every other page stands in one desktop frame: the title and the page's
  controls on one line, the content using the window's width. The ledger
  reads as sentences with the raw fields behind a fold, and each run state on
  the board has its own mark. A run the page learns of after a reload is
  titled by its task or goal.
- Requests are shaped the way each provider's own documentation says: where
  effort is written, which field caps the output, how reasoning is handed
  back, an OpenCode session header, and a `User-Agent`. The reachability
  check no longer reports a good key as refused.
- `read` takes an absolute path that lies inside the city, and a directory
  is answered with its entries.
- The dependency page checks this computer as soon as it opens, shows the
  version installed, the version the repository pins and the newest upstream
  side by side, installs every missing tool of a tier with one press, and
  gathers the cargo tools the repository calls into one pack.
- A hung test is terminated by the runner instead of outliving it.

### Known and unfixed

- The `plan` tool cannot create the first row of an empty Roadmap, so the
  Mayor cannot plan in a new city; chat mode is unaffected.
- A skill on a shelf mounted from outside the city is listed but never
  reaches a run; a skill copied into the city's library does.
- An install runs on the city's writer thread and can hold it for as long
  as a `cargo install` takes.
- The install recipes on the dependency page are English on the Chinese page.

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
