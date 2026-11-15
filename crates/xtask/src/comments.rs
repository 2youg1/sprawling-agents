// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Comment-density gate. Necessity is a writing rule; this gate only
//! stops a file from accumulating more prose than the shared budget.

use std::path::Path;

use crate::report::{Violation, XtaskError};
use crate::walk;

mod lexer;

use lexer::{Language, Reading};

pub(crate) const COMMENT_PERCENT_LIMIT: u128 = 15;

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
    for file in crate::source::files_in(root, scope)? {
        let Some(language) = language(&file) else {
            continue;
        };
        let text = walk::read_text(&file)?;
        let reading = lexer::measure(&text, language);
        if over(reading) {
            let rel = walk::rel(root, &file);
            violations.push(Violation {
                gate: "comments",
                location: rel,
                rule: format!(
                    "comment-bearing lines are at most {COMMENT_PERCENT_LIMIT}% of nonblank lines after the MPL header"
                ),
                violation: format!(
                    "{} comment lines among {} judged lines ({}%)",
                    reading.comment_lines,
                    reading.judged_lines,
                    percentage(reading)
                ),
                alternative: "rethink from zero comments; keep only what must exist".to_owned(),
            });
        }
    }
    Ok(violations)
}

fn language(file: &Path) -> Option<Language> {
    match file.extension().and_then(|part| part.to_str()) {
        Some("rs") => Some(Language::Rust),
        Some("ts" | "tsx") => Some(Language::TypeScript),
        Some("svelte") => Some(Language::Svelte),
        _ => None,
    }
}

fn over(reading: Reading) -> bool {
    let comments = u128::try_from(reading.comment_lines).unwrap_or(u128::MAX);
    let judged = u128::try_from(reading.judged_lines).unwrap_or(u128::MAX);
    comments.saturating_mul(100) > judged.saturating_mul(COMMENT_PERCENT_LIMIT)
}

fn percentage(reading: Reading) -> String {
    let comments = u128::try_from(reading.comment_lines).unwrap_or(0);
    let judged = u128::try_from(reading.judged_lines).unwrap_or(0);
    let tenths = comments
        .checked_mul(1000)
        .and_then(|value| value.checked_div(judged))
        .unwrap_or(0);
    let whole = tenths.checked_div(10).unwrap_or(0);
    let fraction = tenths.checked_rem(10).unwrap_or(0);
    format!("{whole}.{fraction}")
}

#[cfg(test)]
mod tests;
