// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Motion gate: a transition's curve and duration are named once, in the
//! client's theme (tools/xtask/Spec.lean §8-51, docs/frontend-method.md §4-43).
//!
//! The same shape as the colour scan: one production point, the theme's
//! entry and the parts it imports (`crate::theme`), and every
//! other file in the client refused the spellings that would make it a
//! second one. A curve is a timing function; a duration is what Tailwind
//! turns `duration-150` into. Both have tokens - `ease-arrive`,
//! `ease-leave`, `duration-short`, `duration-panel`, `duration-page` - and
//! a view that writes its own is how one kind of movement came to have
//! five answers.
//!
//! **What it does not read**: an inline `animation-duration` or
//! `animation-delay` (the city drawing's ambient loops and the stagger
//! between its figures are not movements across the page), and `delay-*`
//! (a hover hint's intent threshold, client/Spec.lean §4-18, not a duration of
//! motion).

use std::path::Path;

use crate::report::{Violation, XtaskError};
use crate::theme;
use crate::walk;

/// The client's file kinds that carry a class or a style.
const SCAN_EXTS: [&str; 4] = ["svelte", "ts", "css", "html"];

/// The two CSS timing functions whose names are also ordinary words in
/// code: each counts only when its first argument is a number, which is
/// all either one takes. `{#snippet steps(each)}` is a snippet.
const NUMERIC_FUNCTIONS: [&str; 2] = ["linear(", "steps("];

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    for path in walk::files_with_ext(&root.join(walk::CLIENT_SRC), &SCAN_EXTS)? {
        let rel = walk::rel(root, &path);
        if theme::is_theme(&rel) {
            continue;
        }
        let text = walk::read_text(&path)?;
        for (index, line) in text.lines().enumerate() {
            let Some(spelling) = literal_at(line) else {
                continue;
            };
            violations.push(Violation {
                gate: "motion",
                location: format!("{rel}:{}", index.saturating_add(1)),
                rule: "a transition's curve and duration are named once, in the client's theme"
                    .to_owned(),
                violation: format!(
                    "`{spelling}` outside the theme ({} and the parts it imports)",
                    theme::ENTRY
                ),
                alternative: "spell `duration-short`, `duration-panel` or `duration-page` and \
                              `ease-arrive` or `ease-leave` (docs/frontend-method.md §4-43); a \
                              fourth duration or a third curve is declared in the theme's \
                              motion tokens first"
                    .to_owned(),
            });
        }
    }
    Ok(violations)
}

/// The first spelling on this line the gate refuses.
fn literal_at(line: &str) -> Option<&'static str> {
    if line.contains("cubic-bezier(") {
        return Some("cubic-bezier(");
    }
    if let Some(function) = NUMERIC_FUNCTIONS
        .into_iter()
        .find(|function| takes_a_number(line, function))
    {
        return Some(function);
    }
    tailwind_literal(line)
}

/// Whether `function` appears with a number as its first argument.
fn takes_a_number(line: &str, function: &str) -> bool {
    line.match_indices(function).any(|(at, _)| {
        line.get(at.saturating_add(function.len())..)
            .and_then(|rest| rest.trim_start().bytes().next())
            .is_some_and(|byte| byte.is_ascii_digit() || matches!(byte, b'.' | b'-'))
    })
}

/// `duration-150`, `duration-[90ms]` or `ease-[…]`, as a class: the
/// utility starts where nothing that continues a class name stands
/// before it, so a variant's `:` begins one and `--transition-duration-`
/// does not.
fn tailwind_literal(line: &str) -> Option<&'static str> {
    let starts = |at: usize| {
        line.get(..at)
            .and_then(|head| head.bytes().next_back())
            .is_none_or(|byte| !(byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')))
    };
    for (utility, spelled) in [("duration-", "duration-<n>"), ("ease-[", "ease-[")] {
        for (at, _) in line.match_indices(utility) {
            let next = line
                .get(at.saturating_add(utility.len())..)
                .and_then(|rest| rest.bytes().next());
            let literal = utility == "ease-["
                || next.is_some_and(|byte| byte.is_ascii_digit() || byte == b'[');
            if literal && starts(at) {
                return Some(spelled);
            }
        }
    }
    None
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    use super::{check, literal_at};

    /// A curve declared in a part the theme imports is the theme's own
    /// spelling; the same curve in a view is a second home for it.
    #[test]
    fn a_part_the_theme_imports_may_spell_a_curve() {
        let root = std::env::temp_dir().join(format!("motion-theme-part-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let written = [
            (
                "client/src/theme.css",
                "@import \"tailwindcss\";
@import \"./theme/tokens-motion.css\";
",
            ),
            (
                "client/src/theme/tokens-motion.css",
                "@theme {
  --ease-arrive: cubic-bezier(0, 0, 0.1, 1);
}
",
            ),
            (
                "client/src/views/panel.svelte",
                "<div style=\"transition: x 1s cubic-bezier(0, 0, 0.1, 1)\"></div>
",
            ),
        ];
        for (rel, body) in written {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, body).unwrap();
        }

        let found = check(&root).unwrap();
        assert_eq!(
            found
                .iter()
                .map(|v| v.location.as_str())
                .collect::<Vec<_>>(),
            ["client/src/views/panel.svelte:1"]
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_three_timing_functions_are_refused_and_a_snippet_named_steps_is_not() {
        assert_eq!(
            literal_at("transition: x 1s cubic-bezier(0.2, 0, 0, 1);"),
            Some("cubic-bezier(")
        );
        assert_eq!(
            literal_at("animation-timing-function: linear(0, 0.5 50%, 1);"),
            Some("linear(")
        );
        assert_eq!(
            literal_at("animation: tick 1s steps(4) infinite;"),
            Some("steps(")
        );
        assert_eq!(literal_at("{#snippet steps(each: Walk)}"), None);
        assert_eq!(literal_at("const rate = linear(width);"), None);
    }

    #[test]
    fn tailwind_spells_a_duration_or_a_curve_as_a_literal_only_with_a_number_or_brackets() {
        assert_eq!(
            literal_at(r#"class="transition-colors duration-150""#),
            Some("duration-<n>")
        );
        assert_eq!(
            literal_at(r#"class="hover:duration-[90ms]""#),
            Some("duration-<n>")
        );
        assert_eq!(literal_at(r#"class="ease-[cubic]""#), Some("ease-["));
        assert_eq!(
            literal_at(r#"class="duration-panel ease-arrive open:ease-leave""#),
            None
        );
        assert_eq!(
            literal_at("transition: x var(--transition-duration-short);"),
            None
        );
        assert_eq!(literal_at("animation: none;"), None);
    }
}
