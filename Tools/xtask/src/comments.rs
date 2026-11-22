// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use crate::report::{Violation, XtaskError};
use crate::walk;

mod lexer;
mod python;

use lexer::Language;

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    check_with(root, None)
}

pub(crate) fn check_scoped(
    root: &Path,
    scope: &crate::source::Scope,
) -> Result<Vec<Violation>, XtaskError> {
    check_with(root, Some(scope))
}

fn check_with(
    root: &Path,
    scope: Option<&crate::source::Scope>,
) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    for file in crate::source::comment_files_in(root, scope)? {
        let Some(language) = language(&file) else {
            continue;
        };
        let text = walk::read_text(&file)?;
        let mut reading = lexer::measure(&text, language);
        if language == Language::Python {
            reading.comment_lines = reading
                .comment_lines
                .saturating_add(python::documentation_lines(&file, &walk::rel(root, &file))?);
        }
        if reading.comment_lines > 0 {
            let rel = walk::rel(root, &file);
            violations.push(Violation {
                gate: "comments",
                location: rel,
                rule: "only the exact leading MPL header may be a comment".to_owned(),
                violation: format!("{} non-MPL comment lines", reading.comment_lines),
                alternative: "make the invalid state unrepresentable or enforce the constraint with a concrete lint, test or gate, never prose".to_owned(),
            });
        }
    }
    Ok(violations)
}

fn language(file: &Path) -> Option<Language> {
    match file.extension().and_then(|part| part.to_str()) {
        Some("rs") => Some(Language::Rust),
        Some("ts" | "tsx") => Some(Language::TypeScript),
        Some("js" | "mjs") => Some(Language::JavaScript),
        Some("svelte") => Some(Language::Svelte),
        Some("zig") => Some(Language::Zig),
        Some("py") => Some(Language::Python),
        Some("css") => Some(Language::Css),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
