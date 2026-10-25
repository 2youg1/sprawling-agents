# xtask

The machine gates, and the build and release commands, run as `cargo xtask <command>` through the alias in `.cargo/config.toml`. A gate turns a rule of `ARCHITECTURE.md` or `AGENTS.md` that a machine can judge into a check: when the rule is broken, the gate names the rule, the violation and an alternative, and the run exits non-zero. Its manifest marks it `role = "tool"`; it ships in nothing a person downloads and sits outside the product's crate graph (`ARCHITECTURE.md` section 3).

It depends on `kernel`, whose secret scanner the `secret` gate reuses, on `browser`, which holds the verdicts the `render` gate reaches, and on `wire` with its `schema` feature, from which `wire-ts` writes the client's types. No package depends on it.

## Gates and commands

```bash
cargo xtask gates --list        # every gate, in the order `cargo xtask gates` runs them
cargo xtask gates               # every gate
cargo xtask gates <name>...     # only the named gates; an unknown name is refused, not read as "all"
just gates                      # every gate, then the supply-chain read
```

The roster is the array `GATES` in `src/gates.rs`, and nothing else lists it; the table *The machine gates* in [`AGENTS.md`](../../AGENTS.md) says which rule each gate holds. The commands that are not gates are dispatched in `src/main.rs`. Among them, `docnum --write` and `wire-ts --write` regenerate what the `docnum` and `wire-ts` gates compare; `spec <lib>` writes a crate's SPEC skeleton; `commits --range <base>..<tip>` judges commit subjects; `proof` runs the kani harnesses; `shots` photographs every page; and `mem`, `package`, `sbom` and `repro` are measurements and deliverables that `just` recipes call. A gate only reads: no gate changes a file, and `docnum` and `wire-ts` write only when given `--write`.

The data the gates read lives beside them: `lexicon.toml` holds the retired words the `lexicon` gate refuses, and `budgets.toml` holds the budgets, among them sizes, function lengths and parameter counts, each a ratchet whose new reading is recorded beside it with its reason. A change to either, or to anything else in this directory, is a change to gate machinery and goes in a commit apart from the source it judges (`AGENTS.md`, *The machine gates*).

## Specification

The SPEC is [`Spec.lean`](Spec.lean), with its parts under [`spec/`](spec/), which prove what one `gates` run concludes, how the `guard` gate compares lint tables key by key, and which items the `packaged` gate keeps out of production. Section 1 says what each gate judges, section 7 describes this package's own modules, which the module map in `architecture.toml` does not list (decision D9), and a decision is a comment that opens with `D<n>`, cited as `xtask D<n>`. The SPEC is written in Chinese with concept names in English, as the *Language* section of `AGENTS.md` requires.

## Checking it

```bash
cargo nextest run -p xtask      # the tests; each gate is tested on fixtures its tests build
lake build tools.xtask.Spec     # the SPEC's proofs; `just models` builds every specification
just shots-frames               # the one test that needs an engine that draws frames; ignored by `just check`
```
