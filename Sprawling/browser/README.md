# sprawling-browser

What a resident may see of a web page and do to it, decided for a browser the city drives over WebDriver BiDi. A run looks at a page as a snapshot, clicks, fills a field, presses keys, takes a screenshot, compares two images, looks again after changing code, and finds the same signed-in account the next time through the building's own browser profile.

The package holds the decisions and the value types, and no I/O. It starts no browser process, holds no socket, downloads no driver, opens no file, and has no `async`. In `crates/sprawling`, `browser_bidi` starts or attaches to the browser, holds the socket and implements this package's `BrowserPort`, while `browser_tool` turns the actions into the `browser` and `usersbrowser` tools. It does not decide whether a building may use a browser at all: `city::policy` does, and it is also where a confidential building is refused. Page text becomes tainted like any other tool result, in `kernel::taint`.

Who reads a survey is the caller's question too. `survey` answers where a drawn page's boxes went wrong; whether that turns a gate red is decided by the `render` gate in `crates/xtask`, and whether a resident's own page is worth surveying is decided by the tool that asked.

## How it is shaped

Each module holds one decision:

- `port` — the seam, and the only trait here: `BrowserPort::send` takes a `Frame` and answers a `Reply`. A transport failure is `Err`, a remote refusal is `Reply::Error`, because "that node is gone" is an answer rather than a fault.
- `session` — which frames a conversation with a browser is made of. `Session` mints the frame ids, so a replay cannot renumber them, and never holds a socket. `Recording` is the offline adapter: it answers from a conversation somebody already had.
- `snapshot` — what a model may see of a page. An accessibility tree flattened into named roles with short labels, each carrying a reference the actor can name back; raw DOM never enters the window. A reference is a position in one snapshot and travels with its generation.
- `act`, `input` — turning an intention into frames. Click, type, read, drag, scroll and press; a drag or a scroll from an element is two frames on the wire, because BiDi names an element origin by the page's own shared id. Acting on a page nobody looked at is not expressible.
- `keyboard` — the keys a run may press and the modifiers it may hold, each name spelled once in the place both the argument reader and the frame builder read, plus the `input.performActions` frame that presses one.
- `verb` — the actions the `browser` tool offers: open, snapshot, act, screenshot, measure, survey, fetch, console, viewport, close. One action can need more than one frame, so the answer is a list, and nothing here sends anything.
- `shot` — what a screenshot was asked for and what came back. The region is a rectangle, an element or a union of elements; both sides are read from the returned PNG's own bytes. Quality and scale are integer percentages, and the fraction the protocol wants is written out by formatting digits.
- `diff` — what changed between two screenshots: a ten-thousandth, and the boxes it changed in. Dimensions that differ are refused rather than scaled, because what a scale compares is the scaling algorithm.
- `geometry` — the one reading of an element's box, which `measure`, a union screenshot and `survey` all splice into the script they send.
- `devloop` — the development loop's decision: change something, look, decide whether to look again. Four exhaustive outcomes and a look budget, so a loop that would never end is a thing the types answer for.
- `profile` — where a browser keeps what it remembers. Login state belongs to a building, and the profile lives under the city's reserved subtree in `PROFILES_DIR`; `Profile::of` refuses a room, and refuses the city's own reserved subtree.
- `own` — whether an address is one of the city's own listeners, and the address a tab is on. A name is judged by its spelling and never resolved, because a lookup would make the answer depend on the network.
- `survey` — what a page turns out to be once it has been laid out: alignment, text that outgrew its container, legibility, a word outside the declared vocabulary. It measures only what layout brings into existence, and reports boxes; mapping a box back to the line that drew it happens where a source tree exists.

`BrowserPort` is the seam list row `browser::port` in [`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 4. Its production adapters are `BidiSocket`, `LazyEngine` and `AttachedBrowser` in `crates/sprawling/src/browser_bidi`, and its second adapter is `Recording`; section 8 of the SPEC, decision D1, says why there is no conformance suite beside the trait.

## When a call fails

A refusal is an `AxError` in three parts: the failed action, the subject, and a recovery. These are the codes this package raises; section 12 of the SPEC holds the state each one leaves behind, and no judgement here changes a session's, a snapshot's or a loop's state.

| Code | When | What a caller does |
|---|---|---|
| `E_INVALID_ARGS` | A frame cannot be built from the arguments, a reference does not parse, an address is not a building, an action's parameters do not read, `clip` and `ref` are given together, two screenshots differ in size, or a key name is unknown | Fix the call. A reference can only arrive malformed from something that made it up, since a snapshot mints its own. |
| `E_WIRE_MISMATCH` | A reply or a tree does not read, an element's shared id is missing from the reply, a format other than PNG is asked for its size, or decoded bytes are shorter than the header declared | Stop; the far side is not the version this package was written against. |
| `E_BROWSER_UNAVAILABLE` | The far side refused, or a replay has no answer for a frame | Report it to the person; this is news about the world, not about the request. |
| `E_LOOP_SUSPECTED` | The caller observed again after the development loop had reached an outcome | Stop observing. The caller holds the `Step`, so going past an outcome is its own error. |
| `E_GATE_DENIED` | The address is one of the city's own pages, which `own::refuse` refuses before a tab opens | Open the address the work serves; the city's own pages belong to the person. |

Each refusal carries a recovery. These are refused before a frame is built: acting without a snapshot, a reference from a snapshot the page has moved past, an address for `fetch` that is relative to the page or is `file:` or `data:`, and a rectangle screenshot carrying a handle. A size asked of bytes that are not a PNG is refused rather than guessed, and a screenshot whose retake still exceeds `SHOT_MAX_EDGE_PX` is refused after the capture.

## Constraints a reader acts on

- The package declares no features; nothing here can be compiled out.
- No `async`, no executor, no socket, no file handle: a whole browser conversation is asserted without a browser, and the binary owns the one place bytes move (D2).
- The crate reads no policy. A confidential building is refused where the two settings are read together, `city::policy::evaluate`, which is why `profile` has no confidential branch of its own.
- The `browser` tool is off until a building's rules turn it on, and `usersbrowser` is a second setting: its value is the address of a browser the person already runs, and a building that declares no address has every call meet the gate's question. Attaching never closes the person's browser.
- `fetch` sends one `script.evaluate` frame that runs the page's own Fetch API with credentials, so the request carries the page's cookies and obeys its same-origin and CORS rules. Redirects are not followed: a redirect that changes host stops at the redirect, and the next request goes through the gate again (D6).
- A screenshot's long edge is `SHOT_MAX_EDGE_PX`, stated to the capture and judged on both sides read from the bytes, with one retake at a computed scale before a refusal (D7).
- There is no `unsafe` in this package, and it names only `kernel` besides `serde`, `serde_json`, `base64` and `png`.

## Who uses it

Two workspace packages depend on it: `sprawling`, where `browser_bidi` implements the seam and `browser_tool` offers the tools, and `xtask`, whose `render` gate reads `survey`. `cargo tree --workspace -i sprawling-browser -e normal --depth 1` prints the current list.

Its line in the `depmap` block of [`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 3 allows it `kernel` alone.

## The specification

The SPEC is [`Spec.lean`](Spec.lean), with its parts under [`spec/`](spec/); section 11 of [`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md), *Specifications in Lean*, describes that layout. `spec/Act.lean`, `spec/Keyboard.lean`, `spec/Devloop.lean`, `spec/Snapshot.lean` and `spec/Profile.lean` prove the properties of acting, pressing keys, the loop's end, a label that is clean and bounded, and two buildings' profiles not nesting. Section 3 of the entry holds what is assumed about the protocol and what is still open, section 8 each module's interface, section 12 the error table, and section 16 the tests that hold what the proofs leave open.

A decision is a comment opening with `D<n>` directly above what it governs, cited elsewhere as `browser D<n>`. The SPEC is written in Chinese with concept names in English, as the *Language* section of [`AGENTS.md`](../../AGENTS.md) requires; [`docs/glossary.md`](../../docs/glossary.md) defines *browser*, and section 6 of the SPEC keeps *snapshot* meaning `PageSnapshot` here and never the client's fold.

## Building and checking

```bash
cargo nextest run -p sprawling-browser    # the module tests, which need no browser
lake build crates.browser.Spec            # this package's Lean specification
cargo xtask gates spec depmap             # its specification and its dependency edges
```

The `render` gate judges this repository's gallery through `survey`: `cargo xtask gates render`, after `just build-web`, because the gate reads the built bundle and refuses rather than skips when it is absent. `just test` and `just models` run the same checks for the whole workspace.
