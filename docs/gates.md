# The machine gates

A violation turns the check red with a message naming the rule, the violation and an alternative. The roster is the array in `tools/xtask/src/gates.rs` (`cargo xtask gates --list` prints it); nothing compares it with the table below, so a commit that adds or removes a gate changes both.

| Gate | Holds |
|---|---|
| workspace lints, `-D warnings` | the Rust rules in [AGENTS.md](../AGENTS.md#rust); `unsafe` only in `crates/desktop/ffi` and `crates/console_ffi` |
| `xtask header` | the MPL-2.0 notice, then the copyright line, atop every `.rs` file |
| `xtask lexicon` | one name per concept, from the glossary |
| `xtask modmap`, `xtask length` | production modules in the module map; production functions within 200 lines and 4 parameters |
| `xtask boundary` | white-box checks in Rust beside the code; stranger-entry checks in Lean under `tools/adversary/` |
| `xtask artifact` | nothing written for a test is compiled into the binary a person downloads |
| `xtask packaged` | files a publishable package compiles in production (`include!`, `include_str!`, `include_bytes!`) lie inside its directory or its build script's `OUT_DIR`, since crates.io carries nothing else |
| `xtask depmap` | `pub(crate)` by default; `pub` traits only on the seam list |
| `xtask unused` | every declared dependency named by its package's code; every workspace dependency inherited by some package |
| `xtask npm` | client lockfile in step with its manifest; runtime dependencies exactly `RUNTIME` in `tools/xtask/src/npm.rs`, each admitted with the reason `client/Spec.lean` §7-9 records; every licence on the `deny.toml` list |
| `xtask secret` | credentials as `secret:realm/name` references; plaintext reaches the vault and nowhere else |
| `xtask color` | colour from the client's theme (the `@theme` blocks `client/src/theme.css` imports from `client/src/theme/`) as a ratio of the gamut limit; text on glass legible over the brightest surface behind it |
| `xtask motion` | transition curve and duration from the motion tokens (`client/src/theme/tokens-motion.css`), never spelled in a view |
| `xtask wording` | every word a reader is given comes from `client/src/lang.json` |
| `xtask render` | every role, accessible name and landmark a settled screen wrote down is offered by the shipped screen; no box outside its container; no fixed bar over the conversation; standing controls within `talk_controls` |
| `xtask wiring` | every verb the city can carry out reached by some control, or classified on the wire seam with the reason a person may not ask for it |
| `xtask wire-ts` | `client/src/wire.ts` regenerated whenever `WIRE_V` moves |
| `xtask docnum` | every number a document quotes, and every kernel enum table a SPEC carries, equal to the code that decides it |
| `xtask proof` | kani roster read from the `#[kani::proof]` attributes: no workflow names a harness, a stated total is the total, a harness left unproved cites where that was decided |
| `xtask budget` | sizes inside their budget |
| `xtask specalign` | kernel enums and the tables and rosters of `crates/kernel/Spec.lean` agree variant by variant; every module's SPEC anchor resolves |
| `xtask spec` | one effective specification per crate; no prose naming a SPEC the tree lacks; the checker and the specifications import along the crate graph, never each other; no `sorry`, `admit` or `axiom` in any `.lean`; every cited path on disk; each `D<n>` heading one decision (`cargo xtask spec <lib>` writes a skeleton, the gate of that name only judges) |
| `xtask release` | nothing published names one machine's home directory, its working notes, or a document this tree does not contain |
| `xtask guard` | the two lint tables of their own, `crates/desktop/ffi`'s and `crates/console_ffi`'s, each equal to the workspace's except `unsafe_code`, every other crate inheriting the workspace's; every workspace package pinned in `[workspace.dependencies]` to `=` the `[workspace.package]` version and named by no member through a path of its own |

- Fix the cause when a gate goes red. Loosening a gate in the change it is failing requires an explicit ruling from the person, recorded as the commit's `Verdict: user-approved` trailer; the ruling's wording stays with the person, and the trailer records that there was one. Review holds this rule rather than a gate, because a gate that read commit history would make every run depend on the range its caller passed.
- Put a change to gate machinery — `tools/xtask/`, `justfile`, `.github/`, `flake.nix`, the root `Cargo.toml`, `deny.toml`, `clippy.toml`, `rust-toolchain.toml`, `lakefile.toml`, `lean-toolchain`, `tools/xtask/budgets.toml`, `architecture.toml` — in a commit apart from the source it judges, so review sees whether the gate moved to admit it. Re-pricing a rule in a commit of its own is ordinary work and needs no ruling.
- A rule that excludes an architecture carries the parameter that made it right, and when that parameter moves, re-argue the rule instead of obeying it. Rules that exclude a defect — the panic bans, the arithmetic bans, the determinism rules — carry no such condition, because nothing about them expires.
