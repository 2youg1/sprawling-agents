// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `cargo xtask apisync`: the committed public-surface baselines of the
//! two crates whose surface is read outside this repository equal the
//! live surface, rendered by the one nightly rustdoc and cargo-public-api
//! release `tools/xtask/public-api.txt` pins (xtask-SPEC §12-6). Not a
//! gate: the nightly job runs it, and whether an interface change belongs
//! in a SPEC is a reviewer's call (xtask-SPEC §8-32).

use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::members;
use crate::report::{Violation, XtaskError};
use crate::walk;

const BASELINE_DIR: &str = "tools/xtask/api-baselines";

/// The file that pins the renderer; the nightly workflow installs from it.
const RENDERER: &str = "tools/xtask/public-api.txt";

/// How many drifted lines a violation names before it only counts them
/// (xtask-SPEC §12-7).
const DRIFT_SHOWN: usize = 40;

/// The crates whose public surface is a seam other code reads across a
/// process or a repository boundary, by lib name; every other crate's
/// surface is held by the compiler at its call sites.
const SEAM_CRATES: [&str; 2] = ["wire", "kernel"];

/// What turns a crate into its baseline text: a dated nightly whose
/// rustdoc emits the JSON, and the cargo-public-api release that prints
/// it (xtask-SPEC §12-6). Only `pinned` builds one, so no render can
/// happen with a toolchain the file does not name.
struct Renderer {
    /// A dated nightly such as `nightly-2026-09-29`, passed as `+<rustdoc>`.
    rustdoc: String,
    /// A cargo-public-api version such as `0.52.0`.
    version: String,
}

impl Renderer {
    /// Reads `RENDERER`: `KEY=value` lines, with `#` comments and blank
    /// lines skipped. A file without both keys is `XtaskError::Doc`.
    fn pinned(root: &Path) -> Result<Self, XtaskError> {
        let text =
            std::fs::read_to_string(root.join(RENDERER)).map_err(|source| XtaskError::Io {
                path: RENDERER.to_owned(),
                source,
            })?;
        let value = |key: &str| {
            text.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .filter_map(|line| line.split_once('='))
                .find(|(name, _)| name.trim() == key)
                .map(|(_, value)| value.trim().to_owned())
                .filter(|value| !value.is_empty())
                .ok_or_else(|| XtaskError::Doc {
                    file: RENDERER.to_owned(),
                    msg: format!(
                        "no `{key}=` line; write the dated nightly (PUBLIC_API_RUSTDOC) and the \
                         cargo-public-api version (PUBLIC_API_VERSION) the baselines are \
                         rendered with (xtask-SPEC §12-6)"
                    ),
                })
        };
        Ok(Self {
            rustdoc: value("PUBLIC_API_RUSTDOC")?,
            version: value("PUBLIC_API_VERSION")?,
        })
    }

    /// Asks the pinned toolchain which cargo-public-api it runs. A missing
    /// toolchain, a missing tool or another version is `XtaskError::Cmd`,
    /// whose message names both install lines, because a render by any
    /// other pair would be judged against a baseline it did not write.
    fn answers(&self, root: &Path) -> Result<(), XtaskError> {
        let answer = match Command::new("cargo")
            .arg(format!("+{}", self.rustdoc))
            .args(["public-api", "--version"])
            .current_dir(root)
            .output()
        {
            Ok(output) if output.status.success() => {
                String::from_utf8_lossy(&output.stdout).trim().to_owned()
            }
            Ok(output) => String::from_utf8_lossy(&output.stderr)
                .lines()
                .last()
                .unwrap_or("no output")
                .to_owned(),
            Err(err) => err.to_string(),
        };
        if answer == format!("cargo-public-api {}", self.version) {
            return Ok(());
        }
        Err(XtaskError::Cmd {
            cmd: format!("cargo +{} public-api --version", self.rustdoc),
            msg: format!(
                "the baselines are rendered by `cargo +{rustdoc} public-api` {version} \
                 ({RENDERER}); install it with `rustup toolchain install {rustdoc} --profile \
                 minimal` and `cargo install --locked cargo-public-api@{version}`; the pinned \
                 toolchain answered: {answer}",
                rustdoc = self.rustdoc,
                version = self.version,
            ),
        })
    }
}

/// The live surface, normalized to trimmed non-empty lines. Derived and
/// blanket impls are omitted (-sss): they move with the toolchain, not
/// with our decisions.
fn live_api(root: &Path, renderer: &Renderer, package: &str) -> Result<String, XtaskError> {
    let output = Command::new("cargo")
        .arg(format!("+{}", renderer.rustdoc))
        .args([
            "public-api",
            "-p",
            package,
            "--simplified",
            "--omit",
            "blanket-impls,auto-trait-impls,auto-derived-impls",
        ])
        .current_dir(root)
        .output()
        .map_err(|err| XtaskError::Cmd {
            cmd: format!("cargo +{} public-api -p {package}", renderer.rustdoc),
            msg: format!("{err}; run `just prereqs`, which names every tool this repository needs"),
        })?;
    if !output.status.success() {
        return Err(XtaskError::Cmd {
            cmd: format!("cargo +{} public-api -p {package}", renderer.rustdoc),
            msg: String::from_utf8_lossy(&output.stderr)
                .lines()
                .last()
                .unwrap_or("failed")
                .to_owned(),
        });
    }
    Ok(normalize(&String::from_utf8_lossy(&output.stdout)))
}

fn normalize(raw: &str) -> String {
    let mut lines: Vec<&str> = raw
        .lines()
        .map(str::trim_end)
        .filter(|l| !l.is_empty())
        .collect();
    // The tool's order is deterministic, but sorting makes the baseline
    // immune to ordering changes across tool versions.
    lines.sort_unstable();
    let mut joined = lines.join("\n");
    joined.push('\n');
    joined
}

/// The lines only one side holds, each on a line of its own after a
/// leading newline: `- ` for a line only the baseline has, `+ ` for a
/// line only the live surface has. Past `DRIFT_SHOWN` lines the rest is
/// only counted (xtask-SPEC §12-7).
fn drift(committed: &str, live: &str) -> String {
    let (committed, live) = (normalize(committed), normalize(live));
    let mut gone = committed.lines().filter(|l| !l.is_empty()).peekable();
    let mut came = live.lines().filter(|l| !l.is_empty()).peekable();
    let moved: Vec<String> = std::iter::from_fn(|| {
        loop {
            let order = match (gone.peek(), came.peek()) {
                (None, None) => return None,
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (Some(old), Some(new)) => old.cmp(new),
            };
            match order {
                Ordering::Less => return gone.next().map(|l| format!("- {l}")),
                Ordering::Greater => return came.next().map(|l| format!("+ {l}")),
                Ordering::Equal => {
                    gone.next();
                    came.next();
                }
            }
        }
    })
    .collect();
    let left_out = moved.len().saturating_sub(DRIFT_SHOWN);
    moved
        .iter()
        .take(DRIFT_SHOWN)
        .map(|l| format!("\n{l}"))
        .chain((left_out > 0).then(|| format!("\n… and {left_out} more lines")))
        .collect()
}

fn baseline_path(root: &Path, krate: &str) -> PathBuf {
    root.join(BASELINE_DIR).join(format!("{krate}.txt"))
}

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let renderer = Renderer::pinned(root)?;
    renderer.answers(root)?;
    let found = members::members(root)?;
    let mut violations = Vec::new();
    for krate in SEAM_CRATES {
        let path = baseline_path(root, krate);
        let rel = walk::rel(root, &path);
        let Ok(committed) = std::fs::read_to_string(&path) else {
            violations.push(Violation {
                gate: "apisync",
                location: rel,
                rule: "every seam crate carries a public-api baseline".to_owned(),
                violation: format!("no baseline for `{krate}`"),
                alternative: "run `cargo xtask apisync --write` and commit the baseline \
                              together with the SPEC"
                    .to_owned(),
            });
            continue;
        };
        let live = live_api(root, &renderer, &members::find(&found, krate)?.package)?;
        if normalize(&committed) != live {
            violations.push(Violation {
                gate: "apisync",
                location: rel,
                rule: "the committed baseline equals the live public API".to_owned(),
                violation: format!(
                    "`{krate}` public API drifted from its baseline{}",
                    drift(&committed, &live)
                ),
                alternative: "run `cargo xtask apisync --write`, review the diff, and \
                              touch the crate SPEC in the same change-set"
                    .to_owned(),
            });
        }
    }

    Ok(violations)
}

/// `cargo xtask apisync --write`: regenerate every seam baseline with the
/// pinned renderer. The only files this command ever writes.
pub(crate) fn write(root: &Path) -> Result<(), XtaskError> {
    let renderer = Renderer::pinned(root)?;
    renderer.answers(root)?;
    let dir = root.join(BASELINE_DIR);
    std::fs::create_dir_all(&dir).map_err(|source| XtaskError::Io {
        path: dir.display().to_string(),
        source,
    })?;
    let found = members::members(root)?;
    for krate in SEAM_CRATES {
        let live = live_api(root, &renderer, &members::find(&found, krate)?.package)?;
        let path = baseline_path(root, krate);
        std::fs::write(&path, live).map_err(|source| XtaskError::Io {
            path: path.display().to_string(),
            source,
        })?;
        println!("baseline written: {}", walk::rel(root, &path));
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::{drift, normalize};

    #[test]
    fn normalization_sorts_trims_and_ends_with_one_newline() {
        let raw = "pub fn b()\n\npub fn a()   \n";
        assert_eq!(normalize(raw), "pub fn a()\npub fn b()\n");
        assert_eq!(normalize(""), "\n");
    }

    #[test]
    fn a_drift_names_each_line_only_one_side_holds() {
        let committed = "pub fn wire::a()\npub fn wire::b()\n";
        let live = "pub fn wire::b()\npub fn wire::c()\n";
        assert_eq!(
            drift(committed, live),
            "\n- pub fn wire::a()\n+ pub fn wire::c()"
        );
    }

    #[test]
    fn a_drift_longer_than_the_cap_says_how_many_lines_it_left_out() {
        let live: String = (0..45)
            .map(|n| format!("pub fn wire::f{n:02}()\n"))
            .collect();
        let report = drift("", &live);
        assert_eq!(
            (
                report.lines().filter(|l| l.starts_with("+ ")).count(),
                report.lines().next_back()
            ),
            (40, Some("… and 5 more lines"))
        );
    }
}
