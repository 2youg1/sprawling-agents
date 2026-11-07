# sprawling-city

The space and identity of a city, and the documents a building keeps. A city is one directory, and
the directory tree is the model: this package answers the questions that tree raises — which
building governs an address and how a new one is raised, who a resident is and what its identity
adds to a run's frozen prefix, what a building's `RULES.toml` allows, how the three configuration
files resolve into one frozen configuration, which documents a building starts with and which an
upgrade has left behind, what its shelves hold, and what the city writes when it is first formed.
Rust code names the crate `city`; `cargo -p` takes `sprawling-city`.

It reads and writes the files of the directory it is handed — a room's `URBANITE.md`, a building's
`RULES.toml`, the three `CONFIG.toml` layers, a skill package landing on a shelf — and it samples no
wall clock: what needs a time receives one. Three things are not here. History is not here: an
action hands back the `kernel::Payload` a caller appends, and `sprawling-storage` owns the Ledger,
so no call in this package writes a line. The look of an identity is not here: a `Dossier` is
numbers, and `client/` decides how to draw them. And writing a shelved skill from a page is not
built yet — `wire::Command::PutShelved` is answered `not_built` by the city's writer, and the
way to put a document on a shelf today is to edit it there.

## How it is shaped

`src/lib.rs` is an index and the modules are the library. It is organised by capability, so a public
name may differ from the name inside the module that holds it: `config_layers::load` is
`city::load_config`, `room::open` is `city::open_room`, `building::create` is
`city::create_building`.

The modules group by the part of the city they govern:

- **Space.** `building` decides which building governs an address and how a new one comes into
  being; `building::removal` takes one out with every file kept; `building::template` holds the
  templates a raised building starts from; `room` answers whether an address is a room and which
  room an unnamed request opens; `neighbourhood` and `neighbours_tool` say where the city's places
  are and who stands nearby.
- **Identity.** `resident` answers who is running this run and what the identity contributes to the
  prefix; `identity` reads the identity area of the two governed documents, rewrites it against a
  base, and freezes the version a session runs on (`identity::area` holds that grammar).
- **What a building allows.** `policy` evaluates the confidential rules, the write domains, the
  egress allowlist and the reading room, and `policy::cache` keeps another building's rules by
  `(mtime, len)`; `governed` names the three documents that govern a city and the one place they are
  written through; `prompt_documents` holds the documents a page may write and the whole-document
  steps a person takes on them.
- **Configuration.** `config_layers` reads the three files, resolves them through the ladder, and
  writes a chosen value back into the layer that governs it; `config_layers::remote`, `::search`,
  `::session`, `::shelves`, `::cache` and `::city_layer` each own one table of it. `format` owns the
  format number a file carries, refuses a newer one, and brings an older one forward with the
  original kept.
- **The documents a building keeps.** `spine_files` lays out the documents a building keeps its long
  work in and the job file a run reads; `spine_files::blank` says whether one is still the form it
  was laid out as; `spine_files::hall` reads City Hall's two identity files; `handoff_form` reads a
  handoff into four prose sections; `defaults` says whether a copy in the city is the person's or an
  earlier default.
- **The library.** `library` holds the shelves and the reading room, `library::install` is the one
  door a skill lands through (`install`, `plan_install`, and the `precheck` walk behind them),
  `library::shipped` stages the skills the binary carries, `library::audit` reads a skill's audit
  state from its audits and the digest it carries now, and `library::projection` rebuilds a
  read-only package for an auditor.
- **The rest of the city's bookkeeping.** `archive` (a projection: what a building remembers between
  runs), `schedule`, `watch`, `session`, `gitignore`, `vocation`, `history`, `wizard`, `check`
  (every TOML of a city, with the error placed at a line and column), and the three tools a resident
  reaches its own city through: `rules_tool`, `city_tool` and `neighbours_tool`.

One more door is worth knowing first: `document`, the read–judge–swap lock. It reads the bytes under
a per-path writer lock, hands them to a decision the caller wrote, and replaces the file only when
the decision accepts, so two saves from one version land once (`city D17`).

The package declares no trait and implements no seam of `docs/ARCHITECTURE.md` section 4. Where it
needs the outside world it takes a value or a closure: `library::install` accepts the registration
that records a package in the content store, and `library::audit` takes its HTTP and scanner answers
as plain values, so `citysim` and the assembly layer drive the whole package from outside.

Two directories travel inside the package, because a crates.io archive carries a package's directory
and nothing else (`city D18`, `city D20`). [`templates/`](templates/) holds the default text of the
files a city and its buildings start with — `URBANITE.md`, `JOB.md`, `RULES.toml`, `Roadmap.md`,
`City.md` — and the code reads them with `include_str!`, so a template's bytes are the first bytes
of a new city. [`skills/`](skills/) holds the skills shipped with the binary: `crates/city/build.rs`
walks every directory there that holds a `SKILL.md`, writes one `include_bytes!` entry per file into
`OUT_DIR`, and `library::shipped` shelves them when a city forms. A file directly under `skills/` is
not a skill and rides in the release archive instead.

## When it fails

Every failure is a `kernel::AxError`: a code plus the failed action, the subject, any nearby
candidates, and the recovery sentence a person can act on. The codes this package produces are
`E_CONFIG_INVALID` for a configuration file its reader refuses, `E_INVALID_ARGS` for an argument or
an address the action cannot use, `E_STORAGE_FATAL` for a disk this package cannot read or write,
`E_VERSION_CONFLICT` for a document or an identity area that moved under a save, `E_PATH_NOT_FOUND`
for a shelf item or package file that is not there, `E_CAS_CORRUPT` for content on a shelf that does
not match its digest, `E_GATE_DENIED` for a read or write the building's rules refuse, and
`E_TOOL_UNAVAILABLE` or `E_TIMEOUT` for an audit fetch that could not run. `crates/city/Spec.lean`
section 12 answers whether each code can be defined away by a type, and each module's SPEC section
names the recovery it writes.

<!-- coderabbit-ai-generated:v1 -->

<!-- coderabbit-ai-generated-section:v1:c3fb9fbb290da4b5c4175ddefae39ac282333e3db67505eaee2f1f27c743d814 -->

Refused rather than attempted, with the file untouched afterwards: a `RULES.toml` that does not
state `confidential` and `write`, or a write prefix outside a confidential building; a write to the
rules, the desktop scope or a governed document through a resident write domain; a second birth of a building that exists; a
building under the reserved subtree, or a room address used as a building; a configuration table
written into a layer that cannot reach it; a skill package reached through a symlink, one over the
package size limit, or one whose source changed between the decision and the landing; and two saves
from one version, where the first stands and the second is told to reread.
`write_governed` can save a governed document when the identity area is readable and the baseline
has not changed.

<!-- /coderabbit-ai-generated:v1 -->

## Constraints a reader acts on

The package has no features: every caller compiles the same code. It declares no `unsafe`; the
workspace lint table at the root `Cargo.toml` forbids it.

<!-- coderabbit-ai-generated:v1 -->

<!-- coderabbit-ai-generated-section:v1:9a00cfebcf21e9fa64887c62622f8114626feb64f9f570d3f66d7a88d30d0844 -->

It reads file metadata — an `mtime` and a length — to know whether a cached rules file is still
current, and takes no clock sample of its own. It keeps one piece of process-wide state: `document`
holds a table of per-path writer locks behind a `OnceLock`, so two saves of one document serialise
inside one process. This lock coordinates only in-process edits; it does not protect changes by external editors or other processes, so concurrent edits across processes need separate coordination and can still be overwritten by a save. `city::document::edit` does not require the Ledger writer lock.

<!-- /coderabbit-ai-generated:v1 -->

Two external dependencies need a reason a reader can check. `cap-std` and `cap-fs-ext` walk a skill
package by directory handle, following no link, because the standard library opens only by path and
a source directory's owner can swap one (`crates/city/Spec.lean` section 13). `toml` parses the
three layers with `deny_unknown_fields`, so a misspelled key fails at the field.

Five decisions change how a caller must use the package. `city D1`: a run never rewrites the rules
that judge it, so the write surface of `RULES.toml` is not a tool a resident holds. `city D2`: the
read bound reads the target building's rules at the moment of the call, so a cached verdict is not a
verdict. `city D3`: another building's rules are kept within a run by `(mtime, len)` only. `city D4`
says no public function exists without a production caller, so an absent capability is absent rather
than exported for a test. `city D8`: a refusal's recovery sentence belongs to the place that writes
the refusal, so the configuration ladder only prefixes the file path and never rewrites it.

Against `docs/ARCHITECTURE.md` section 3, the one workspace package this crate may name is `kernel`,
and the `depmap` gate holds it to that; its SPEC parts import `kernel`'s parts for the address and
layout models rather than restating them.

## Who uses it

Two workspace packages depend on it: `sprawling`, the assembly root, which dispatches, raises
buildings, opens rooms and writes sessions through this package, and `sprawling-accounting`, the
city's one writer. `cargo tree --workspace -i sprawling-city -e normal --depth 1` prints the current
list, which is the authority for this paragraph.

## The specification

[`Spec.lean`](Spec.lean) is the entry, and its parts live under [`spec/`](spec/);
`docs/ARCHITECTURE.md` section 11, *Specifications in Lean*, describes that layout. Section 1 is the
module table, section 3 records the assumptions and the two rules that are decided but not built,
section 7 says which neighbour owns what this crate does not, section 8 maps every interface label
to the part that holds it, section 12 answers for each code whether a type can define it away, and
section 16 names the Rust test behind each acceptance. The table at the end of section 12 says where
each `D<n>` decision lives.

## Building and checking

```bash
cargo nextest run -p sprawling-city   # the module tests beside the code
lake build crates.city.Spec           # this package's Lean proofs
cargo xtask gates spec lexicon        # the SPEC and the words it uses
```

One acceptance check needs a crate this package may not name:
`crates/sprawling/tests/skill_install.rs` installs a skill with `city::install_skill` and hands it
the production content store of `sprawling-storage` as the registration, so the shelf index and
every package file resolve through verified reads. Several bin-side checks read what a model was
actually sent — the same resident segment across two runs, and a confidential building's bytes
reaching no request — and those live beside the writer that sends it. `crates/city/Spec.lean`
section 16 says which test checks what and where it lives, and `just test` and `just models` run the
whole workspace.
