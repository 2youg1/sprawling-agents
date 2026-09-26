// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Module directions inside one crate (xtask-SPEC.md section 8-33).
//!
//! The crate edges in the `depmap` block cannot see a cycle between two
//! modules of the same crate: `sprawling` compiles as one unit whichever
//! way its modules name each other. The `directions` block in
//! ARCHITECTURE.md names, for a module, the paths its production code
//! never names, and this module reads every file of that module for them.
//!
//! The reading is by text, one line at a time. A comment line is not
//! code, and a test is not production: a file named `tests.rs` or
//! ending in `_tests.rs`, a file under a `tests` directory, and the item
//! under a `#[cfg(test)]` line are skipped, because a test may build its fixture through the
//! assembly point without the module it tests depending on it.

use std::collections::BTreeSet;
use std::path::Path;

use crate::report::{Violation, XtaskError};
use crate::walk;

use super::ARCH;

const TEST_ATTRIBUTE: &str = "#[cfg(test)]";

/// One line of the block: a module, and the paths it never names.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Direction {
    /// Repo-relative, without extension: both `<module>.rs` and every
    /// `.rs` file under `<module>/` belong to it.
    module: String,
    forbidden: BTreeSet<String>,
}

pub(super) fn check(
    root: &Path,
    text: &str,
    violations: &mut Vec<Violation>,
) -> Result<(), XtaskError> {
    for direction in parse_block(text)? {
        let files = files_of(root, &direction.module)?;
        if files.is_empty() {
            violations.push(Violation {
                gate: "depmap",
                location: format!("{ARCH} ```directions {}", direction.module),
                rule: "every module the directions block names exists in the tree".to_owned(),
                violation: format!("no `{0}.rs` and no `{0}/` directory", direction.module),
                alternative: "rename the line to the module's present path, or delete it"
                    .to_owned(),
            });
        }
        for file in files {
            let rel = walk::rel(root, &file);
            if is_test_file(&rel) {
                continue;
            }
            let source = walk::read_text(&file)?;
            for (line_no, forbidden) in named(&source, &direction.forbidden) {
                violations.push(Violation {
                    gate: "depmap",
                    location: format!("{rel}:{line_no}"),
                    rule: format!("`{}` never names `{forbidden}`", direction.module),
                    violation: format!("production code names `{forbidden}`"),
                    alternative: format!(
                        "move what this line needs into a module `{forbidden}` depends on, \
                         and let `{forbidden}` hand it over"
                    ),
                });
            }
        }
    }
    Ok(())
}

/// Parse the ```` ```directions ```` fenced block: `module: path, path`.
///
/// # Errors
/// When the block is absent or a line has no colon: a gate whose
/// authority has moved says so rather than judging nothing.
pub(super) fn parse_block(text: &str) -> Result<Vec<Direction>, XtaskError> {
    let doc_error = |msg: String| XtaskError::Doc {
        file: ARCH.to_owned(),
        msg,
    };
    let mut lines = text.lines().map(str::trim);
    if !lines.any(|line| line == "```directions") {
        return Err(doc_error(
            "no ```directions fenced block found in ARCHITECTURE.md".to_owned(),
        ));
    }
    let mut out = Vec::new();
    for line in lines {
        if line == "```" {
            return Ok(out);
        }
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (module, paths) = line
            .split_once(':')
            .filter(|(module, _)| !module.contains("::"))
            .ok_or_else(|| doc_error(format!("directions line without a module: {line:?}")))?;
        out.push(Direction {
            module: module.trim().to_owned(),
            forbidden: paths
                .split(',')
                .map(str::trim)
                .filter(|path| !path.is_empty())
                .map(str::to_owned)
                .collect(),
        });
    }
    Err(doc_error(
        "the ```directions block is never closed".to_owned(),
    ))
}

fn files_of(root: &Path, module: &str) -> Result<Vec<std::path::PathBuf>, XtaskError> {
    let dir = root.join(module);
    let mut files = if dir.is_dir() {
        walk::files_with_ext(&dir, &["rs"])?
    } else {
        Vec::new()
    };
    let head = root.join(format!("{module}.rs"));
    if head.is_file() {
        files.insert(0, head);
    }
    Ok(files)
}

fn is_test_file(rel: &str) -> bool {
    rel.split('/').any(|segment| {
        segment == "tests" || segment == "tests.rs" || segment.ends_with("_tests.rs")
    })
}

/// Every production line that names a forbidden path, with its 1-based
/// number. The item under `#[cfg(test)]` is skipped whole: one line when
/// it ends in `;`, otherwise until its braces close.
pub(super) fn named<'a>(source: &str, forbidden: &'a BTreeSet<String>) -> Vec<(usize, &'a str)> {
    let mut found = Vec::new();
    let mut skipping = Skip::No;
    for (index, line) in source.lines().enumerate() {
        let code = line.trim();
        skipping = match skipping {
            Skip::No if code == TEST_ATTRIBUTE => Skip::Attribute,
            Skip::No => {
                if !code.starts_with("//") {
                    found.extend(
                        forbidden
                            .iter()
                            .filter(|path| names(code, path))
                            .map(|path| (index.saturating_add(1), path.as_str())),
                    );
                }
                Skip::No
            }
            Skip::Attribute if code.starts_with("#[") || code.is_empty() => Skip::Attribute,
            Skip::Attribute | Skip::Body(_) => after(skipping, code),
        };
    }
    found
}

/// Where a `#[cfg(test)]` item stands after one more of its lines.
#[derive(Clone, Copy)]
enum Skip {
    No,
    Attribute,
    Body(usize),
}

fn after(skipping: Skip, code: &str) -> Skip {
    let depth = match skipping {
        Skip::Body(depth) => depth,
        Skip::No | Skip::Attribute => 0,
    };
    let code = without_literals(code);
    let opened = depth.saturating_add(code.matches('{').count());
    let closed = code.matches('}').count();
    match opened.checked_sub(closed) {
        Some(0) | None if code.ends_with(';') || code.ends_with('}') => Skip::No,
        Some(0) | None => Skip::Body(0),
        Some(left) => Skip::Body(left),
    }
}

/// `code` with its string and char literals removed, so the braces left
/// are the ones that open and close blocks. A `'` that does not start a
/// char literal starts a lifetime and is dropped alone.
fn without_literals(code: &str) -> String {
    let mut kept = String::with_capacity(code.len());
    let mut chars = code.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => skip_past(&mut chars, '"'),
            '\'' => {
                let mut ahead = chars.clone();
                match (ahead.next(), ahead.next()) {
                    (Some('\\'), _) => {
                        chars.next();
                        chars.next();
                        skip_past(&mut chars, '\'');
                    }
                    (Some(_), Some('\'')) => {
                        chars.next();
                        chars.next();
                    }
                    _ => {}
                }
            }
            other => kept.push(other),
        }
    }
    kept
}

/// Advance past the next `end` that no backslash escapes.
fn skip_past(chars: &mut std::str::Chars<'_>, end: char) {
    while let Some(c) = chars.next() {
        if c == '\\' {
            chars.next();
        } else if c == end {
            return;
        }
    }
}

/// Whether `code` names `path` as a whole path segment, so that
/// `crate::assembly` is not found inside `crate::assembly_line`.
fn names(code: &str, path: &str) -> bool {
    code.match_indices(path).any(|(at, _)| {
        code.get(at.saturating_add(path.len())..)
            .and_then(|rest| rest.chars().next())
            .is_none_or(|next| !(next.is_alphanumeric() || next == '_'))
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn forbidden() -> BTreeSet<String> {
        BTreeSet::from(["crate::assembly".to_owned()])
    }

    #[test]
    fn a_production_line_is_found_and_a_test_item_is_not() {
        let source = "use crate::assembly::McpLink;\n\
                      use crate::assembly_line;\n\
                      // crate::assembly in a comment\n\
                      #[cfg(test)]\n\
                      use crate::assembly::init_city;\n\
                      fn f() {}\n\
                      #[cfg(test)]\n\
                      #[allow(clippy::unwrap_used)]\n\
                      mod tests {\n\
                          fn g() {\n\
                              crate::assembly::init_city();\n\
                          }\n\
                      }\n\
                      fn h() { crate::assembly::now_ms(); }\n";
        let forbidden = forbidden();
        assert_eq!(
            named(source, &forbidden),
            vec![(1, "crate::assembly"), (14, "crate::assembly")]
        );
    }

    #[test]
    fn a_brace_inside_a_literal_does_not_extend_a_test_item() {
        let source = "#[cfg(test)]\n\
                      mod tests {\n\
                          fn g() { let open = \"{\"; let close = '}'; let quoted: &'static str = \"\\\"{\"; }\n\
                      }\n\
                      fn h() { crate::assembly::now_ms(); }\n";
        let forbidden = forbidden();
        assert_eq!(named(source, &forbidden), vec![(5, "crate::assembly")]);
    }

    #[test]
    fn the_block_reads_modules_and_paths() {
        let text = "```directions\n# comment\ncrates/x/src/views: crate::assembly\n```\n";
        assert_eq!(
            parse_block(text).unwrap(),
            vec![Direction {
                module: "crates/x/src/views".to_owned(),
                forbidden: forbidden(),
            }]
        );
    }

    /// A test module split off beside the code it judges is named
    /// `<subject>_tests.rs`, and is test code as much as `tests.rs` is.
    #[test]
    fn a_split_off_test_file_is_not_production() {
        assert_eq!(
            [
                "crates/x/src/views/standing_tests.rs",
                "crates/x/src/views/tests/holding.rs",
                "crates/x/src/views/testsuite.rs",
            ]
            .map(is_test_file),
            [true, true, false]
        );
    }

    #[test]
    fn a_missing_block_is_a_doc_error() {
        assert!(parse_block("```depmap\n```\n").is_err());
    }
}
