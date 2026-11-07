# xtask

The machine gates and the build and release commands, run as `cargo xtask <command>` through the alias in `.cargo/config.toml`. A gate turns a rule of [`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) or [`AGENTS.md`](../../AGENTS.md) that a machine can judge into a check: when the rule is broken the gate names the rule, the violation and an alternative, and the run exits non-zero. It also carries the commands that build, measure, photograph and pack the product, none of which judges anything.

The package sits outside the product graph. Its manifest writes `role = "tool"` and `publish = false`, the `depmap` block in [`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) section 3 lists product crates only, and nothing here enters the `sprawling` binary or a release archive.

## Gates and commands

```bash
cargo xtask gates --list        # every gate, in the order `cargo xtask gates` runs them
cargo xtask gates               # every gate
cargo xtask gates <name>...     # only the named gates, in roster order; an unknown name is refused, exit 2
just gates                      # every gate, then the supply-chain read
```

The roster is the array `GATES` in [`src/gates.rs`](src/gates.rs), and `COUNT` is its length type parameter, so the number and the list are one token apart and no document has to hold the count. `--list`, the usage text and name selection all read that array. The table *The machine gates* in [`docs/gates.md`](../../docs/gates.md) names the gates and says which rule each one holds; nothing compares that table with the array, so a commit that adds or removes a gate changes both.

Each gate is one file, with the shape `fn check(root: &Path) -> Result<Vec<Violation>, XtaskError>`, so a gate judges and never repairs. Three modules judge nothing and are shared: `main` dispatches, `report` renders a violation and decides what one run exits with, and `walk` is the deterministic file traversal. The rest is read by gates rather than being one: `members` is the only reader of `cargo metadata` and answers which packages are product and which are tools; `architecture` cuts `ARCHITECTURE.md` by section for the `depmap` and `proof` gates; `lean` reads a `.lean` file as text for `spec`, `specalign` and `wiring`; `root` decides which tree is judged; `vocabulary`, `bundle`, `platform`, `attestation` and `secret::nix` hold one fact each that a gate asks about.

The commands that are not gates are dispatched in the same file. `docnum --write` and `wire-ts --write` regenerate exactly what the `docnum` and `wire-ts` gates then compare: the managed spans of the documents, and `client/src/wire.ts`. `spec <lib>` writes a crate's SPEC skeleton into its directory and never overwrites one. `members`, `commits --range <base>..<tip>`, `channel`, `acp-catalog`, `proof`, `shots`, `mem`, `package`, `sbom` and `repro` are the rest; `render`, `color` and `budget` also run alone, because a person reads the reading as well as the verdict.

The data the gates read lives beside them: `lexicon.toml` holds the retired words the `lexicon` gate refuses, and `budgets.toml` holds the budgets — sizes, function lengths, parameter counts — each a ratchet whose new reading is recorded beside it with its reason. Two more facts come from a manifest rather than from a list here: whether a package belongs to the product or is a tool, which its own `[package.metadata.sprawling] role` states (`xtask D4`), and which modules exist, which `architecture.toml` states.

## When a gate answers red

One violation renders as four lines: the gate and the location, then the rule, the violation and an alternative. A run exits 0 when every gate judged and found nothing, 1 when any gate found a violation, and 2 when a gate could not judge. Gates run on threads of their own, so the wall time is the slowest gate's, and the report keeps roster order, so two runs on one tree report the same bytes.

A gate that cannot judge is a type of its own, never a violation: `Io`, `Doc` and `Cmd` name the path or command that failed, and the rest carry a stable code with the action, the subject and a recovery — `unknown-gate`, `gate-panicked`, `stale-build`, `no-checkout`, `member-outside-checkout`, `unknown-role`, `unknown-package`, `theme-part-missing` and `theme-part-unimported` (`xtask D1`). A mistyped gate name refuses with `unknown-gate` and lists the roster instead of running everything. `stale-build` means the binary was built from a different checkout; the recovery is `cargo clean -p xtask` and the command again (`xtask D3`). One broken gate neither turns the run green nor hides another gate's findings (`xtask D2`).

## Constraints

- A gate only reads. The tree is byte-identical after any run, including a failed one; a file is written only by a command given `--write`, and `spec`, which creates a skeleton where none exists. Nothing is cached: every run rescans.
- The scan surface is the tree, not its build output: `walk` skips `target`, `node_modules`, `.lake`, `.svelte-check`, `.zig-cache` and `web-dist`, and never enters a directory below the root that holds its own `.git` entry, because a second checkout is not this tree.
- Three gates need artifacts rather than sources. `render` reads the client in `crates/sprawling/web-dist`, which `just build-web` produces, and a Chromium-family engine, which `SPRAWLING_BROWSER` may name. `npm` reads `client/node_modules` and refuses when it is absent. `budget` weighs what exists and stays silent when nothing has been built, because `just check` builds no release binary.
- Lean is read as text, never run (`xtask D8`): the shapes a gate knows are a one-constructor-per-line `inductive`, a one-arm-per-line `def` and the `import` lines. `just models` is what builds a proof.
- `modmap` judges every package except this one, whose modules the SPEC describes instead (`xtask D9`); a test module needs no row of its own (`xtask D40`).
- It may name `kernel`, whose secret scanner the `secret` gate reuses, `browser`, which holds the verdicts the `render` gate reaches, and `wire` with its `schema` feature, from which `wire-ts` writes the client's types.
- A change to this directory, to `justfile`, `.github/` or a root manifest is a change to gate machinery and goes in a commit apart from the source it judges ([`docs/gates.md`](../../docs/gates.md)). Loosening a gate in the change it is failing needs a ruling recorded as the commit's `Verdict: user-approved` trailer.

## Who uses it

No package depends on it: `cargo tree --workspace -i xtask -e normal --depth 1` prints the package alone. Its callers are `just check`, which runs the source gates before clippy and the artifact gates after `just build-web`; `just gates`, which runs every gate and then the supply-chain read; and every recipe and workflow that names one command — `just models`, `just spec <lib>`, `just package`, `just sbom`, `just repro`, `just mem`, `just shots`. `ci.yml` lints and tests this package on Windows alone, because it judges the repository rather than shipping in it.

## Specification

The SPEC is [`Spec.lean`](Spec.lean), with its parts under [`spec/`](spec/); section 1 of it says what each gate judges, section 7 describes this package's own modules, which the module map in `architecture.toml` does not list (`xtask D9`), section 8 gives the violation shape and the exit codes, section 12 the failures a gate's own breakage takes, and section 16 the tests that hold what the three proofs leave open. The parts prove what one `gates` run concludes (`spec/Report.lean`), how the `guard` gate compares lint tables key by key (`spec/Guard/Wall.lean`), and which items the `packaged` gate keeps out of production (`spec/Packaged.lean`). A decision is a comment that opens with `D<n>` and is cited as `xtask D<n>`. The SPEC is written in Chinese with concept names in English, as the *Language* section of [`AGENTS.md`](../../AGENTS.md) requires.

## Checking it

```bash
cargo nextest run -p xtask       # every gate tested on fixtures its tests build
lake build crates.xtask.Spec     # the SPEC's proofs; `just models` builds every specification
cargo xtask gates                # every gate, on this tree
cargo xtask gates <name>...      # one or a few, in roster order
just gates                       # every gate, then the supply-chain read
just shots-frames                # the one test that needs an engine that draws frames; ignored by `just check`
```

CI runs this suite in its `client` environment, which installs the client first; the step's own comment says the `npm` gate's tests read the installed client tree and refuse when it is missing. The tests judge parsing and verdicts on fixtures they build, and that the gates pass on this repository is a fact about a run, which `just check` produces.
