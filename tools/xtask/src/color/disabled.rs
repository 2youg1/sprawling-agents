// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The disabled ink is written only behind a variant that names a
//! disabled state (tools/xtask/Spec.lean §8-8a).
//!
//! `--color-text-disabled` aims at APCA Lc 30, about 2:1 on the light
//! page: enough to tell a hand that a control will not answer, not enough
//! for an eye to read a figure. Information a person reads - a cost, a
//! time, a model name, a placeholder - takes `text-text-faint` or above.

use std::path::Path;

use super::THEME;
use crate::report::{Violation, XtaskError};
use crate::walk;

const DISABLED_INK: &str = "text-text-disabled";

const CLIENT_EXTS: [&str; 3] = ["svelte", "ts", "css"];

pub(super) fn judge_disabled_ink(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    for path in walk::files_with_ext(&root.join(walk::CLIENT_SRC), &CLIENT_EXTS)? {
        let rel = walk::rel(root, &path);
        if rel == THEME {
            continue;
        }
        violations.extend(bare_uses(&walk::read_text(&path)?).into_iter().map(|line| {
            Violation {
                gate: "color",
                location: format!("{rel}:{line}"),
                rule: RULE.to_owned(),
                violation: format!("`{DISABLED_INK}` outside a disabled variant"),
                alternative: "write `aria-disabled:text-text-disabled` on an element that \
                              carries `aria-disabled`, or use `text-text-faint` for \
                              information a person reads"
                    .to_owned(),
            }
        }));
    }
    Ok(violations)
}

const RULE: &str = "the disabled ink is written only behind a variant that names a disabled state";

/// The one-based lines that write the disabled ink without a disabled
/// variant in front of it.
pub(super) fn bare_uses(text: &str) -> Vec<usize> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| {
            line.match_indices(DISABLED_INK)
                .any(|(at, _)| !behind_disabled_variant(line.get(..at).unwrap_or("")))
        })
        .map(|(number, _)| number.saturating_add(1))
        .collect()
}

/// Whether the class token that ends where `before` ends carries a
/// variant naming a disabled state.
fn behind_disabled_variant(before: &str) -> bool {
    before
        .rsplit(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '`' | '{' | '}'))
        .next()
        .unwrap_or("")
        .rsplit_once(':')
        .is_some_and(|(variants, _)| {
            variants
                .split(':')
                .any(|variant| variant.contains("disabled"))
        })
}

#[cfg(test)]
mod tests {
    use super::bare_uses;

    #[test]
    fn disabled_ink_is_bare_unless_a_variant_names_a_disabled_state() {
        let text = "<span class=\"text-note text-text-disabled\">{cost}</span>\n\
                    <button class=\"aria-disabled:text-text-disabled\">go</button>\n\
                    <input class=\"placeholder:text-text-disabled\" />\n\
                    const MUTED = \"group-aria-disabled:hover:text-text-disabled\";\n\
                    {off ? 'text-text-disabled' : 'text-text'}\n";
        assert_eq!(bare_uses(text), vec![1, 3, 5]);
    }

    #[test]
    fn text_glued_to_the_class_is_not_a_variant() {
        let text = "<span class=\"xdisabledtext-text-disabled\">{cost}</span>\n";
        assert_eq!(bare_uses(text), vec![1]);
    }
}
