# sprawling-documents

The rules of a document a person reads and edits through a page. Given the bytes of one version,
this package answers which encoding they spell or that they are not text, how the file's format
splits them into blocks, how much of them one answer carries without cutting a character, what one
save changes and how to undo it, where a reader's place lands after a save, and how a run's
suggestion about one stretch becomes a card a person accepts, amends or rejects. It lays a window of
a Markdown version out as blocks and inline marks, which is the document preview, a model's reply as
it streams in, and the HTML export. Rust code names the crate `documents`; `cargo -p` takes
`sprawling-documents`. [`docs/glossary.md`](../../docs/glossary.md) defines document version,
document window, proposal, review slice, preview and closure point.

It does no I/O, reads no clock and keeps no global state: bytes arrive as slices and leave as
values. What happens around it belongs to neighbours. The bytes come from the caller —
`accounting::views` reads the disk and the content store and hands in slices. A version is stored by
`storage::cas`, whose address for an object is the digest that is also the version's identity. A
save reaches the disk through `city::document`, which holds the lock and replaces the file whole,
and the city's writer records it in `accounting`. `sprawling-wire` carries this
package's `Span`, `Format`, `Encoding`, `Window`, `Preview`, `Slice` and `TextEdit` as they are, so
the page receives these values directly rather than a rendering of them.
`crates/documents/Spec.lean` section 7 states these boundaries as the shape of the interface.

## How it is shaped

`src/lib.rs` is an index and the modules are the library. Each module is one file named for what it
owns, and nine of them are separately acceptable units, exactly as `crates/documents/Spec.lean`
section 1 counts them:

- **`span`** — a stretch of one version, a half-open byte interval with one constructor and a wire
  spelling that reads back through it.
- **`encoding`** — which characters a version's bytes spell. The byte-order mark decides first, then
  UTF-8 without a NUL; anything else is `Opaque` rather than a guess (`documents D4`).
- **`format`** and **`layout`** — the grammar a file's blocks are read by, decided by its name, and
  where those blocks lie in its bytes. Inside the crate, `layout::blocks` gives the block spans and
  `layout::closed` gives the closure point: the last byte before which a still-streaming Markdown
  reply can no longer change.
- **`window`** — how much of a version one answer carries and where it is cut: `head` for the first
  window, `lift` for the bytes to read, `cut` for the window itself, none of them past the end or
  through a character (`documents D7`).
- **`edit`** — one save: a `Transaction` is a string of edits against a baseline version, `save`
  takes the page's `TextEdit`s, judges the baseline, writes them back in that version's own
  encoding, and returns an `Applied` holding the new version and the transaction that undoes it.
- **`selection`** — a reader's place in a document, `mapped` across a save (`documents D10`). No
  resident of the city holds one today: the page carries its own cursor, so this module is exercised
  by its tests only.
- **`proposal`** — one suggestion about one stretch of a version: its identity computed from the
  content rather than drawn (`documents D13`), its card of sentences (`proposal::slices` holds the
  sentence cut and the alignment), the text a person's verdicts make of it, and several cards
  landing as one save. A card that is out of date, or two accepted cards that overlap, are refused
  rather than merged.
- **`markdown`** — one window laid out as blocks and inline marks. `markdown::lowering` reads
  comrak's tree into this crate's tree under a nesting cap, `markdown::position` turns comrak's line
  and byte column into a stretch of the version, `markdown::target` decides which link and image
  targets a page may follow, and `markdown::export` writes the same tree as one HTML page that
  stands alone. `markdown::tree` is the data a page draws.

The seven shapes of `docs/ARCHITECTURE.md` section 9 appear as two of them: `span`, `Window`,
`Slice` and `TextEdit` are values, and every other module is a decision — no I/O, no clock, an
exhaustive enum rather than a bool. This package declares no trait and implements no seam of section
4; `comrak` is used directly, because a grammar with one implementation behind a trait would be
decoration.

## When it fails

Every failure is a `kernel::AxError` carrying the failed action, the subject and a recovery
sentence, and this package returns two codes.

`E_INVALID_ARGS` covers a `Span` whose start is past its end; edits that overlap or arrive out of
order; an edit past the end of the version; bytes that spell no text in their encoding; lifted bytes
too short for the window asked for; a source that is not text, or that stops being the same kind of
text after the edit; a proposal span reversed, or a proposal's original or replacement over
`WINDOW_BYTES_MAX`; a verdict naming a sentence that does not exist, naming one twice, or amending a
deleted sentence; one card decided twice, two accepted cards that overlap, and a card whose original
is not the text it was cut from.

`E_VERSION_CONFLICT` covers a source that is no longer the baseline — a save against a version that
moved, or an accepted card cut from a version the document has left. The first writer's change
stands, every card stays open, and the recovery is to reread the document and save again, or to ask
the run to offer the card again.

The grammar itself never refuses. Any string is a CommonMark document: bytes whose structure cannot
be read become paragraph text, and a construct the city reads but the page has no element for
becomes `Unsupported` carrying its source. A reply over the nesting cap reads as an `Unsupported`
block of the nesting construct rather than failing. Encoding is the one place that answers rather
than refuses: a UTF-16 version of a preview answers `Preview::Unsupported`, because guessing at a
document is worse than saying it cannot be read, and an export of one is refused with
`E_INVALID_ARGS`.

Nothing here can leave a file half-written, because nothing here writes. A refused save leaves the
source byte for byte as it was, and the failure that comes after a write belongs to the writer.

## Constraints a reader acts on

One feature, `schema`, adds the JSON Schema of the values the wire carries out of this package. It
is off by default and `sprawling-wire` turns it on from its own `schema` feature, which `cargo xtask
wire-ts` reads; the product binary is built without it. `just test` runs the workspace with
`--all-features`.

No `unsafe`: the workspace lint table at the root `Cargo.toml` forbids it. No clock, no random
source and no global state.

Three decisions change how a caller must use the package. `documents D1`: the wire carries these
types directly, so changing the shape of `Span`, `Format`, `Encoding`, `Window` or `Preview` moves
the wire schema and regenerates `client/src/wire.ts`. `documents D3`: a version's identity is the
digest of its whole bytes, so a save is judged against a version and never against a timestamp;
`save` compares the digest of the bytes it is handed with the baseline itself, so a caller passes
the bytes it read and the version it believes them to be. `documents D20` and `D22`: the Markdown
grammar is comrak's CommonMark and GFM implementations, and this package only chooses the
extensions, reads the resulting tree and judges the link protocols — so a preview cannot be fixed by
patching a parser here.

Two limits are product decisions rather than tunables: `WINDOW_BYTES_MAX`, the most one window
answer carries, and `EXPORT_BYTES_MAX`, the largest version one export reads. Section 14 of the SPEC
records both with their reasons.

Against `docs/ARCHITECTURE.md` section 3, the one workspace package this crate may name is `kernel`,
for `AxError` and for the `B3Hash` digest; the `depmap` gate holds it to that, and the dependencies
outside the workspace are `serde`, `comrak` and the optional `schemars`.

## Who uses it

Two workspace packages depend on it: `sprawling-accounting`, the city's one writer, which reads a
document, saves it and decides proposals through this package, and `sprawling-wire`, which carries
its values to the page. `cargo tree --workspace -i sprawling-documents -e normal --depth 1` prints
the current list, which is the authority for this paragraph.

## The specification

[`Spec.lean`](Spec.lean) is the entry, and its parts live under [`spec/`](spec/): `Edit.lean`,
`Encoding.lean`, `Proposal.lean`, `Markdown.lean` and `Window.lean`, one per module that carries a
property worth proving. `docs/ARCHITECTURE.md` section 11, *Specifications in Lean*, describes that
layout. Section 1 lists the nine units, section 2 maps each acceptance to the Rust test that checks
it, including the halves that run in `sprawling-accounting`, section 3 records what the models
assume, section 7 states the boundaries, section 8 gives each module's shape and signature, section
12 is the code table, and section 16 names the tests. A decision is a comment opening with `D<n>`
beside what it governs.

## Building and checking

```bash
cargo nextest run -p sprawling-documents   # the module tests beside the code
lake build crates.documents.Spec           # this package's Lean proofs
cargo xtask gates spec lexicon             # the SPEC and the words it uses
```

The two Markdown corpora live in the tests rather than in the model: the `markdown::lowering` tests
carry the CommonMark cases migrated from the client's former TypeScript prose reader, and the
`layout` tests carry the block spans of RefRain's regression corpus, whose bytes were checked
against that corpus's own SHA-256 list. `crates/documents/Spec.lean` section 2 names every
acceptance with the test that settles it. `just test` and `just models` run every check of the
workspace.
