// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The fifth assertion of `specalign`: every module row's `spec` anchor
//! resolves to where that module is specified (xtask-SPEC.md sections
//! 8-10 and 8-43).
//!
//! A package that has not migrated names a section of its Markdown SPEC,
//! `<crate>-SPEC.md#8-N`. A package that has migrated names the Lean
//! module that specifies the row, `crates.browser.spec.Act`, and that
//! module has to be a file inside the same package: a row cites its own
//! crate's specification. Which of the two forms a row may use is decided
//! by one fact, whether the package holds a `Spec.lean`, so a crate never
//! answers in both.

use std::collections::BTreeMap;
use std::path::Path;

use crate::lean;
use crate::members::{self, Member};
use crate::modmap::{self, Anchor, MAP};
use crate::report::{Violation, XtaskError};
use crate::walk;

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
pub(super) fn section_present(spec: &str, section: &str) -> bool {
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

/// Every module row's anchor resolves.
///
/// Existence is asserted, uniqueness is not: section numbers repeat
/// inside several Markdown SPECs because successive cards numbered
/// independently, and renumbering them is its own work (xtask-SPEC.md
/// section 8-10 records the condition for tightening this).
pub(super) fn check(root: &Path, violations: &mut Vec<Violation>) -> Result<(), XtaskError> {
    let found = members::members(root)?;
    let dirs: Vec<String> = found.iter().map(|member| member.dir.clone()).collect();
    let mut loaded: BTreeMap<String, Spec> = BTreeMap::new();
    for anchor in modmap::anchors(root)? {
        let at = format!("{MAP}: {}", anchor.module);
        let owner = found.iter().find(|member| member.holds(&anchor.file));
        let migrated = owner.is_some_and(|member| lean::migrated(root, &member.dir));
        let markdown = anchor.spec.contains('#') || anchor.spec.ends_with(".md");
        let problem = match (false, true, owner) {
            (true, false, Some(member)) => lean_part(root, &anchor, member),
            (true, true, _) => Some((
                "a migrated crate's rows name the Lean part that specifies them \
                 (xtask-SPEC.md section 8-43)",
                format!(
                    "{}: {:?} names a Markdown SPEC, and this crate's specification is {}",
                    anchor.module,
                    anchor.spec,
                    lean::ENTRY
                ),
                "write the module name of the part, such as `crates.browser.spec.Act`, or \
                 `crates.browser.Spec` where the entry specifies the module",
            )),
            (false, false, _) | (true, false, None) => Some((
                "a crate names its Lean parts once it has a Spec.lean (xtask-SPEC.md section 8-43)",
                format!(
                    "{}: {:?} is a Lean module name, and this crate has no {}",
                    anchor.module,
                    anchor.spec,
                    lean::ENTRY
                ),
                "name the section of the crate's Markdown SPEC, `<crate>-SPEC.md#8-N`, until \
                 the crate migrates",
            )),
            (false, true, _) => markdown_section(root, &dirs, &mut loaded, &anchor),
        };
        if let Some((rule, violation, alternative)) = problem {
            violations.push(anchored(at, rule, violation, alternative));
        }
    }
    Ok(())
}

/// A migrated crate's anchor: the module it names is a file of that
/// crate.
fn lean_part(
    root: &Path,
    anchor: &Anchor,
    owner: &Member,
) -> Option<(&'static str, String, &'static str)> {
    let file = lean::module_file(&anchor.spec);
    if !owner.holds(&file) {
        return Some((
            "a row cites its own crate's specification (xtask-SPEC.md section 8-43)",
            format!(
                "{}: {:?} is not under {}, the directory of the crate this row belongs to",
                anchor.module, anchor.spec, owner.dir
            ),
            "name a part of this crate, or move the row to the crate the module belongs to",
        ));
    }
    if root.join(&file).is_file() {
        return None;
    }
    Some((
        "the Spec column points at a part that exists (xtask-SPEC.md section 8-43)",
        format!("{}: {file} is not on disk", anchor.module),
        "correct the module name, or write that part",
    ))
}

/// A Markdown anchor, `<crate>-SPEC.md#8-N`, of a crate that has not
/// migrated.
fn markdown_section(
    root: &Path,
    dirs: &[String],
    loaded: &mut BTreeMap<String, Spec>,
    anchor: &Anchor,
) -> Option<(&'static str, String, &'static str)> {
    let Some((file, section)) = anchor.spec.split_once('#') else {
        return Some((
            "the Spec column is `<crate>-SPEC.md#8-N` (xtask-SPEC.md section 8-10)",
            format!("{}: {:?} has no section", anchor.module, anchor.spec),
            "write the SPEC file and the section it is specified in",
        ));
    };
    if !file.ends_with("-SPEC.md") {
        return Some((
            "the Spec column names a crate SPEC (xtask-SPEC.md section 8-10)",
            format!("{}: {file:?} is not a `<crate>-SPEC.md`", anchor.module),
            "name the SPEC of the crate the module lives in",
        ));
    }
    let spec = &*loaded
        .entry(file.to_owned())
        .or_insert_with(|| Spec::find(root, dirs, file));
    let exists = "the Spec column points at a SPEC that exists (xtask-SPEC.md section 8-10)";
    match spec {
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
    }
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::{check, section_present};
    use crate::root::fixture;

    #[test]
    fn a_section_is_found_as_a_heading_or_as_a_fence_marker() {
        assert!(section_present("### 8-27 kernel::gate", "8-27"));
        assert!(section_present("// 8-1 port（形状 3）", "8-1"));
        assert!(section_present("## 8-48 `kernel::node_id`", "8-48"));
        assert!(!section_present("### 8-27 kernel::gate", "8-2"));
        assert!(!section_present("### 8-7 six tools", "8-70"));
        assert!(!section_present("a paragraph mentioning 8-27", "8-27"));
    }

    /// Every violation `check` reports on the fixture, as its text.
    fn found(root: &std::path::Path) -> Vec<String> {
        let mut out = Vec::new();
        let ran = check(root, &mut out);
        std::fs::remove_dir_all(root).unwrap();
        ran.unwrap();
        out.into_iter().map(|v| v.violation).collect()
    }

    /// A module row under tools/ is read, and its anchor resolves in the SPEC
    /// beside its package, which has no section 8-9.
    #[test]
    fn a_relocated_package_has_its_anchors_checked() {
        let root = fixture::relocated("specalign");
        fixture::write(&root, "tools/k/k-SPEC.md", "# k\n\n### 8-1 a\n");
        assert_eq!(found(&root), ["k::a: k-SPEC.md has no section 8-9"]);
    }

    /// A crate that migrated names its parts; a part that is not on disk,
    /// a part of another crate and a Markdown anchor are each refused.
    #[test]
    fn a_migrated_crate_anchors_its_rows_to_lean_parts_of_its_own() {
        let root = fixture::relocated("specalign-lean");
        fixture::write(&root, "tools/k/Spec.lean", "");
        fixture::write(&root, "tools/k/spec/A.lean", "");
        fixture::write(&root, "tools/k/src/b.rs", "");
        fixture::write(&root, "tools/k/src/c.rs", "");
        fixture::write(&root, "tools/k/src/d.rs", "");
        let row = |name: &str, spec: &str| {
            format!(
                "  {{ name = \"k::{name}\", file = \"tools/k/src/{name}.rs\", owns = \"x\", \
                 shape = \"value\", since = \"T\", status = \"built\", spec = \"{spec}\" }},\n"
            )
        };
        let map = [
            row("a", "tools.k.spec.A"),
            row("b", "tools.k.spec.B"),
            row("c", "crates.j.Spec"),
            row("d", "k-SPEC.md#8-9"),
        ]
        .concat();
        fixture::write(&root, "architecture.toml", &format!("module = [\n{map}]\n"));
        let found = found(&root);
        assert_eq!(
            found,
            [
                "k::b: tools/k/spec/B.lean is not on disk".to_owned(),
                "k::c: \"crates.j.Spec\" is not under tools/k, the directory of the crate this \
                 row belongs to"
                    .to_owned(),
                "k::d: \"k-SPEC.md#8-9\" names a Markdown SPEC, and this crate's specification \
                 is Spec.lean"
                    .to_owned(),
            ]
        );
    }
}
