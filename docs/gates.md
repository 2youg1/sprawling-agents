# The machine gates

A violation turns the check red with a message naming the rule, the violation and an alternative. The roster is the array in `Tools/xtask/src/gates.rs` (`cargo xtask gates --list` prints it); nothing compares it with the table below, so a commit that adds or removes a gate changes both.

| Gate | Holds |
|---|---|
| workspace lints, `-D warnings` | the Rust rules in [AGENTS.md](../AGENTS.md#rust) and the hardening contract in [ARCHITECTURE.md](ARCHITECTURE.md#10-determinism-and-hardening); during migration, legacy `unsafe` remains only in `Sprawling/desktop/ffi` and `Calyx/ffi` |
| `xtask header` | the MPL-2.0 notice, then the copyright line, atop every `.rs` file |
| `xtask lexicon` | one name per concept, from the glossary |
| `xtask modmap`, `xtask length` | production modules in the module map; Rust, TypeScript and TSX source files within 500 lines; production functions within 200 lines and 4 parameters |
| `xtask comments` | zero comment-bearing lines after the exact MPL header is stripped, across changed Rust, TypeScript, TSX, Zig, Python, JavaScript and CSS sources; untouched legacy violations remain in the full-tree inventory |
| `xtask unsafe` | zero real `unsafe` tokens in changed Rust sources, including macro token trees; untouched legacy violations remain in the full-tree inventory |
| `xtask boundary` | white-box checks in Rust beside the code; stranger-entry checks in Lean under `Tools/adversary/` |
| `xtask artifact` | nothing written for a test is compiled into the binary a person downloads |
| `xtask packaged` | files a publishable package compiles in production (`include!`, `include_str!`, `include_bytes!`) lie inside its directory or its build script's `OUT_DIR`, since crates.io carries nothing else |
| `xtask crate-size` | Cargo's actual compressed packages fit crates.io's upload limit before publication |
| `xtask depmap` | `pub(crate)` by default; `pub` traits only on the seam list |
| `xtask unused` | every declared dependency named by its package's code; every workspace dependency inherited by some package |
| `xtask npm` | client lockfile in step with its manifest; runtime dependencies exactly `RUNTIME` in `Tools/xtask/src/npm.rs`, each admitted with the reason `RefRain/Spec.lean` §7-9 records; every licence on the `deny.toml` list |
| `xtask secret` | credentials as `secret:realm/name` references; plaintext reaches the vault and nowhere else |
| `xtask color` | colour from the client's theme (the `@theme` blocks `RefRain/src/theme.css` imports from `RefRain/src/theme/`) as a ratio of the gamut limit; text on glass legible over the brightest surface behind it |
| `xtask motion` | transition curve and duration from the motion tokens (`RefRain/src/theme/tokens-motion.css`), never spelled in a view |
| `xtask wording` | every word a reader is given comes from `RefRain/src/lang.json` |
| `xtask render` | every role, accessible name and landmark a settled screen wrote down is offered by the shipped screen; no box outside its container; no fixed bar over the conversation; standing controls within `talk_controls` |
| `xtask wiring` | every verb the city can carry out reached by some control, or classified on the wire seam with the reason a person may not ask for it |
| `xtask wire-ts` | `RefRain/src/wire.ts` regenerated whenever `WIRE_V` moves |
| `xtask docnum` | every number a document quotes, and every kernel enum table a SPEC carries, equal to the code that decides it |
| `xtask upstream` | every row of `docs/third-party.md` that links a GitHub project names that project, one watched path and the commit that path was read at, because `upstream-watch.yml` decomposes those cells itself; the crate-licence and font tables carry no GitHub link and are not judged |
| `xtask proof` | kani roster read from the `#[kani::proof]` attributes: no workflow names a harness, a stated total is the total, a harness left unproved cites where that was decided |
| `xtask budget` | sizes inside their budget |
| `xtask specalign` | kernel enums and the tables and rosters of `Sprawling/kernel/Spec.lean` agree variant by variant; every module's SPEC anchor resolves |
| `xtask spec` | one effective specification per crate; no prose naming a SPEC the tree lacks; the checker and the specifications import along the crate graph, never each other; no `sorry`, `admit` or `axiom` in any `.lean`; every cited path on disk; each `D<n>` heading one decision (`cargo xtask spec <lib>` writes a skeleton, the gate of that name only judges) |
| `xtask release` | nothing published names one machine's home directory, its working notes, or a document this tree does not contain |
| `xtask guard` | the two lint tables of their own, `Sprawling/desktop/ffi`'s and `Calyx/ffi`'s, each equal to the workspace's except `unsafe_code`, every other crate inheriting the workspace's; every workspace package pinned in `[workspace.dependencies]` to `=` the `[workspace.package]` version and named by no member through a path of its own |
| `xtask residue` | product roots only from the `[[product_root]]` table in `architecture.toml`; a `layout = "current"` root present on disk; a retired `[[legacy_path]]` absent from the tree and from active configuration path tokens (`crates/…`); the isolation zone is skipped at walk and CHANGELOG filenames are outside the scan by scope; quoted labels that are not paths are not residue; wrong-case siblings of a current root refused; Calyx shell residue under `Calyx/src/console/{ui,editor,screen}` refused permanently (`Calyx/Spec.lean` D7) |

- A red gate has three exits: fix the code, narrow the rule to a precise scope, or delete the gate. Never add an exception. A scope names the property that makes every included case legitimate and has positive and negative fixtures. Review judges that property; a waiver, suppression, grandfather entry or re-priced threshold is not a substitute for it.
- Put a change to gate machinery — `Tools/xtask/`, `justfile`, `.github/`, `flake.nix`, the root `Cargo.toml`, `deny.toml`, `clippy.toml`, `rust-toolchain.toml`, `lakefile.toml`, `lean-toolchain`, `Tools/xtask/budgets.toml`, `architecture.toml` — in a commit apart from the source it judges, so review sees whether the gate moved to admit it.
- Existing `length`, `comments` and `unsafe` violations remain visible in full-tree inventory runs until cleaned. Daily and branch checks pass changed paths through `cargo xtask gates length comments unsafe --files <path>...`; an empty list judges nothing and never falls back to the tree. A change touching a violating file clears every violation in it, and a change may not add a violation elsewhere. For `comments` and `unsafe`, the allowance is exactly zero. No register exempts old content, and every other gate stays blocking.
- A rule that excludes an architecture carries the parameter that made it right, and when that parameter moves, re-argue the rule instead of obeying it. Rules that exclude a defect — the panic bans, the arithmetic bans, the determinism rules — carry no such condition, because nothing about them expires.
