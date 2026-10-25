# sprawling-documents

The rules of a document a person reads and edits through a page. Given the bytes of one document version, this package says which encoding they are in or that they are not text, how the file's format splits them into blocks, how much of them one answer carries without splitting a character, what one save changes and how to undo it, and where a reader's selection lands after a save. It turns a run's proposal into review slices and applies the person's decision sentence by sentence, and it lays out a window of a Markdown version as blocks and inline marks — the preview of a document, a model's reply as it streams in, and the HTML export. `docs/glossary.md` defines document version, document window, proposal, review slice, preview and closure point.

The package does no I/O, reads no clock and keeps no global state. Where the bytes come from is the caller's business (`accounting::views`), where a version is stored belongs to `storage::cas`, how a save reaches the disk belongs to `city::document`, and `sprawling-wire` carries this package's values to the page as they are. `crates/documents/Spec.lean` §7 lists these boundaries.

The `schema` feature adds the JSON Schema of the values the wire carries out of this package, for the `schema` feature of `sprawling-wire`. The product binary leaves it off.

## Who uses it

Two workspace packages depend on it: `sprawling-accounting`, the city's writer, which reads, saves and decides proposals through it, and `sprawling-wire`, which carries its values. The packages this one may name are its line in the `depmap` block of `ARCHITECTURE.md` section 3.

## The specification

`crates/documents/Spec.lean` is the entry, and its parts live under `crates/documents/spec/`: edits, encodings, windows, proposals and the Markdown grammar are proved there. `ARCHITECTURE.md` section 11, *Specifications in Lean*, says how a SPEC is laid out.

## Building and checking

```bash
cargo nextest run -p sprawling-documents   # the module tests beside the code
lake build crates.documents.Spec           # this package's Lean specification
```

`crates/documents/Spec.lean` §2 maps each acceptance to the test that checks it, including the halves that run in `sprawling-accounting`. `just test` and `just models` run every check for the whole workspace.
