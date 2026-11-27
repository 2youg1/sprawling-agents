# Verification

Only tier 1 runs on the machine the change is written on, because the whole check takes minutes a workstation should not spend compiling. Tier 1 catches a defect in the change, tier 2 a change that breaks its neighbours, tier 3 two branches that are each green and wrong together; tiers 2 and 3 run on GitHub Actions, whose runners also cover the platforms a workstation is not.

## The loop

```bash
cargo install just --locked   # once
just prereqs                  # every other tool the loop needs, with the install line for each absent one
just check                    # the whole check: fmt, gates, Lean specifications, clippy, features, nextest, client
```

`just check` opens with `just prereqs`, which reads the develop tier of the doctor's table (`Sprawling/app/src/doctor/table/prereqs.tsv`), so a missing tool is named at once. `build-web` runs inside `check` because the `render` and `npm` gates judge the built client.

| Command | What it does |
|---|---|
| `just check` | the whole check; *Verification* says when it runs |
| `just check-branch [base]` | the local equivalent of `ci.yml` tier 2 for a change that cannot reach the runners, over `<base>...HEAD`; every step runs even after another fails, and a green result is recorded against the tree |
| `just check-all [phase...]` | every phase of `just check`, each run even after another fails; logs and `phases.tsv` under `$CARGO_TARGET_DIR/check-all`; named phases run alone, including the existing fmt, gates, desktop and console phases; unknown names fail |
| `just gates` | the machine gates, then the supply-chain read |
| `just verify-sources [--full] [--scope-base <base>]` | the complete source graph; only length/comments/unsafe use the optional changed-file scope |
| `cargo xtask ci plan [--base <base>]` | content keys and selection; `--base` explicitly selects the affected graph |
| `cargo xtask gates <name>...` | the named gates only; `--list` prints the roster |
| `cargo xtask gates length comments unsafe --files <path>...` | file length, zero non-MPL comments and zero unsafe tokens only on the named changed sources; an empty list is an empty judgement |
| `just commits <base>..<tip>` | commit subjects and `Verdict:` trailers in a range, judged against *Commits* in [AGENTS.md](../../AGENTS.md#commits); CI runs it on the change-set, while `just check` cannot, because a tree has no range |
| `just models` | every Lean specification, built with no `sorry`, `admit` or `axiom`; fails where Lean is absent |
| `just fmt` | rustfmt, `zig fmt` and `bun fmt` (width and file list in `bunfig.toml`); `just fmt-check` runs them in check mode |
| `just features` | the two feature combinations nothing else compiles: default features, and `wire` without `server` |
| `just check-client` | the client's lint, `bun check`, typecheck and tests on the `build-web` bundle |
| `just build-web` | the client bundle, into `Sprawling/app/web-dist` |
| `just dist` | client, binary, bill of materials |
| `just sim` | citysim scenarios: fixed scripts on a counted clock, so a failure replays |
| `just provider <script> <record>` | a stand-in provider on a loopback port playing a wire script and recording every exchange; prints `SPRAWLING_PROVIDER=<url>` first |
| `just shots` | every page at 1440 and 1920, dark and light, in `target/shots`; a person reads them, no gate does |
| `cargo xtask docnum [--write]` | every number and generated section a document carries, checked against the code; `--write` rewrites them |
| `just adversary` | the out-of-tree property checker attacking the binary through the wire; never a gate, a no-op without Lean |
| `just proof` | kani propositions on real MIR; never a gate, a no-op without kani (no Windows host); `cargo xtask proof --list` prints the roster |
| `just --list` | every other recipe — SPEC skeletons, ledger replay, measurements, fuzzing, mutation testing — each commented |

- Let a Rust command finish, and never kill it by PID; waiting on the build lock is expected. Time a slow build with `cargo build --timings` before tuning it, because the seconds sit in particular compilation units.

## Source verdicts

`just verify-sources` runs every source gate in the gate roster, formatting, channel validation, the CI-tools and nightly contract tests, and the desktop and console Zig recipes. The console recipe tests the frame leaf; shell formatting belongs to `fmt`. The separate shell test command is `(cd Calyx && zig build test)`. Each action reads an owned source snapshot. External and Cargo-backed actions receive separate copies, so their writes cannot change another gate's inputs.

CI runs the complete graph with `--full --scope-base <event-base>`. The baseline is resolved once to immutable Git objects and compared with the captured manifest. Only `length`, `comments` and `unsafe` receive those changed paths; other gates keep their whole-tree judgement. A release-tag push (`v*`) uses the first parent of its checked-out commit whether the tag is new or re-cut, because a re-cut tag's `before` names the retired tag object, which no ref reaches and a fresh clone cannot resolve, so a tag event never reads that field. Other invalid baselines fail. On Unix the comparison and snapshot also preserve the Git executable bit. Without either base option, the three file gates retain the full migration inventory, including untouched debt. An explicit empty scope judges no files. `--base` opts into affected-node selection as well as that same file scope; it conflicts with `--full` and `--scope-base`.

Six gates can reuse a successful verdict on Linux: `header`, `lexicon`, `motion`, `wording`, `wire-ts` and `release`. Their keys bind captured source bytes, operation, toolchain inputs, dependency keys and the loaded verifier read through `/proc/self/exe`. A failed identity probe disables reuse and still runs verification; other platforms always execute. Other nodes remain `Never` because their external environment is not closed by the key. A refusal or execution failure is never a reusable success.

The optional `.github/ci-cache` adapter uses the GitHub cache service through a Node 24 action, keeping its SDK and storage in the original checkout. Only SDK installation is nonfatal. Missing, corrupt or unavailable cache entries cause real verification. The cache may accelerate execution but cannot turn a failed action green. The Rust build cache remains separate from these verdict records.

Cache acceptance requires a cold production run, an exact-key hit on a fresh runner at the same tree and ref, and a changed-input miss followed by real execution. Restoring a build target or observing a cache prefix match does not establish verdict reuse. Linux `core` separately lints xtask and runs its CI-domain tests, including Unix paths and loaded-image identity, before running this same graph; its existing collector and release policy is unchanged.

## Tier 1 — while iterating

Run the narrowest command that can fail for the change, as soon as the change exists: `cargo nextest run -p <crate> -E '<filter>'` naming the new test and the tests of the modules you changed; `cargo clippy -p <crate> --all-targets --all-features -- -D warnings`; `cargo check -p <dependent>` for each direct dependent (`cargo tree --workspace -i <crate> --depth 1`) after a `pub` surface changed; `cargo xtask gates <name>...` naming the gates that judge the files you touched, with `length comments unsafe --files <changed path>...` for the three file-level gates; `lake build <module>` for a Lean file you touched; and `cargo fmt --check`, which compiles nothing. A measurement also runs locally, alone on the machine.

Before opening a pull request, apply that selection to the complete final diff, including repairs made during review, and record the commands and outcomes in the description. Do not leave an affected local check for CI to discover. `just check` is not required for every change: use it when the change is broad enough to need every phase, or use `just check-branch` when the branch cannot reach the runners. Whole suites, mutation, fuzzing, kani, the adversary, screenshots and release builds otherwise belong to the next tiers. Code behind `#[cfg(unix)]` or `#[cfg(not(windows))]` is linted only where it compiles, and CI lints Windows on every push but macOS once a night, so before such a change merges run `cargo clippy --workspace --all-targets --all-features --target x86_64-unknown-linux-gnu -- -D warnings` or wait for the macOS job.

The `length`, `comments` and `unsafe` gates carry a progressive migration: old violations may remain in full-tree inventory runs, but daily and branch checks judge only changed files. The no-base `just check` remains the full migration inventory. A change touching such a file must clear every violation in it and leave none behind, and a change may not create new violations elsewhere. The changed-file comment and unsafe-token allowances are zero. `check-branch` and CI provide a fixed baseline for the graph to capture those lists once; full-tree runs are debt inventories rather than the daily loop. No baseline or waiver hides old content. Every other gate remains blocking.

## Tier 2 — the branch's CI

Before a branch merges, push it and let `ci.yml` judge it: a pull request starts it, and `gh workflow run ci.yml --ref <branch>` starts it on a branch with none. Nobody waits for the run: whoever picks up the next piece of work reads it first, and a red run is fixed on that branch before anything else lands on top of it. `on-demand.yml` gives the same branch whatever else it needs (*mutants*, *sim*, *shots*, *release*, *pgo*).

A contributor without push access runs tier 2 in a fork before opening the pull request, so its own run confirms a green result instead of being the first time the code meets the check; this is a ruling. **Fork the repository**, enable workflows on the fork's Actions tab (a new fork has them disabled), keep the fork's `main` level with this repository's, because a dispatched run judges commit messages from where the branch left the fork's `main`, and push to a branch of the fork. `gh workflow run ci.yml --repo <fork> --ref <branch>` **starts the run**, `gh run list --repo <fork> --workflow ci.yml --branch <branch>` shows its status, and `gh run view <run-id> --repo <fork> --log-failed` prints the failed steps. Open the pull request once the run is green and link it in the description. `ci.yml` reads no secret, so every job runs in a fork; a check that needs what a fork lacks, such as a workflow that reads a secret, is left to the maintainer's run, and the description names it.

## Tier 3 — the batch

A batch of green branches merges into one branch, `ci.yml` runs on that branch, and `main` fast-forwards only to a batch whose run is green. A red batch gets a fix commit on the batch branch, and the run is dispatched again. `just check` and `just check-all` run the same phases on one machine, for a change that cannot reach the runners; `just check-all` lists every failure in one run, and `just check-all <phase>` reruns a red phase alone.

A green result belongs to the tree it ran on: record it with `git rev-parse HEAD^{tree}` and rerun it only when something outside the tree, such as the toolchain, changed.

## Scheduled and on-demand workflows

The nightly Zig campaigns run the desktop buffer rules and the Calyx frame renderer separately on x86_64 Linux. From `Sprawling/desktop/ffi` or `Calyx`, run `zig build fuzz --fuzz=100K` with the pinned compiler. Each campaign restores its own corpus and saves the next generation after success. A failing input is attached to the run. Use `gh workflow run nightly.yml --ref <branch> -f zig_only=true` to exercise these two jobs on demand.

Five scheduled workflows judge what a push cannot: `nightly.yml` (advisories, fuzzing, and the stable compile of the standalone fuzz targets), `platforms.yml` (macOS `check-core` and the non-file-scoped gates, the workspace on the oldest compiler `rust-version` names, the Nix flake, a byte-for-byte rebuild), `adversary.yml` (the Lean adversary on a fresh seed), `upstream-watch.yml`, and `nightly-release.yml` (a snapshot of main when its exact SHA has green CI and advances the current nightly). The macOS lane does not repeat `length`, `comments` and `unsafe`: their deterministic changed-file verdict already runs in `ci.yml`, so legacy debt does not turn every scheduled run red. A red scheduled run is news about the tree as it stands: read it the next morning and fix the cause on `main`. The advisory job instead files and deduplicates an issue for every finding; a tracked issue does not make the next run red, while a failed check or issue write still does. `on-demand.yml`, dispatched with `-f job=<name>`, runs what nobody waits for on a push (mutation of a branch's diff, citysim, the screenshots, a release build of a ref, a profile-guided release build), and the kani autoharness lead runs when `nightly.yml` is dispatched by hand.

`nightly-release.yml` publishes only through GitHub Releases. It embeds the full source SHA, builds all three platforms or none, creates a draft before publication, and retains the two newest published snapshots, dropping the older one when the newest curated release already contains its commit; the newest snapshot always stays, because an empty channel leaves `install.sh` with nothing to fetch. Its prepare and retention scenarios run locally with `sh .github/workflows/test-nightly-release.sh`, using mocked GitHub responses and no repository writes. `ci.yml` evaluates the Nix package when its inputs change, so a deterministic flake failure does not wait for the scheduled campaign.

### Served PDF acceptance

The PDF browser check accepts the packaged client's local port and an evidence directory. Start a disposable city, obtain its initial pairing code from its terminal, and run `PLAYWRIGHT_MODULE=<installed module> REFRAIN_PDF_PAIR_CODE_FILE=<temporary code file> bun .github/workflows/acceptance-pdf.mjs <port> <evidence directory>`. A one-use URL generated by the city's native browser opener can be passed in `REFRAIN_PDF_OPEN_URL_FILE` instead. The script also uses the argument convention of `lake exe acceptance client-ui` for an acceptance serving that supplies the browser's admission. It registers a temporary folder through the file library and generates a vector PDF with differently coloured regions inside and outside the formula selection. The evidence records the actual draft pixels, the reading anchor before and after a cold page open, and live canvas backing bytes. Canvas insertion times measure raster availability from the browser's navigation clock and from the file entry click.

Set `REFRAIN_PDF_BROWSER` to a browser executable when running a particular Chromium-family browser. Set `REFRAIN_PDF_SCAN` to a locally available image-based PDF to exercise the scan path. One repeatable sample is the 300 ppi [scan of E. F. Codd's 1970 paper](https://github.com/tpn/pdfs/blob/master/A%20Relational%20Model%20of%20Data%20for%20Large%20Shared%20Data%20Banks%20-%20E.F.%20Codd%20%281970%29.pdf), whose first page contains both image strips and invisible OCR text. The runner records the scan's SHA-256 and checks it through the same file library. Keep the source PDF outside the repository.

For a comparison of small formula strokes, use Carl H. Love's [NBS Institute for Basic Standards scan](https://doi.org/10.6028/jres.070b.023) with `REFRAIN_PDF_SCAN_PAGE=2`. The two screenshots use the same viewport and reading position, first with night mode enabled and then with the original temporarily held. The runner checks the reading position again after release.

The reader's 4,194,304-pixel limit applies to each full canvas and the preview limit is 131,072 pixels. Continuous reading mounts the viewport, up to four pages ahead and up to two behind. It also retains a page with an active region selection outside that window. Single and book layout mount the current turn. The detached raster cache adds at most 48 MiB of RGBA backing. The browser check verifies full-spread fit, counts the attached canvases in its workload and checks the mounted page range of its continuous layout fixture. Peak process measurements also include the PDF bytes, parsing and image decoder state, and the browser compositor. A large PDF keeps one preview document and opens a full-detail document on demand. The browser acceptance blocks non-loopback requests. The production bundle packages the worker, CMaps, symbol fonts and WASM resources used by the PDF.js loader.

Run `PLAYWRIGHT_MODULE=<installed module> REFRAIN_OPEN_URL_FILE=<one-use URL file> bun .github/workflows/acceptance-gallery.mjs <port> <evidence directory>` against the same packaged client with a fresh opening. The Gallery check follows the served welcome and Gallery routes. Its inventory lists rendered controls and whether each reads the shared control presentation. The interaction records contain the control's computed scale, background, outline, glyph and exposed state before and during pointer press, while held, when the pointer leaves, after release and after finite transitions settle. A trace measures whole browser event dispatch. Headless callback timing varies with the browser and system load.

| Served control | Observations in the browser runner |
|---|---|
| PDF original, details, reading brightness and magnification | Pointer hover, press, hold, drag out, release and settled frames in normal, reduced-motion and forced-colour presentation. The held original bitmap also responds to the keyboard. Details and brightness preserve the canvas, and temporary zoom and pan restore the original line. |
| PDF text, region and source reference | The native text range survives returning from its keyboard context menu. Right-click crop retains the region and excludes pixels outside the box. CDP touch at a narrow viewport covers a blank area, an existing native range, a drag to capture a formula and Back. Reopening a reference reaches the versioned range, and a changed version refuses the jump. |
| Gallery tabs, rows and segmented choices | Pointer cancellation, selection and steady state; reduced motion and forced colours for tabs; arrow and tab focus; a disabled choice remains skipped and exposes its reason. |
| Gallery field, tooltip, combobox, table, dialog and copy | Text entry retains focus. A focused hint dismisses on Escape and restores the control. A missing option leaves the choice unchanged. Table selection, sorting and editing reach the same table. Cancel and confirm return to the dialogue opener. Copy writes the sample to the browser clipboard. |
| Gallery stand state and offline rendering | Independent stand inputs, Git picks and navigation rails. Fixture use sends no city command. Back restores real directory access and browser drafts. The browser loads local CMaps, renders DOCX content and its counts, refuses an oversized DOCX, and shows preview HTML in a sandbox without an external request. |

The Gallery inventory and these per-action traces are separate evidence. They identify controls that still need an interaction scenario. CDP touch input measures browser behaviour under emulation. Physical reach, display refresh, font coverage and total process memory require separate device observations.

### Render gate with a served client

`cargo xtask render` drives headless Chromium through Bun and waits for the built Gallery to load before taking DOM measurements. For a Chromium installation that blocks navigation to local files, start a disposable city with the same embedded client or the same specified build directory. Save its native opener's one-use loopback URL to a private file, then run `SPRAWLING_BROWSER=<Chromium executable> cargo xtask render --served-entry <private file> --bundle <built directory>`. The tool pairs through the real entry and reads the same DOM probe for all five width and colour conditions. Keep the one-use URL outside the repository. Either entry fails on a page error.

## What CI skips, and what it will not run

`ci.yml`'s `changes` job classifies the paths a pull request touched into `rust`, `client`, and `lean`. A Rust-only change skips `client` and `swap`; a client-only change skips `clippy`, the test archive and its slices, and Linux `core`; a Lean-only change runs `models` (and `gates` / `supply`); documentation and root prose alone still run `gates` and `supply` only. `gates` is a collector over parallel `gates-sources` and `gates-artifacts` jobs. macOS `check-core` runs in `platforms.yml`, not on every pull request. A path-filtered skip is not a green substitute for a red job: `validation` accepts `skipped` only for those optional jobs, and still requires `gates` and `supply` to succeed. `workflow_dispatch` and a release-validation call run the full required set.

Small CI tools (`just`, `cargo-nextest` and `cargo-deny`) are prepared through `.github/actions/ci-tools`: an exact cache hit restores only the declared binaries and skips the GitHub Release download, while a miss still uses the pinned, checksum-verifying installer. Tool caches are scoped by OS, tool set and installer contract; a pull request may save its own small entries, and `cache-prune.yml` includes that family in pressure cleanup. A cache save race never changes a verification verdict.

When a push to `main` finishes a green `ci` run and that commit changes the inputs of the release Rust cache (`Cargo.lock`, workspace and member manifests, `rust-toolchain.toml`, `.cargo/**`), `release-warm.yml` calls `release.yml` with `channel: source`. That run writes the three-platform `release` cache entries a later tag build restores; the release profile (`lto = "fat"`, one codegen unit) is unchanged. A main push that does not touch those inputs leaves the previous dependency cache in place. The warm workflow is not a required check, so a cold dependency compile cannot paint CI red.

## Why Miri is not a gate

[Miri](https://github.com/rust-lang/miri) interprets MIR and catches undefined behaviour in pure Rust `unsafe`. New and changed Rust contains no `unsafe`; legacy `unsafe_code` remains only in `desktop_ffi` and `console_ffi` during the file-by-file migration, where almost every `unsafe` block is a single Zig or Win32 FFI call behind a `(ptr, len)` boundary already checked by a Rust reference, fuzzed on both sides, and specified in Lean. Miri cannot execute those leaves: `-Zmiri-native-lib` is experimental and Unix-only, and the desktop leaf is Windows-first. Putting Miri in CI or in `just check` would either stub the leaf (and stop testing the real one) or stay red on unsupported FFI. Effort stays on the Zig seam and its fuzzer ([issue 83](https://github.com/2youg1/sprawling-agents/issues/83)), not on an interpreter gate. A one-off local `cargo +nightly miri test` against a stubbed `console_ffi` buffer contract remains available to a person who needs it; it is not a machine gate.
