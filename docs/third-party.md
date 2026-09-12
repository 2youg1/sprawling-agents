# Third-party projects and licensing

> **For anyone who needs to know whose work this stands on and what is owed for it.** It covers the part a machine cannot see: which upstreams are followed for intelligence, which work is outsourced to an outside service, and how a future bolt-on crate is licensed.
>
> Section 3 lists the crates this repository chose and the licence each declares. The full resolved graph is the lockfile's, checked crate by crate by `cargo deny` against the allowlist in `deny.toml` on every CI run.

sprawling is MPL-2.0; see `LICENSE`.

Two kinds of outside thing appear here, and the boundary differs, so they get a section each: **intelligence** is a constant copied down, and a **service** is a third party that does work for the user at run time.

## 1 Where the intelligence comes from

Signing in to a provider requires knowing four things: the authorization endpoint, the token endpoint, the client id, and the scopes. Those are **facts** rather than works, and citing them creates no licence obligation. The source still has to be written down, or "check periodically whether upstream changed" is a discipline with no address to go to.

| Project | Licence | What is followed | Where to look | Tracked to |
|---|---|---|---|---|
| [openai/codex](https://github.com/openai/codex) | Apache-2.0 | OpenAI's subscription login: endpoints, client id, scopes, device-code flow | `codex-rs/login/` | `1b83e5cdf998` |
| [earendil-works/pi](https://github.com/earendil-works/pi) | MIT | the same intelligence for Anthropic and the other subscription providers | `packages/ai/src/auth/oauth/` | `55b0db4d3e90` |

> **Machine authority**: `.github/workflows/upstream-watch.yml` parses the five
> columns of the two rows above - project, watch path, tracked commit. The
> column shape is fixed; the prose around them is not (cf. ARCHITECTURE.md
> §3, §12 tables). |

The split is by provider: codex covers the OpenAI side, pi covers Anthropic and the rest. **Only these two.** A third source would buy one cross-check and cost an extra place to read on every review, plus a round of judgement whenever the three disagree.

**How to re-check**: watch those two paths for changes rather than watching releases. An endpoint migration often arrives in a patch version with no mention in the changelog. Where two sources disagree, the provider's own documentation decides, not the majority.

**The watch is automated.** Every day `upstream-watch` asks each path for its newest commit and compares it with `Tracked to`; a difference opens one issue naming the commit, a compare view, and what to re-check. `Tracked to` advances only in the PR that actually realigns the constants - the same change-set that carries the new facts, so the watermark never runs ahead of what the code knows.

**Why follow intelligence and not code**, three reasons, the last one learned by measurement:

1. "Has this endpoint expired?" is not machine-decidable. It is a periodic human task, so the thinner the dependency the better.
2. Facts carry no copyright and code does. Following intelligence makes this file a courtesy; following code would make it an obligation.
3. **Upstream flow code can be wrong while the constants in the same files are right.** Measured in 2026-08: `pi` set the OAuth `state` parameter to the same value as the PKCE `verifier`. The same pattern had been found and fixed by another harness that year, because Anthropic's endpoint refuses such a request with `400 invalid_grant`; every endpoint constant in `pi` was correct. **So this code refuses `state == code_verifier` in `oauth_begin`**: somebody copying the upstream shape while wiring it is the one path by which that defect could arrive here.

## 2 The outsourced service: outside applications and wake-ups

A user may want the city connected to dozens of outside applications - mail, GitHub, Figma, Discord. Writing an integration for each means following each of their APIs, which is a weekly chore unrelated to the problem this repository solves. So that whole class of work is outsourced, and the first choice is [Composio](https://composio.dev) (SDK monorepo [ComposioHQ/composio](https://github.com/ComposioHQ/composio), MIT).

| It does | This code only does |
|---|---|
| each application's OAuth and connection management | read the tool table it offers |
| tool discovery and call execution | write every call into the Ledger, so it replays offline |
| listening for outside events - new mail, a new pull request - and pushing them here | accept what is pushed, and wake the building it belongs to |

The connection is **MCP**, not their SDK: opening the `mcp` option when a session is created yields an MCP endpoint URL that any MCP client can reach. (The older standalone `composio.mcp` service-management API is deprecated; no new code is written against it.) That choice has a direct consequence: **`protocol::mcp` never knows what Composio is.** It connects to any MCP server, and Composio is one URL among them. A user who does not trust it points at another, or runs their own, and not one line of the protocol changes. Knowledge of Composio is permitted in exactly one module, named two paragraphs below, and that module brokers OAuth rather than carrying a second transport.

**One-click connect is built here, and this paragraph records why the earlier refusal to build it was wrong.** That refusal rested on three claims and two of them were false when they were written. *The step a button saves is not saved* - it is: `POST /api/v3/auth_configs` creates a Composio-managed auth config with no dashboard visit and no OAuth client of the user's own, the documented fastest way to start, so `auth_config_id` is a value this code obtains rather than one a person fetches. *The endpoint to write against is moving* - the moving one is the endpoint being replaced: `POST /api/v3/connected_accounts` retires for Composio-managed OAuth on redirectable schemes on 2026-05-08 for organisations created from that date and 2026-07-03 for the rest, while `POST /api/v3/connected_accounts/link` is the named replacement and is explicitly outside that retirement, so writing against `link` is writing against the survivor. *The city would learn what Composio is* - it already had, in the client: `client/src/views/mcp/composio.tsx` carries the backend URL and the `x-api-key` header name, and `client/src/views/mcp/toolkits.ts` carries a directory of toolkits. The boundary under defence had already been crossed, and only its location was still open.

**Where the knowledge is allowed to live.** Composio is an OAuth broker standing in front of MCP, so the module that knows about it brokers OAuth and does nothing else: it holds three calls - `GET /api/v3/toolkits` for the directory, `POST /api/v3/auth_configs` for a managed config, `POST /api/v3/connected_accounts/link` for the consent redirect - and hands back a server URL and a connection to wait on. `protocol::mcp` receives that URL and remains a transport that has never heard of any of it. There is one broker today and therefore no trait; a second outsourced service is what would earn one.

**The project key stays the user's.** It is held in `gateway::credential::vault` beside every other credential, is never bundled and never proxied, and the consent screen the redirect opens is Composio's own - so the person grants access to a third party knowingly, which is the guarantee the `link` flow exists to enforce.

Four boundaries, each of them part of what the product promises:

1. **The account is the user's.** No key is bundled, nothing is paid on their behalf, nothing is proxied.
2. **Never poll from here.** Events are pushed and the city receives them. A city that asked "anything new?" on a timer would be generating traffic nobody reads, and would be a second authority on who arrived first.
3. **No redelivery after a disconnection.** A broken machine or router is something a person notices; no compatibility layer is built for it. But nothing degrades silently either: connecting and disconnecting each land an event, so "when was this building not listening" is a readable fact rather than a guess.
4. **A building that is taken down stops listening.** A building is one line of business; when the business ends its outside connections end with it, and no second cleanup list is needed.

Everything an outside tool brings back joins the taint set - outside content is data, never instructions - because those tools cross the same seam as the built-in ones. A confidential building refuses all outbound calls.

## 3 The code this binary is built from

Two lists exist and they answer different questions, so both are kept and neither is a copy of the other.

**What this repository chose** is the table below: the twenty-seven crates named in a `Cargo.toml` of this workspace, each with the licence its own manifest declares. A person asking "whose work did these authors decide to stand on" reads this.

**What ends up in the binary** is 387 packages once every transitive dependency is resolved, and that list is the lockfile's. `cargo deny check` reads it on every CI run and refuses any licence outside the allowlist in `deny.toml`; `cargo xtask sbom` writes it out as CycloneDX, which `just dist` produces beside the release artifacts. **That is the machine authority, and this table is deliberately not a second one** — it is twenty-seven rows a reader can hold in their head, and it goes stale the way any prose does, while the gate does not.

| Crate | Version | Licence |
|---|---|---|
| [axum](https://crates.io/crates/axum) | 0.8.9 | MIT |
| [blake3](https://crates.io/crates/blake3) | 1.8.7 | CC0-1.0 OR Apache-2.0, or Apache-2.0 with the LLVM exception |
| [flate2](https://crates.io/crates/flate2) | 1.1.10 | MIT OR Apache-2.0 |
| [futures-util](https://crates.io/crates/futures-util) | 0.3.34 | MIT OR Apache-2.0 |
| [getrandom](https://crates.io/crates/getrandom) | 0.4.3 | MIT OR Apache-2.0 |
| [git2](https://crates.io/crates/git2) | 0.21.0 | MIT OR Apache-2.0 |
| [insta](https://crates.io/crates/insta) | 1.48.0 | Apache-2.0 |
| [keyring](https://crates.io/crates/keyring) | 3.6.3 | MIT OR Apache-2.0 |
| [proc-macro2](https://crates.io/crates/proc-macro2) | 1.0.107 | MIT OR Apache-2.0 |
| [proptest](https://crates.io/crates/proptest) | 1.11.0 | MIT OR Apache-2.0 |
| [redb](https://crates.io/crates/redb) | 4.2.0 | MIT OR Apache-2.0 |
| [reqwest](https://crates.io/crates/reqwest) | 0.13.5 | MIT OR Apache-2.0 |
| [secrecy](https://crates.io/crates/secrecy) | 0.10.3 | Apache-2.0 OR MIT |
| [serde](https://crates.io/crates/serde) | 1.0.229 | MIT OR Apache-2.0 |
| [serde_json](https://crates.io/crates/serde_json) | 1.0.151 | MIT OR Apache-2.0 |
| [sha2](https://crates.io/crates/sha2) | 0.11.0 | MIT OR Apache-2.0 |
| [syn](https://crates.io/crates/syn) | 3.0.5 | MIT OR Apache-2.0 |
| [tempfile](https://crates.io/crates/tempfile) | 3.27.0 | MIT OR Apache-2.0 |
| [thiserror](https://crates.io/crates/thiserror) | 2.0.20 | MIT OR Apache-2.0 |
| [tokio](https://crates.io/crates/tokio) | 1.53.1 | MIT |
| [tokio-tungstenite](https://crates.io/crates/tokio-tungstenite) | 0.30.0 | MIT |
| [toml](https://crates.io/crates/toml) | 1.1.5 | MIT OR Apache-2.0 |
| [trybuild](https://crates.io/crates/trybuild) | 1.0.121 | MIT OR Apache-2.0 |
| [uuid](https://crates.io/crates/uuid) | 1.26.0 | Apache-2.0 OR MIT |
| [wat](https://crates.io/crates/wat) | 1.258.0 | Apache-2.0 with the LLVM exception, or Apache-2.0, or MIT |
| [zeroize](https://crates.io/crates/zeroize) | 1.9.0 | Apache-2.0 OR MIT |
| [zip](https://crates.io/crates/zip) | 8.6.0 | MIT |

**Every licence above is permissive, and none of them is copyleft**, which is why an MPL-2.0 binary may be built from them. MPL-2.0 is file-level copyleft: it governs the files in this repository and asks nothing of the crates linked beside them.

**Two obligations travel with a release artifact rather than with this repository**, and neither is discharged by this table:

- Apache-2.0 §4 requires the NOTICE to be kept on distribution and modified files to be marked, so a release archive carries a NOTICE.
- MIT requires the copyright and licence notice to be kept, so the same.

**Links go to crates.io rather than to each project’s repository**: the question this table answers is which licence a crate declares, and crates.io shows that field beside the version a lockfile would resolve to.

**How to regenerate this table.** `cargo metadata --format-version 1 --locked`, take the packages named as a dependency by a workspace member, and read each one's `license` field. A version here that disagrees with `Cargo.lock` is this table being stale, and the lockfile wins.

## 4 The faces the client ships

The client draws itself in **Geist Sans** and **Geist Mono** ([vercel/geist-font](https://github.com/vercel/geist-font)), one variable-weight `woff2` each, under **SIL Open Font License 1.1**. They are the only binary assets in this repository that are somebody else's work.

| File | Family | Licence |
|---|---|---|
| `client/src/fonts/Geist-Variable.woff2` | Geist | OFL-1.1 |
| `client/src/fonts/GeistMono-Variable.woff2` | Geist Mono | OFL-1.1 |
| `client/src/fonts/OFL.txt` | the licence text, copied verbatim from upstream `LICENSE.TXT` | OFL-1.1 |

**The licence travels with the font, not with this document.** OFL-1.1 §2 requires the copyright notice and the licence text to accompany every copy of the font, including one embedded in a program, so `OFL.txt` sits in the same directory as the two `woff2` files and `client/vite.config.ts` emits it into the bundle as `fonts/OFL.txt`. The binary embeds the bundle, so the obligation is discharged wherever the binary goes. A build whose `client/src/fonts/` is missing any of the three names says so once per file and produces a bundle that draws in the fallback stack.

**Three things OFL-1.1 asks that this repository must keep honouring**: the font files are not sold on their own, they are not renamed while still carrying a reserved name, and a modified copy would have to drop the name *Geist*. This code does none of the three — the files travel unmodified, under their own names.

The font is not fetched from a font host at run time. The reason is in `client/src/theme.css` beside the declaration: a request to an outside host would tell that host a city was opened, and would draw nothing on a machine with no route out.

## 5 A future bolt-on crate

The provider intelligence table is planned to move out into a crate of its own under its own licence (MIT or Apache-2.0), outside this repository's MPL notice, because it is not part of this work.

The cost has to be stated plainly: it would be a **build-time dependency**, so its code enters the shipped binary. The licence obligations therefore **travel with the binary rather than with the repository**:

- Apache-2.0 §4 requires keeping the NOTICE on distribution and marking modified files, so a release artifact must carry a NOTICE.
- MIT requires keeping the copyright and licence notice, so the same.

The acknowledgement in `README.md` does not discharge either. **An acknowledgement is a courtesy and a NOTICE is an obligation; both are required and neither substitutes for the other.**

The same measure governs something not yet started: **upstream synchronisation**. When an upstream publishes a new version - usually because a new model appeared - realigning the bolt-on crate should be **one run opening one pull request**, rather than a person periodically reading two repositories' diffs. It is work the city can do itself, so no separate mechanism is built for it: a schedule entry and a building that owns the bolt-on are enough.

## 6 What will not be done

**Upstream code is not vendored in.** Every `.rs` file here carries the MPL-2.0 notice, and a file pasted from an MIT or Apache-2.0 project cannot wear one. Note that the machine only checks whether a notice is present, not whether it is the right one - **so this rule is held by people, not by a gate.**

**Credential custody is not delegated.** Plaintext reaches only the credential service on this machine. Which facts a bolt-on may hold (endpoints, client ids, scopes) and which never leave (custody, redemption, renewal) is part of what the product promises rather than an organisational convenience.

**No outside service's key is bundled, nothing is paid, nothing is proxied.** Which service to connect and whose account to use is the user's decision.
