// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The disabled ink is written only behind a variant that names a
//! disabled state (xtask-SPEC.md section 8-8a).
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

/// Files another package of the same wave is rewriting, each pinned at
/// the number of bare uses it carried when the rule arrived. A pin may
/// only fall; a pin that reaches zero is itself reported so it is struck.
const PINNED: [(&str, usize); 11] = [
    ("client/src/views/city.svelte", 2),
    ("client/src/views/city/panel.svelte", 1),
    ("client/src/views/gallery/conversation.svelte", 4),
    ("client/src/views/machine/report.svelte", 2),
    ("client/src/views/parts/combobox.svelte", 2),
    ("client/src/views/run.svelte", 9),
    ("client/src/views/run/prompt.svelte", 3),
    ("client/src/views/talk.svelte", 1),
    ("client/src/views/talk/composer.svelte", 3),
    ("client/src/views/talk/divider.svelte", 1),
    ("client/src/views/talk/thread.svelte", 6),
];

pub(super) fn judge_disabled_ink(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    let mut seen: Vec<(&str, usize)> = Vec::new();
    for path in walk::files_with_ext(&root.join(walk::CLIENT_SRC), &CLIENT_EXTS)? {
        let rel = walk::rel(root, &path);
        if rel == THEME {
            continue;
        }
        let bare = bare_uses(&walk::read_text(&path)?);
        match PINNED.iter().find(|(file, _)| *file == rel) {
            Some(&(file, pin)) => {
                seen.push((file, bare.len()));
                if bare.len() > pin {
                    violations.push(pin_violation(
                        file,
                        format!("{} bare uses, pinned at {pin}", bare.len()),
                        "move the new use behind a disabled variant or onto `text-text-faint`",
                    ));
                }
            }
            None => violations.extend(bare.into_iter().map(|line| {
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
            })),
        }
    }
    violations.extend(
        PINNED
            .iter()
            .filter(|(file, _)| !seen.iter().any(|(s, count)| s == file && *count > 0))
            .map(|(file, _)| {
                pin_violation(
                    file,
                    "no bare use remains".to_owned(),
                    "strike this entry from `PINNED` in xtask/src/color/disabled.rs",
                )
            }),
    );
    Ok(violations)
}

const RULE: &str = "the disabled ink is written only behind a variant that names a disabled state";

fn pin_violation(file: &str, violation: String, alternative: &str) -> Violation {
    Violation {
        gate: "color",
        location: file.to_owned(),
        rule: RULE.to_owned(),
        violation,
        alternative: alternative.to_owned(),
    }
}

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
        .split(':')
        .any(|variant| variant.contains("disabled"))
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
}
