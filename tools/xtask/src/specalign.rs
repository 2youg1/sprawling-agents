// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Gate: kernel enums and the kernel-SPEC tables agree variant by
//! variant (C8), and every module's `Spec` anchor resolves. The gate
//! consumes the real enums — `AxCode::ALL` and `EventKind::ALL` — and
//! reads the SPEC as data, so "the table drifted" and "the enum grew
//! silently" are the same red. Asserts: every AxCode appears exactly
//! once in the 8-1 table with its declared carrier; every EventKind
//! exactly once in the 8-4 table with its window class; every other
//! `pub enum` body in the SPEC holds the variants the kernel compiles
//! (`enums`); and the seventh module-table column names a SPEC section
//! that is on disk.

use std::collections::BTreeMap;
use std::path::Path;

use kernel::{AxCode, Carrier, EventKind, WindowClass};

use crate::members;
use crate::modmap;
use crate::report::{Violation, XtaskError};
use crate::walk;

mod enums;

const SPEC_PATH: &str = "crates/kernel/kernel-SPEC.md";
use crate::modmap::MAP;

fn violation(rule: &str, violation: String, alternative: &str) -> Violation {
    Violation {
        gate: "specalign",
        location: SPEC_PATH.to_owned(),
        rule: rule.to_owned(),
        violation,
        alternative: alternative.to_owned(),
    }
}

/// Splits a pipe-table line into trimmed cells, or None for non-rows.
fn cells(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim();
    let inner = trimmed.strip_prefix('|')?.strip_suffix('|')?;
    Some(inner.split('|').map(|c| c.trim().to_owned()).collect())
}

fn unticked(cell: &str) -> Option<&str> {
    cell.strip_prefix('`')?.strip_suffix('`')
}

/// The carrier cell a code must show, derived from the enum itself.
fn expected_carrier(code: AxCode) -> Result<String, XtaskError> {
    match code.carrier() {
        Carrier::Loadtime => Ok("loadtime".to_owned()),
        Carrier::Event(kind) => serde_json::to_value(kind)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .ok_or_else(|| XtaskError::Doc {
                file: "kernel enums".to_owned(),
                msg: "EventKind must serialize to a string".to_owned(),
            }),
    }
}

/// Normalizes a SPEC carrier cell: "`tool_result`" -> tool_result,
/// "装载期（无 carrier）" -> loadtime.
fn spec_carrier(cell: &str) -> String {
    if let Some(name) = unticked(cell) {
        name.to_owned()
    } else if cell.contains("carrier") || cell.contains("\u{88c5}\u{8f7d}\u{671f}") {
        "loadtime".to_owned()
    } else {
        cell.to_owned()
    }
}

/// What a `<crate>-SPEC.md` cell finds on disk, looked for in every
/// package directory (xtask-SPEC.md section 8-10). The file name is the
/// whole address: which directory holds it is cargo's answer, never a
/// path spelled here.
enum Spec {
    Text(String),
    Unreadable(String),
    Missing,
    Ambiguous(Vec<String>),
}

impl Spec {
    fn find(root: &Path, dirs: &[String], file: &str) -> Self {
        let holders: Vec<String> = dirs
            .iter()
            .map(|dir| format!("{dir}/{file}"))
            .filter(|path| root.join(path).is_file())
            .collect();
        match holders.as_slice() {
            [] => Self::Missing,
            [path] => match walk::read_text(&root.join(path)) {
                Ok(text) => Self::Text(text),
                Err(err) => Self::Unreadable(err.to_string()),
            },
            [..] => Self::Ambiguous(holders.clone()),
        }
    }
}

/// True when `section` labels a section of `spec`.
///
/// A SPEC's section 8 is written one of two ways, and both are the real
/// shape of this tree: a `### 8-N …` heading, or a `// 8-N …` line inside
/// the one interface fence that is section 8 (browser, protocol, desktop
/// and parts of web and runtime write it that way). Accepting both is not
/// a widening — accepting only headings would redden six crates for how
/// their SPEC has always been written.
pub(crate) fn section_present(spec: &str, section: &str) -> bool {
    spec.lines().any(|line| {
        let trimmed = line.trim_start();
        let body = trimmed
            .trim_start_matches('#')
            .strip_prefix(" ")
            .or_else(|| trimmed.strip_prefix("// "));
        let Some(body) = body else { return false };
        if trimmed.starts_with('#') && !trimmed.starts_with("##") {
            return false;
        }
        body.strip_prefix(section)
            .is_some_and(|tail| !tail.starts_with(|c: char| c.is_ascii_digit()))
    })
}

/// Every module row's seventh cell resolves to a SPEC section on disk.
///
/// The cell is `<crate>-SPEC.md#8-N`. Existence is asserted, uniqueness is
/// not: section numbers repeat inside several SPECs because successive
/// cards numbered independently, and renumbering them is its own work
/// (xtask-SPEC.md section 8-10 records the condition for tightening this).
fn check_anchors(root: &Path, violations: &mut Vec<Violation>) -> Result<(), XtaskError> {
    let dirs: Vec<String> = members::members(root)?
        .into_iter()
        .map(|member| member.dir)
        .collect();
    let mut loaded: BTreeMap<String, Spec> = BTreeMap::new();
    for anchor in modmap::anchors(root)? {
        let at = format!("{MAP}: {}", anchor.module);
        let Some((file, section)) = anchor.spec.split_once('#') else {
            violations.push(anchored(
                at,
                "the Spec column is `<crate>-SPEC.md#8-N` (xtask-SPEC.md section 8-10)",
                format!("{}: {:?} has no section", anchor.module, anchor.spec),
                "write the SPEC file and the section it is specified in",
            ));
            continue;
        };
        if !file.ends_with("-SPEC.md") {
            violations.push(anchored(
                at,
                "the Spec column names a crate SPEC (xtask-SPEC.md section 8-10)",
                format!("{}: {file:?} is not a `<crate>-SPEC.md`", anchor.module),
                "name the SPEC of the crate the module lives in",
            ));
            continue;
        }
        let spec = &*loaded
            .entry(file.to_owned())
            .or_insert_with(|| Spec::find(root, &dirs, file));
        let exists = "the Spec column points at a SPEC that exists (xtask-SPEC.md section 8-10)";
        let problem = match spec {
            Spec::Text(text) if section_present(text, section) => None,
            Spec::Text(_) => Some((
                "the Spec anchor names a section that exists (xtask-SPEC.md section 8-10)",
                format!("{}: {file} has no section {section}", anchor.module),
                "write that section, or point the row at the section that does specify this module",
            )),
            Spec::Unreadable(why) => Some((
                exists,
                format!("{}: {file} is not readable: {why}", anchor.module),
                "correct the crate name, or write that SPEC",
            )),
            Spec::Missing => Some((
                exists,
                format!("{}: no package directory holds {file}", anchor.module),
                "correct the crate name, or write that SPEC",
            )),
            Spec::Ambiguous(paths) => Some((
                "a SPEC file name belongs to one package (xtask-SPEC.md section 8-10)",
                format!("{}: {file} is at {}", anchor.module, paths.join(" and ")),
                "keep one of the files, so the anchor names one SPEC",
            )),
        };
        if let Some((rule, violation, alternative)) = problem {
            violations.push(anchored(at, rule, violation, alternative));
        }
    }
    Ok(())
}

/// One finding about a module row's anchor, located at that row.
fn anchored(at: String, rule: &str, violation: String, alternative: &str) -> Violation {
    Violation {
        gate: "specalign",
        location: at,
        rule: rule.to_owned(),
        violation,
        alternative: alternative.to_owned(),
    }
}

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let text = walk::read_text(&root.join(SPEC_PATH))?;
    let mut ax_rows: BTreeMap<String, String> = BTreeMap::new();
    let mut kind_rows: BTreeMap<String, String> = BTreeMap::new();
    let mut violations = Vec::new();

    for line in text.lines() {
        let Some(row) = cells(line) else { continue };
        let (Some(second), third) = (row.get(1), row.get(2)) else {
            continue;
        };
        let Some(name) = unticked(second) else {
            continue;
        };
        let Some(third) = third else { continue };
        if name.starts_with("E_") {
            if ax_rows
                .insert(name.to_owned(), spec_carrier(third))
                .is_some()
            {
                violations.push(violation(
                    "each AxCode owns exactly one carrier row (C9)",
                    format!("`{name}` appears more than once in the 8-1 table"),
                    "delete the duplicate row",
                ));
            }
        } else if third.contains("in-window") || third.contains("record-only") {
            let class = if third.contains("in-window") {
                "in-window"
            } else {
                "record-only"
            };
            if kind_rows
                .insert(name.to_owned(), class.to_owned())
                .is_some()
            {
                violations.push(violation(
                    "each EventKind belongs to exactly one partition (3.1)",
                    format!("`{name}` appears more than once in the 8-4 table"),
                    "delete the duplicate row",
                ));
            }
        }
    }

    for code in AxCode::ALL {
        let spelling = code.as_str();
        match ax_rows.remove(spelling) {
            None => violations.push(violation(
                "every AxCode variant has its 8-1 table row (C8)",
                format!("`{spelling}` is in the enum but not in the table"),
                "add the row (and its carrier) in the same change-set as the variant",
            )),
            Some(cell) => {
                let expected = expected_carrier(code)?;
                if cell != expected {
                    violations.push(violation(
                        "the table carrier equals the enum's declaration (C9)",
                        format!("`{spelling}`: table says `{cell}`, enum says `{expected}`"),
                        "fix whichever side no longer matches the other",
                    ));
                }
            }
        }
    }
    for (orphan, _) in ax_rows {
        violations.push(violation(
            "the 8-1 table lists enum variants only (C8)",
            format!("`{orphan}` is in the table but not in the enum"),
            "remove the row or add the variant in the same change-set",
        ));
    }

    for kind in EventKind::ALL {
        let value = serde_json::to_value(kind).map_err(|err| XtaskError::Doc {
            file: "kernel enums".to_owned(),
            msg: format!("EventKind serialization failed: {err}"),
        })?;
        let Some(spelling) = value.as_str().map(str::to_owned) else {
            return Err(XtaskError::Doc {
                file: "kernel enums".to_owned(),
                msg: "EventKind must serialize to a string".to_owned(),
            });
        };
        let expected = match kind.window_class() {
            WindowClass::InWindow => "in-window",
            WindowClass::RecordOnly => "record-only",
        };
        match kind_rows.remove(&spelling) {
            None => violations.push(violation(
                "every EventKind variant has its 8-4 table row (C8)",
                format!("`{spelling}` is in the enum but not in the table"),
                "add the row (and its partition) in the same change-set as the variant",
            )),
            Some(class) => {
                if class != expected {
                    violations.push(violation(
                        "the table partition equals the enum's window class (3.1)",
                        format!("`{spelling}`: table says {class}, enum says {expected}"),
                        "fix whichever side no longer matches the other",
                    ));
                }
            }
        }
    }
    for (orphan, _) in kind_rows {
        violations.push(violation(
            "the 8-4 table lists enum variants only (C8)",
            format!("`{orphan}` is in the table but not in the enum"),
            "remove the row or add the variant in the same change-set",
        ));
    }

    enums::check(root, &text, SPEC_PATH, &mut violations)?;
    check_anchors(root, &mut violations)?;

    Ok(violations)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::{cells, section_present, spec_carrier, unticked};

    #[test]
    fn table_rows_split_and_untick() {
        let row = cells("| a | `E_TIMEOUT` | `tool_result` |").unwrap();
        assert_eq!(row.len(), 3);
        assert_eq!(unticked(row.get(1).unwrap()).unwrap(), "E_TIMEOUT");
        assert!(cells("not a row").is_none());
        assert!(unticked("plain").is_none());
    }

    #[test]
    fn carrier_cells_normalize() {
        assert_eq!(spec_carrier("`gate_denied`"), "gate_denied");
        assert_eq!(spec_carrier("装载期（无 carrier）"), "loadtime");
    }

    #[test]
    fn a_section_is_found_as_a_heading_or_as_a_fence_marker() {
        assert!(section_present("### 8-27 kernel::gate", "8-27"));
        assert!(section_present("// 8-1 port（形状 3）", "8-1"));
        assert!(section_present("## 8-48 `kernel::node_id`", "8-48"));
        assert!(!section_present("### 8-27 kernel::gate", "8-2"));
        assert!(!section_present("### 8-7 six tools", "8-70"));
        assert!(!section_present("a paragraph mentioning 8-27", "8-27"));
    }

    /// A module row under tools/ is read, and its anchor resolves in the SPEC
    /// beside its package, which has no section 8-9.
    #[test]
    fn a_relocated_package_has_its_anchors_checked() {
        let root = crate::root::fixture::relocated("specalign");
        crate::root::fixture::write(&root, "tools/k/k-SPEC.md", "# k\n\n### 8-1 a\n");
        let mut found = Vec::new();
        let ran = super::check_anchors(&root, &mut found);
        std::fs::remove_dir_all(&root).unwrap();
        assert!(
            ran.is_ok()
                && found
                    .iter()
                    .any(|v| v.violation.contains("k-SPEC.md has no section 8-9")),
            "{ran:?} {found:?}"
        );
    }
}
