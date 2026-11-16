// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;
use std::process::Command;

use crate::members;
use crate::report::{Violation, XtaskError};
use crate::vocabulary;
use crate::walk;

mod roster;

use roster::{in_file, module_base};

pub(super) const ATTRIBUTE: &str = "#[kani::proof]";

pub(super) const EXCUSED: &str = "// not-proved:";

const CI: &str = ".github/workflows/ci.yml";

use crate::architecture::PATH as ARCH;

const STATED: &str = "kani harness";

const ENTRY: &str = "cargo xtask proof";

pub(crate) struct Harness {
    pub(crate) package: String,
    pub(crate) location: String,
    pub(crate) name: String,

    pub(crate) excuse: Option<String>,
}

pub(crate) fn harnesses(root: &Path) -> Result<Vec<Harness>, XtaskError> {
    let mut out = Vec::new();
    for member in members::members(root)? {
        if !member.in_product_graph() {
            continue;
        }
        let src = root.join(&member.dir).join("src");
        if !src.is_dir() {
            continue;
        }
        for file in walk::files_with_ext(&src, &["rs"])? {
            let rel = walk::rel(root, &file);
            let Some(base) = module_base(&walk::rel(&src, &file)) else {
                continue;
            };
            let text = walk::read_text(&file)?;
            out.extend(in_file(&text, &member.package, &rel, &base));
        }
    }
    Ok(out)
}

pub(crate) fn list(root: &Path) -> Result<String, XtaskError> {
    let found = harnesses(root)?;
    let mut out = String::new();
    for harness in found.iter().filter(|h| h.excuse.is_none()) {
        out.push_str(&harness.name);
        out.push('\n');
    }
    Ok(out)
}

#[expect(clippy::disallowed_methods, reason = "developer tool (child D4)")]
pub(crate) fn run(root: &Path) -> Result<String, XtaskError> {
    if !installed(root) {
        return Ok(format!(
            "proof: kani is not installed on this machine; CI's linux job runs it\n{}",
            list(root)?
        ));
    }

    if harnesses(root)?.iter().all(|h| h.excuse.is_some()) {
        return Err(XtaskError::Cmd {
            cmd: ENTRY.to_owned(),
            msg: "every harness carries a not-proved marker; nothing would be proved".to_owned(),
        });
    }
    let found = harnesses(root)?;
    let mut out = String::new();
    for harness in &found {
        if let Some(reason) = &harness.excuse {
            out.push_str(&format!("not proved {}: {reason}\n", harness.name));
        }
    }
    for harness in found.iter().filter(|h| h.excuse.is_none()) {
        let status = Command::new("cargo")
            .current_dir(root)
            .args(["kani", "-p", &harness.package, "--harness", &harness.name])
            .status()
            .map_err(|source| XtaskError::Cmd {
                cmd: format!("cargo kani --harness {}", harness.name),
                msg: source.to_string(),
            })?;
        if !status.success() {
            return Err(XtaskError::Cmd {
                cmd: format!("cargo kani --harness {}", harness.name),
                msg: format!("{} did not hold", harness.location),
            });
        }
        out.push_str(&format!("proved {}\n", harness.name));
    }
    Ok(out)
}

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let found = harnesses(root)?;
    let mut violations = Vec::new();
    workflow_names_no_harness(root, &mut violations)?;
    stated_total_is_the_total(root, found.len(), &mut violations)?;
    every_excuse_cites_where_it_was_decided(&found, &mut violations);
    Ok(violations)
}

pub(crate) fn every_excuse_cites_where_it_was_decided(found: &[Harness], out: &mut Vec<Violation>) {
    for harness in found {
        let Some(reason) = &harness.excuse else {
            continue;
        };
        if [".md", ".lean"]
            .iter()
            .any(|document| reason.contains(document))
        {
            continue;
        }
        out.push(Violation {
            gate: "proof",
            location: harness.location.clone(),
            rule: "a proposition left unproved names where that was decided".to_owned(),
            violation: format!("`{EXCUSED}` here reads {reason:?}, which cites no document"),
            alternative: "cite the SPEC section that records why this shape is out of reach, \
                          or delete the marker and let it be proved"
                .to_owned(),
        });
    }
}

fn workflow_names_no_harness(root: &Path, out: &mut Vec<Violation>) -> Result<(), XtaskError> {
    let text = walk::read_text(&root.join(CI))?;
    let job: Vec<(usize, &str)> = job_lines(&text, "proof");
    if job.is_empty() {
        return Err(XtaskError::Doc {
            file: CI.to_owned(),
            msg: "no `proof:` job; this gate judges a job that must exist".to_owned(),
        });
    }
    for (index, line) in &job {
        if line.contains("--harness") {
            out.push(Violation {
                gate: "proof",
                location: format!("{CI}:{}", index.saturating_add(1)),
                rule: "the harness roster has one home: the `#[kani::proof]` attributes".to_owned(),
                violation: "this step names a harness, so renaming one proves less in silence"
                    .to_owned(),
                alternative: format!("run `{ENTRY}`, which reads the roster out of the source"),
            });
        }
    }
    if !job.iter().any(|(_, line)| line.contains(ENTRY)) {
        out.push(Violation {
            gate: "proof",
            location: CI.to_owned(),
            rule: "the harness roster has one reader in CI".to_owned(),
            violation: format!("the `proof` job never runs `{ENTRY}`"),
            alternative: format!("give the job one step that runs `{ENTRY}`"),
        });
    }
    Ok(())
}

fn stated_total_is_the_total(
    root: &Path,
    total: usize,
    out: &mut Vec<Violation>,
) -> Result<(), XtaskError> {
    let text = walk::read_text(&root.join(ARCH))?;
    let mut stated = 0_usize;
    for (index, line) in text.lines().enumerate() {
        for value in vocabulary::counts_before(line, STATED) {
            stated = stated.saturating_add(1);
            if value != total {
                out.push(Violation {
                    gate: "proof",
                    location: format!("{ARCH}:{}", index.saturating_add(1)),
                    rule: "a count a machine can recount is not written by hand".to_owned(),
                    violation: format!("this line says {value} kani harness(es); {total} exist"),
                    alternative: format!(
                        "correct it to {total}, or state it without a number; \
                         `cargo xtask proof --list` recounts"
                    ),
                });
            }
        }
    }
    if stated == 0 {
        out.push(Violation {
            gate: "proof",
            location: ARCH.to_owned(),
            rule: "the verification table states how many harnesses exist".to_owned(),
            violation: format!(
                "no line states a `{STATED}` count, so nothing can go stale — and \
                                nothing tells a reader the roster shrank"
            ),
            alternative: format!("state `{total} kani harnesses` in the §11 verification table"),
        });
    }
    Ok(())
}

fn job_lines<'a>(text: &'a str, job: &str) -> Vec<(usize, &'a str)> {
    let opener = format!("  {job}:");
    let mut out = Vec::new();
    let mut inside = false;
    for (index, line) in text.lines().enumerate() {
        if line.trim_end() == opener {
            inside = true;
            continue;
        }
        if inside
            && line.starts_with("  ")
            && !line.starts_with("   ")
            && line.trim_start().starts_with(|c: char| c.is_alphanumeric())
        {
            break;
        }
        if inside {
            out.push((index, line));
        }
    }
    out
}

#[expect(clippy::disallowed_methods, reason = "developer tool (child D4)")]
fn installed(root: &Path) -> bool {
    Command::new("cargo")
        .current_dir(root)
        .args(["kani", "--version"])
        .output()
        .is_ok_and(|out| out.status.success())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
