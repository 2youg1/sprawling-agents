# sprawling-kernel

Every decision in the city, and nothing that touches a disk. The crate is a layer of pure functions:
each takes its arguments and returns a verdict, and no module holds a file handle, a clock reading
or a writable global. Addresses and the reserved subtree, the event envelope and the Ledger's chain,
the event kinds and their window classes, the error codes with the carrier event each one surfaces
in, the Gate and its doors, plans and shares, the policy constants, the layout of a city on disk,
and the traits the outer crates implement are defined here once. Every other crate of the product
builds on those definitions rather than keeping a second copy. Rust code names the crate `kernel`;
`cargo -p` takes `sprawling-kernel`.

What it deliberately does not do. It opens no file: `Address::parse` judges a relative path and
never asks whether it exists, and `layout` derives paths without touching them. It samples no clock
and draws no randomness, because a run id, a timestamp and a seed arrive as arguments, and `uuid` is
compiled without its generation features. It implements no port — the Ledger, tool and model traits
are declared here and implemented outside. It holds no plaintext credential: `secret::Sealed` has no
`Serialize`, so a credential cannot reach a ledger line or the wire. It knows nothing of colour or
of Markdown, each of which belongs to the module that owns its effect.

## Shape

`src/lib.rs` is an index and the modules are the library. Each module is one file named for what it
owns, and a caller reaches a decision through the module that produced it — `gate::domain`,
`discard::decide`, `approval::may_answer` — so the call site says which authority answered. The root
re-exports value types only: the vocabulary a caller spells its own fields and arguments in. No
function is re-exported, and nothing is renamed on the way out, because a second spelling of one
name is the defect the `lexicon` gate exists to catch.

Each module instantiates exactly one of the seven shapes of `docs/ARCHITECTURE.md` section 9:

| Shape | Modules, and the question each answers |
|---|---|
| value | `address` (is this path inside that subtree), `locator` (which content does this reference name), `error` (what failed, and what may the caller do), `event` and its parts (what did this line record, which run and which sequence), `version`, `idem`, `release`, `skill`, `share`, `node_id`, `policy_limit`, `registry`, `retries` (how many times a failed call is made again, and what an unstated ceiling means), `taint`, `budget`, `completion`, `approval`, `delegation`, `reach`, `change`, `origin` |
| decision | `gate` with one module per door, `write_domain`, `backpressure`, `degradation`, `stall`, `goal`, `repair`, `discard`, `pursuit`, `blockage`, `keep_warm`, `config`, `secret`, `plan`, `spine`, `event::kind::window`, `event::moment` |
| port | `ledger`, `tool`, `model` |
| data | `consts_external`, `consts_policy`, `layout`, `event::record` and its parts |
| adapter | `schema`, which exists only under the `schema` feature |

`account_recovery` is the one module the module map states as a state machine rather than as one of
the seven: given the accounts a provider has and the failures so far, it answers whether a request
goes out again on the same account, moves to another, or stops.

The three seams declared here are `kernel::ledger` in `crates/kernel/src/ledger.rs`, `kernel::tool`
in `crates/kernel/src/tool.rs` and `kernel::model` in `crates/kernel/src/model.rs`; the seam table
in `docs/ARCHITECTURE.md` section 4 names the production adapter and the second adapter of each. A
`pub trait` outside those files is refused by the `depmap` gate. The `Ledger` trait owns `seq` and
`prev`, and `EventRecord::canonical_line` is the only producer of canonical ledger bytes: an adapter
writes what it returns and hashes nothing of its own, because the chain hashes exactly those bytes.

## When it fails

One error shape for the whole city: `AxError` is a `code` from the closed `AxCode` set plus one
boxed detail holding the failed `action`, the `subject` it failed on, any `nearby` candidates, and
the `recovery` sentence a person can act on. A gate refusal adds its own part, a provider failure
adds its kind, and `retry` says with `retry_after_ms` whether the same request may go out again.
`AxError::refusal` is the only constructor of a refusal, and it takes a `GateRefusal` of rule,
violation and a compliant alternative, so a refusal cannot be written with a part missing.

Each code declares the event that carries it into history in `AxCode::carrier`, an exhaustive match,
so a new code that nobody placed is a compile error rather than a code with no reader. A code whose
surface is `Carrier::Loadtime` arises only while the Ledger cannot be written, and that set is
closed on purpose: the refused party there is exactly the one that cannot record its own refusal.

`AxCode::parse` answers `Option`, and `None` is the caller's fail-closed branch. Requests refused
rather than attempted, each with its own code: an address that is absolute or holds a `..` segment,
an empty segment, a backslash, a `.` segment, a leading or trailing `/`, a control character, `:` (a
Windows drive or an NTFS stream), a segment ending in a dot or whitespace, or non-UTF-8 bytes
(`E_INVALID_ARGS`); a locator with a trailing space or odd-length hex (`E_LOCATOR_INVALID`); a
payload holding a float, or nested deeper than `PAYLOAD_DEPTH_MAX`; a `Seq` incremented past its
maximum; a log version older than the first readable one (`E_LOG_VERSION_UNSUPPORTED`). The reason
each refusal exists is in `spec/Error.lean` and `spec/Address.lean`; the code list and each code's
recovery are in `crates/kernel/Spec.lean` section 12.

## Constraints a caller acts on

Two features exist and both are off by default. `conformance` carries the assertion suite every
implementation of a port must pass, and the second adapter of each seam runs it. `schema` derives
the JSON Schema of every value that crosses the wire, which `sprawling-wire` reads through its own
`schema` feature for `cargo xtask wire-ts`; the product binary is built with neither feature, and
`just test` runs the workspace with `--all-features`.

No `unsafe`: the workspace lint table at the root `Cargo.toml` forbids it, and this crate has no FFI
seam. No `#[non_exhaustive]` either, so every enum here is closed and a reader that does not handle
a new variant fails to compile; the alternative, a wildcard arm, is denied by the same lint table.
Iteration on a decision path is explicitly ordered and never a hash order. Ledger payloads carry no
floats, refused at both `Payload::new` and its `Deserialize`.

Four decisions change how a caller uses the crate. `kernel D1`: the Gate answers allow or refuse and
nothing else, so a caller that wants a second question asks a person, not a door. `kernel D2`: there
is no rollback verb — going back is a branch in the conversation or a git restore of the files.
`kernel D4`: kernel stays one crate, so a policy module is imported by module path rather than by
package. `kernel D8`: `kernel::address` is the only home of the protected-metadata names, so no
caller keeps its own list.

It depends on no other crate of the workspace; the `kernel` row of the `depmap` block in
`docs/ARCHITECTURE.md` section 3 is empty, and the `depmap` gate holds it to that. Nothing above it
may be named from here.

## Who uses it

Every crate of the product except `child`, `console_ffi` and `desktop_ffi` depends on it, and so do
the development-only `citysim` and `xtask` and the fuzz workspace in `crates/fuzz/`. `cargo tree
--workspace -i sprawling-kernel -e normal --depth 1` prints the current list, which is the authority
for this paragraph.

## Specification

The SPEC is [`Spec.lean`](Spec.lean), with one part per module under [`spec/`](spec/);
`docs/ARCHITECTURE.md` section 11, *Specifications in Lean*, describes that layout. Section 1 lists
the modules and the shape of each, section 3 records what the models assume, section 8 maps each
interface label to the part that holds it, section 12 answers for every error code whether it can be
defined away by a type, and section 16 names the Rust test that holds each property the proofs leave
open. A decision is a comment opening with `D<n>` beside the declaration it governs, and the table
at the end of section 12 says where each one lives. The SPEC is written in Chinese with concept
names in English, as the *Language* section of [`AGENTS.md`](../../AGENTS.md) requires;
[`docs/glossary.md`](../../docs/glossary.md) defines the words.

## Checking it

```bash
cargo nextest run -p sprawling-kernel --all-features   # the module tests, with both features on
lake build crates.kernel.Spec                          # the SPEC's proofs
cargo xtask gates spec lexicon specalign               # the SPEC, the words, and the two enum tables
just proof                                             # the kani harnesses, on a Linux or macOS host
```

`cargo xtask gates specalign` is the narrow gate for this crate: it compares the `EventKind` roster
and each `AxCode` against the tables in `spec/Event/Kind.lean` and `spec/Error.lean`, variant by
variant. `crates/kernel/tests/refusal_matrix.rs` walks `DOORS` and requires the three parts on every
answer a door can give; it is compiled only under the `conformance` feature, which is one reason the
command above turns both features on. `crates/kernel/tests/trybuild.rs` compiles each file under
`crates/kernel/tests/ui/` and expects the compiler to refuse it, which is how an invalid state is
shown to be unrepresentable rather than merely rejected. The rest of the loop, and which tier runs
where, is in [`AGENTS.md`](../../AGENTS.md).
