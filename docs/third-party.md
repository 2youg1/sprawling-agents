# Third-party projects and licensing

> **For anyone who needs to know whose work this stands on and what is owed for it.** It covers the part a machine cannot see: which upstreams are followed for intelligence, which work is outsourced to an outside service, which crates this repository chose, what the client ships that someone else made, and the skills that travel with the tree.
>
> The full resolved dependency graph is the lockfile's, checked crate by crate by `cargo deny` against the allowlist in `deny.toml` on every CI run.

sprawling is MPL-2.0; see `LICENSE`.

Two kinds of outside thing appear here, and the boundary differs, so they get a section each: **intelligence** is a constant copied down, and a **service** is a third party that does work for the user at run time.

## 1 Where the intelligence comes from

Signing in to a provider requires knowing four things: the authorization endpoint, the token endpoint, the client id, and the scopes. Calling a face this city did not invent requires one more: the shape of the request and of the answer. Those are **facts** rather than works, and citing them creates no licence obligation. The source still has to be written down, or "check periodically whether upstream changed" is a discipline with no address to go to.

| Project | Licence | What is followed | Where to look | Tracked to |
|---|---|---|---|---|
| [openai/codex](https://github.com/openai/codex) | Apache-2.0 | OpenAI's subscription login: authorization endpoint, token endpoint, client id, scopes, device-code flow | `codex-rs/login/` | `b725da3b6d52` |
| [openai/codex](https://github.com/openai/codex) | Apache-2.0 | where OpenAI states the contract for driving codex non-interactively, which is the shape the `Codex` family answers in | `docs/exec.md` | `ab753387ccf5` |
| [openai/codex](https://github.com/openai/codex) | Apache-2.0 | which base URL a ChatGPT subscription is served under, as against the key-billed platform | `codex-rs/model-provider-info/` | `8f103417ef3e` |
| [anthropics/claude-agent-sdk-typescript](https://github.com/anthropics/claude-agent-sdk-typescript) | proprietary, under Anthropic's Commercial Terms of Service | the protocol types the Claude agent wire is spelled in, and which release changed one | `CHANGELOG.md` | `9e477a178c37` |
| [xai-org/grok-build](https://github.com/xai-org/grok-build) | Apache-2.0 | xAI's browser login: that the endpoints come from OIDC discovery at `{issuer}/.well-known/openid-configuration`, and how a refresh is spelled | `crates/codegen/xai-grok-login/src/oidc/` | `f0e3be1100ef` |
| [xai-org/grok-build](https://github.com/xai-org/grok-build) | Apache-2.0 | xAI's issuer `https://auth.x.ai`, the public client id, and the scopes a subscription asks for | `crates/codegen/xai-grok-login/src/config.rs` | `75810042ca27` |
| [xai-org/grok-build](https://github.com/xai-org/grok-build) | Apache-2.0 | xAI's device-code login: the two endpoint paths under the issuer, and the grant type | `crates/codegen/xai-grok-login/src/device_code.rs` | `f0e3be1100ef` |
| [MoonshotAI/kimi-cli](https://github.com/MoonshotAI/kimi-cli) | Apache-2.0 | Moonshot's subscription login: the platform table, the OAuth endpoints, and the refresh | `src/kimi_cli/auth/` | `b5f48ef2aaf1` |
| [openai/openai-openapi](https://github.com/openai/openai-openapi) | MIT | the request and answer of the embeddings face, which every compatible server copied | `openapi.yaml` | `5b29d7c599e2` |
| [huggingface/text-embeddings-inference](https://github.com/huggingface/text-embeddings-inference) | Apache-2.0 | the request and answer of the rerank face, which has no OpenAI shape to copy | `docs/openapi.json` | `d246fbf17cc7` |

> **Machine authority**: `.github/workflows/upstream-watch.yml` reads every
> row above that opens with `| [` and carries a `github.com` link — the
> repository out of the project link, and the watch path and the tracked
> commit out of the last two cells. The column shape is fixed and the
> prose around it is not.

**One row is one watched path.** The workflow sends the fourth cell to the commits API as a single path, so a cell naming two paths would ask GitHub for a path that does not exist, and a run that asks for nothing gets nothing rather than saying so. Two paths therefore mean two rows with two watermarks. The workflow opens no second issue for a repository that already has one open, so when two paths of one repository move at once, the second is reported only after the first issue is closed. That cost grows with each row a repository gains, and it is paid deliberately: a path nobody watches is a constant that goes stale without a signal.

**The split is by family**, and the families are the four this city signs in to directly: `Codex`, `ClaudeCode`, `GrokBuild` and `KimiCli`, the enum in `gateway::provider::registry`, which cites this section as where its facts come from. A fifth source would buy one cross-check and cost an extra place to read on every review, plus a round of judgement whenever two sources disagree.

**Anthropic's subscription login is implemented here**, in `gateway::credential::oauth`, so no third party's reading of that login is followed. What still moves without warning on the Anthropic side is the protocol the wire is spelled in, which is why the agent SDK's changelog is the watched path. If an Anthropic endpoint moves and no watched path says so, an addressable source for that side is restored.

**Anthropic's SDK repository is not open source, and that changes nothing about what may be followed.** Its `LICENSE.md` places use under Anthropic's Commercial Terms of Service, so no file of it may be copied into this tree under any reading. Facts are still facts: a type name, a field, and the release that changed them carry no copyright, and this city writes its own request. The repository also answers to the shorter name `anthropics/claude-agent-sdk`, which redirects; the row spells the name GitHub canonicalises to, so the watch compares the same repository a person visiting the link lands on.

**codex's `docs/exec.md` is a short pointer to `developers.openai.com`.** The contract it names is published as a web page with no commit history, so the page cannot be watched and the pointer can. A change to the pointer is the one machine-visible signal that the contract moved, which is what this row buys; the contract's own text is read at the destination.

**xAI's login constants are literals in the login crate**: the issuer, the public client id and the scopes sit in its configuration module, and the device-code login posts to `{issuer}/oauth2/device/code` and `{issuer}/oauth2/token` without consulting the discovery document. `gateway::oauth_profiles` states that grant.

**The ceiling, modality and price cells of `gateway::provider::preset` are re-checked against the vendor's own page or left alone.** A figure re-stated on the strength of a page nobody opened is a figure with no source, which is the defect this whole section exists to prevent.

**The rerank face is the one shape in this table with no vendor behind it.** Nobody publishes `/rerank` as an API a vendor owns; the servers that serve it defined it, so the row follows the description of the server this city targets, and a second server answering a different shape is a second connection rather than a wildcard in the reader.

**How to re-check**: watch the paths above for changes rather than watching releases, because an endpoint migration often arrives in a patch version with no mention in the changelog. Where two sources disagree, the provider's own documentation decides, not the majority. Every row above was read at the commit its `Tracked to` cell names.

**The watch is automated.** Every day `upstream-watch` asks each path for its newest commit and compares it with `Tracked to`; a difference opens one issue naming the commit, a compare view, and what to re-check. `Tracked to` advances only in the pull request that realigns the constants, the same change-set that carries the new facts, so the watermark never runs ahead of what the code knows.

**Why follow intelligence and not code**:

1. "Has this endpoint expired?" is not machine-decidable. It is a periodic human task, so the thinner the dependency the better.
2. Facts carry no copyright and code does. Following intelligence makes this file a courtesy; following code would make it an obligation.
3. **Upstream flow code can be wrong while the constants in the same files are right.** An upstream login flow once set the OAuth `state` parameter to the same value as the PKCE `verifier`, and Anthropic's endpoint refuses such a request with `400 invalid_grant`, while every endpoint constant in the same files was correct. **So `oauth_begin`, in `gateway::credential::oauth`, refuses `state == code_verifier`**: somebody copying an upstream shape while wiring it is the one path by which that defect could arrive here.

**What is followed is never a vendor's billing side-channel.** An upstream client may send a field that only its own vendor's server understands, and copying it makes this city send a token it cannot read to endpoints that cannot read it either. The reported case is a per-request line carrying a `cch=` parameter, prepended to the system prompt on the messages face: recognised by that vendor's server, and cache-defeating at every third-party endpoint, because a compatible relay keys its prompt cache on the whole prompt text. So the rule has a check rather than a paragraph: `gateway::provider::stability` asserts that, under one configuration, two dispatches send the same system prefix byte for byte, with nothing in front of it.

## 2 The outsourced service: outside applications

A user may want the city connected to dozens of outside applications — mail, GitHub, Figma, Discord. Writing an integration for each means following each of their APIs, which is a weekly chore unrelated to the problem this repository solves. So that whole class of work is outsourced, and the first choice is [Composio](https://composio.dev) (SDK monorepo [ComposioHQ/composio](https://github.com/ComposioHQ/composio), MIT).

| It does | This code only does |
|---|---|
| each application's OAuth and connection management | read the tool table it offers |
| tool discovery and call execution | write every call into the Ledger, so it replays offline |

The connection is **MCP**, not their SDK: a Composio session opened with its `mcp` option yields an MCP endpoint URL that any MCP client can reach. That choice has a direct consequence: **the MCP transports — `protocol::mcp::stdio` and `protocol::mcp::http` — never know what Composio is.** They reach any MCP server, and Composio is one URL among them. A user who does not trust it points at another, or runs their own, and not one line of the protocol changes.

**Where the knowledge is allowed to live.** Composio is an OAuth broker standing in front of MCP, so the server module that knows about it, `protocol::mcp::broker`, brokers OAuth and does nothing else. It makes four calls: `GET /api/v3/toolkits` for the directory, `GET /api/v3/auth_configs` to reuse an auth config the project already holds, `POST /api/v3/auth_configs` to create a Composio-managed one when it holds none, and `POST /api/v3/connected_accounts/link` for the consent redirect. It hands back a server URL and a connection to wait on. How its calls leave this machine is decided by `gateway::client_for`, like every other outbound call. There is one broker and therefore no trait; a second outsourced service is what would earn one. The client's MCP page also spells the broker's MCP base URL and its key header, in `client/src/views/mcp/composio.svelte`.

**One-click connect is built on that module, for three reasons.** A Composio-managed auth config is created by one call, with no dashboard visit and no OAuth client of the user's own, so `auth_config_id` is a value this code obtains rather than one a person fetches. The consent redirect goes through `connected_accounts/link`, which is Composio's named replacement for the endpoint it is retiring for managed OAuth, so this code is written against the survivor. And the consent screen the redirect opens is Composio's own, so the person grants access to a third party knowingly, which is the guarantee the `link` flow exists to enforce. A connect request is recorded as `toolkit_link_opened`; where the application stands afterwards is asked of the broker each time, because a recorded standing would still read "connected" after the person revoked it.

**The project key stays the user's.** It is held in `gateway::credential::vault` beside every other credential, is never bundled and never proxied.

Three boundaries hold, each part of what the product promises:

1. **The account is the user's.** No key is bundled, nothing is paid on their behalf, nothing is proxied.
2. **Nothing polls from here.** An outside event — new mail, a new pull request — is something a service pushes to the city. This build has no receiver for pushed events, and it does not poll in place of one: a city that asked "anything new?" on a timer would generate traffic nobody reads, and would be a second authority on what arrived first.
3. **Everything an outside tool brings back joins the taint set** — outside content is data, never instructions — because those tools cross the same seam as the built-in ones. A confidential building constructs no outside tool at all.

## 3 The code this binary is built from

Two lists exist and they answer different questions, so both are kept and neither is a copy of the other.

**What this repository chose** is the table below: every crate named in the `Cargo.toml` of a workspace member, with the licence its own manifest declares. A person asking "whose work did these authors decide to stand on" reads this. The version each is pinned at is in the manifests, and the version it resolves to is in `Cargo.lock`; neither is repeated here.

**What ends up in the binary** is <!-- xtask:begin dependency_count -->442<!-- xtask:end --> packages once every transitive dependency is resolved, and that list is the lockfile's. `cargo deny check` reads it on every CI run and refuses any licence outside the allowlist in `deny.toml`; `cargo xtask sbom` writes it out as CycloneDX, which `just dist` produces beside the release artifacts. **That is the machine authority, and this table is deliberately not a second one**: it is a list a reader can hold in their head, and it goes stale the way any prose does, while the gate does not.

| Crate | Licence |
|---|---|
| [argon2](https://crates.io/crates/argon2) | MIT OR Apache-2.0 |
| [axum](https://crates.io/crates/axum) | MIT |
| [base64](https://crates.io/crates/base64) | MIT OR Apache-2.0 |
| [blake3](https://crates.io/crates/blake3) | CC0-1.0 OR Apache-2.0, or Apache-2.0 with the LLVM exception |
| [cap-fs-ext](https://crates.io/crates/cap-fs-ext) | Apache-2.0 with the LLVM exception, or Apache-2.0, or MIT |
| [cap-std](https://crates.io/crates/cap-std) | Apache-2.0 with the LLVM exception, or Apache-2.0, or MIT |
| [chacha20poly1305](https://crates.io/crates/chacha20poly1305) | Apache-2.0 OR MIT |
| [cpu-time](https://crates.io/crates/cpu-time) | MIT OR Apache-2.0 |
| [crossterm](https://crates.io/crates/crossterm) | MIT |
| [flate2](https://crates.io/crates/flate2) | MIT OR Apache-2.0 |
| [futures-util](https://crates.io/crates/futures-util) | MIT OR Apache-2.0 |
| [getrandom](https://crates.io/crates/getrandom) | MIT OR Apache-2.0 |
| [git2](https://crates.io/crates/git2) | MIT OR Apache-2.0 |
| [insta](https://crates.io/crates/insta) | Apache-2.0 |
| [keyring](https://crates.io/crates/keyring) | MIT OR Apache-2.0 |
| [memory-stats](https://crates.io/crates/memory-stats) | MIT OR Apache-2.0 |
| [png](https://crates.io/crates/png) | MIT OR Apache-2.0 |
| [postcard](https://crates.io/crates/postcard) | MIT OR Apache-2.0 |
| [proc-macro2](https://crates.io/crates/proc-macro2) | MIT OR Apache-2.0 |
| [proptest](https://crates.io/crates/proptest) | MIT OR Apache-2.0 |
| [regex](https://crates.io/crates/regex) | MIT OR Apache-2.0 |
| [reqwest](https://crates.io/crates/reqwest) | MIT OR Apache-2.0 |
| [same-file](https://crates.io/crates/same-file) | Unlicense OR MIT |
| [schemars](https://crates.io/crates/schemars) | MIT |
| [secrecy](https://crates.io/crates/secrecy) | Apache-2.0 OR MIT |
| [serde](https://crates.io/crates/serde) | MIT OR Apache-2.0 |
| [serde_json](https://crates.io/crates/serde_json) | MIT OR Apache-2.0 |
| [sha2](https://crates.io/crates/sha2) | MIT OR Apache-2.0 |
| [syn](https://crates.io/crates/syn) | MIT OR Apache-2.0 |
| [sysinfo](https://crates.io/crates/sysinfo) | MIT |
| [tempfile](https://crates.io/crates/tempfile) | MIT OR Apache-2.0 |
| [thiserror](https://crates.io/crates/thiserror) | MIT OR Apache-2.0 |
| [thread-priority](https://crates.io/crates/thread-priority) | MIT |
| [tokio](https://crates.io/crates/tokio) | MIT |
| [tokio-tungstenite](https://crates.io/crates/tokio-tungstenite) | MIT |
| [toml](https://crates.io/crates/toml) | MIT OR Apache-2.0 |
| [tower](https://crates.io/crates/tower) | MIT |
| [trybuild](https://crates.io/crates/trybuild) | MIT OR Apache-2.0 |
| [uuid](https://crates.io/crates/uuid) | Apache-2.0 OR MIT |
| [wasmtime](https://crates.io/crates/wasmtime) | Apache-2.0 with the LLVM exception |
| [wasmtime-wasi](https://crates.io/crates/wasmtime-wasi) | Apache-2.0 with the LLVM exception |
| [wat](https://crates.io/crates/wat) | Apache-2.0 with the LLVM exception, or Apache-2.0, or MIT |
| [zeroize](https://crates.io/crates/zeroize) | Apache-2.0 OR MIT |
| [zip](https://crates.io/crates/zip) | MIT |

**Every licence above is permissive, and none of them is copyleft**, which is why an MPL-2.0 binary may be built from them. MPL-2.0 is file-level copyleft: it governs the files in this repository and asks nothing of the crates linked beside them. Where a crate offers a choice, the choice on `deny.toml`'s list is the one taken; `same-file`'s Unlicense is not on that list, and its MIT is.

**Two obligations travel with a release artifact rather than with this repository**, and neither is discharged by this table:

- Apache-2.0 §4 requires the NOTICE to be kept on distribution and modified files to be marked, so a release archive carries a NOTICE.
- MIT requires the copyright and licence notice to be kept, so the same.

**Links go to crates.io rather than to each project's repository**: the question this table answers is which licence a crate declares, and crates.io shows that field beside the version a lockfile would resolve to.

**How to regenerate this table.** Run `cargo metadata --format-version 1 --locked`, take the packages a workspace member names as a dependency, and read each one's `license` field. A row that disagrees with that output is this table being stale, and the output wins.

## 4 The faces the client ships

The client draws itself in **Geist Sans** and **Geist Mono** ([vercel/geist-font](https://github.com/vercel/geist-font)), one variable-weight `woff2` each, under **SIL Open Font License 1.1**. They are the only binary assets in this repository that are somebody else's work.

| File | Family | Licence |
|---|---|---|
| `client/src/fonts/Geist-Variable.woff2` | Geist | OFL-1.1 |
| `client/src/fonts/GeistMono-Variable.woff2` | Geist Mono | OFL-1.1 |
| `client/src/fonts/OFL.txt` | the licence text, copied verbatim from upstream `LICENSE.TXT` | OFL-1.1 |

**The licence travels with the font, not with this document.** OFL-1.1 §2 requires the copyright notice and the licence text to accompany every copy of the font, including one embedded in a program, so `OFL.txt` sits in the same directory as the two `woff2` files and `client/vite.config.ts` emits it into the bundle as `fonts/OFL.txt`. The binary embeds the bundle, so the obligation is discharged wherever the binary goes. A build whose `client/src/fonts/` is missing any of the three names says so once per file and produces a bundle that draws in the fallback stack.

**Three things OFL-1.1 asks that this repository keeps honouring**: the font files are not sold on their own, they are not renamed while still carrying a reserved name, and a modified copy would have to drop the name *Geist*. This code does none of the three; the files travel unmodified, under their own names.

**No CJK face is named, and none is shipped.** `client/src/theme.css` lists no Chinese family at all: a Han glyph reaches whatever face the machine has, which is what an engine does with a glyph the named families do not carry. Naming one would mean either naming a face its vendor owns — Microsoft YaHei, PingFang and Hiragino are the ones a person actually has — or shipping our own, and a CJK face as its vendor ships it is larger, gzipped or not, than the client's whole budget of <!-- xtask:begin budget_bytes:frontend_artifact -->2,097,152 B<!-- xtask:end -->.

The font is not fetched from a font host at run time. The reason is in `client/src/theme.css` beside the declaration: a request to an outside host would tell that host a city was opened, and would draw nothing on a machine with no route out.

## 5 The skills this repository ships

Seven skills under `skills/` travel with this tree. The directory is a skill shelf in the layout harnesses file a skill in — one directory per skill, holding `SKILL.md`, which `crates/city/src/library/reading.rs` spells in the one place this city states it — so a city mounts it read-only through `[skills] shelves`, and a person copies it anywhere a skill is read. **The release archive carries the directory as itself**, `skills/LICENSES.md` beside it: `cargo xtask package` walks the tree in, so the person who unpacks a release finds the skills where the harnesses look.

| Skill | What it is for | Licence | Origin |
|---|---|---|---|
| `skills/sdd/SKILL.md` | spec-first programming work: a SPEC.md before the code, the code kept in step with it | MPL-2.0 | the author's own Chinese-language skill, translated here |
| `skills/tutor/SKILL.md` | discovery teaching: one person, dialogue and verifiable outcomes, no courseware | MPL-2.0 | the same |
| `skills/translation/SKILL.md` | low-variance translation of form-as-content text into Chinese | MPL-2.0 | the same |
| `skills/why/SKILL.md` | design rationale read off evidence, with citations | MIT | modified adaptation of pstack's `why` |
| `skills/how/SKILL.md` | how a subsystem works, and how to critique it | MIT | modified adaptation of pstack's `how` |
| `skills/blast-radius/SKILL.md` | what a change breaks somewhere else | MIT | modified adaptation of pstack's `blast-radius` |
| `skills/authority-review/SKILL.md` | a branch before it merges: correctness, security and the recorded decision, then one lens over the whole diff — a fact with more than one authoritative definition | MIT | modified adaptation of the Thermos plugin (`cursor/plugins`), pass two recut as the owner's one-fact-one-authority audit |

**What is owed, and to whom.** Three of these are the repository owner's own work (2youg1), translated and adapted from the original Chinese-language skills, which are published as open source under AGPL-3.0-or-later. The author licenses these three under MPL-2.0, the licence of the rest of this tree, and the originals remain AGPL-3.0-or-later; each file carries its own provenance note, and `README.md` acknowledges what the files stand on. The translation skill keeps the byline its original wore — KL9 & Claude Fable 5. The pstack three stay MIT: upstream is pstack by Lauren Tan (poteto) ([cursor/plugins](https://github.com/cursor/plugins)), Copyright (c) 2026 Lauren Tan, and **the modifications are marked as 2youg1's (2026)**, because a modified version that does not say who modified it hides what it now is. `authority-review` stays MIT too: its upstream is the Thermos plugin in the same repository, and its modifications are marked the same way. `skills/LICENSES.md` travels with the directory and carries the full MIT text and the attribution each licence asks for, so the obligations reach a copy that leaves this tree.

**Nothing under `skills/` enters the binary or the client bundle; the archive carries them as themselves.** These are prompt documents, read at run time by whoever keeps a city, and the release archive packs the directory whole, so the obligations travel with the files and their `LICENSES.md`, in the tree and in the zip alike. The four MIT adaptations are the one corner of the tree where a reader may not assume MPL-2.0, which is why `README.md` says so at the licence line, and why the MPL header gate judges `.rs` alone: an MPL notice on an MIT document would misstate its terms.

## 6 What the client follows, and what it only watches

**No component library is a dependency of this client, and the ruling that keeps it that way is `client/client-SPEC.md` section 7.** The controls under `client/src/views/parts/` are this repository's own, and `cargo xtask npm` holds the client's runtime dependencies to exactly the list its `RUNTIME` names — `effect`, `svelte`, and the `@lezer` highlighter and grammars (MIT, Marijn Haverbeke), which draw code and are not controls — so adding one means editing that gate first.

What is followed instead is the same kind of thing section 1 follows for provider login, and for the same reason: **behaviour somebody else wrote down is a fact, and a fact carries no licence obligation.** Which pattern a control implements, what each key does, which `aria-*` value goes where, and when the focus returns to whatever opened the control — that is the useful half of a component library, and all of it is published as prose.

| Source | Licence the project declares | What is followed |
|---|---|---|
| WAI-ARIA Authoring Practices Guide, <https://www.w3.org/WAI/ARIA/apg/patterns/> | W3C, permissive document licence per the page's own footer | pattern names and keyboard tables for radio group, tabs, combobox with a listbox popup, modal dialog, and tooltip |
| Kobalte documentation, <https://kobalte.dev/docs/core/> | MIT, declared by `@kobalte/core` | how an implementation divides those same patterns into parts, and which `aria-*` each part owns |
| Ark UI documentation, <https://ark-ui.com/docs/components/> | MIT, declared by `@ark-ui/solid` | the same division, read as a cross-check against Kobalte |

**Not one line of their code is in this tree, so no licence obligation arises from any of the three** — neither a NOTICE nor a copied licence text, because nothing was copied. What the reading produced is a keyboard table per control in `client/client-SPEC.md` section 7, together with the places today's code does not yet meet it and the condition that would reopen the ruling.

> **A table row in this file must not begin with `| [` while also carrying a `github.com` link, unless it is an upstream-watch row.** `.github/workflows/upstream-watch.yml` greps the whole file for rows that open with `| [` and reads a repository out of any `github.com` URL on the line. The three rows above therefore open with plain text and point at each project's documentation site.

## 7 A future bolt-on crate

The provider intelligence table is planned to move out into a crate of its own under its own licence (MIT or Apache-2.0), outside this repository's MPL notice, because it is not part of this work. It covers the four families of section 1, and each family's watch travels with it as its own row.

The cost has to be stated plainly: it would be a **build-time dependency**, so its code enters the shipped binary. The licence obligations therefore **travel with the binary rather than with the repository**:

- Apache-2.0 §4 requires keeping the NOTICE on distribution and marking modified files, so a release artifact must carry a NOTICE.
- MIT requires keeping the copyright and licence notice, so the same.

The acknowledgement in `README.md` does not discharge either. **An acknowledgement is a courtesy and a NOTICE is an obligation; both are required and neither substitutes for the other.**

The same measure governs **upstream synchronisation**. When an upstream publishes a new version, usually because a new model appeared, realigning the bolt-on crate should be **one run opening one pull request**, rather than a person periodically reading several repositories' diffs. It is work the city can do itself, so no separate mechanism is built for it: a schedule entry and a building that owns the bolt-on are enough.

## 8 What will not be done

**Upstream code is not vendored in.** Every `.rs` file here carries the MPL-2.0 notice, and a file pasted from an MIT or Apache-2.0 project cannot wear one. The machine checks only whether a notice is present, not whether it is the right one, **so this rule is held by people, not by a gate.**

**Credential custody is not delegated.** Plaintext reaches only the credential service on the machine the city runs on. Which facts a bolt-on may hold (endpoints, client ids, scopes) and which never leave (custody, redemption, renewal) is part of what the product promises rather than an organisational convenience.

**No outside service's key is bundled, nothing is paid, nothing is proxied.** Which service to connect and whose account to use is the user's decision.
