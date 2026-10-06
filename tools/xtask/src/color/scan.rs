// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The repository scan: the half only a gate can do, because it is a
//! statement about every file rather than about one table.

use std::path::Path;

use super::THEME;
use crate::report::{Violation, XtaskError};
use crate::walk;

/// Each client names colour in one stylesheet. Playback owns its own
/// palette because the exported page cannot load the browser client's CSS.
const PRODUCTION_POINTS: [&str; 2] = [THEME, "crates/city/skills/playback/src/style.css"];

/// The offline output of the playback stylesheet. Its generation banner
/// identifies output; the shared Bun recipe verifies the assembled bytes.
const PLAYBACK_PAGE: &str = "crates/city/skills/playback/template.html";

/// The files that spell colour because reading a colour means naming
/// it, and the one test that writes the spelling it asserts. Same shape
/// and same reason as `tools/xtask/lexicon.toml` being outside the lexicon
/// scan - a checker has to be able to spell what it forbids.
/// `tables.rs` joined them when the token tables moved into the
/// stylesheet: reading an `oklch()` value means naming the function
/// that holds it. The probe and the survey's `Paint` test are that same
/// case one step out: the probe hands the engine a value to resolve and
/// reads the pixels back, so it must spell the transparent fill that
/// clears the canvas between readings, and the test states the one
/// spelling `Paint::to_string` promises a person. `glass.rs` writes the
/// rungs its tests lay glass over, for the reason `tests.rs` does.
const SPELLS_COLOUR: [&str; 8] = [
    "tools/xtask/src/color.rs",
    "tools/xtask/src/color/glass.rs",
    "tools/xtask/src/color/roles.rs",
    "tools/xtask/src/color/scan.rs",
    "tools/xtask/src/color/tables.rs",
    "tools/xtask/src/color/tests.rs",
    "crates/browser/src/survey/probe.rs",
    "crates/browser/src/survey/tests.rs",
];

/// Extensions worth scanning. Rust, and the two file kinds that carry style.
const SCAN_EXTS: [&str; 3] = ["rs", "css", "html"];

/// Colour spellings that must not appear outside the theme.
///
/// Hex is judged differently per language, because `#` means different
/// things in each. In style files a bare `#1a2b3c` is a colour. In Rust it
/// is far more often a Locator fragment or an issue number, so only a
/// fully-quoted `"#1a2b3c"` counts - the shape somebody actually writes
/// when they mean a colour. The first run of this gate caught
/// `cas:b3-…#B01-2` and taught this distinction.
pub(super) fn literal_at(line: &str, style_file: bool) -> Option<&'static str> {
    for syntax in ["oklch(", "rgb(", "rgba(", "hsl(", "hsla("] {
        if line.contains(syntax) {
            return Some(syntax);
        }
    }
    if hex_colour(line, style_file) {
        return Some("#rrggbb");
    }
    None
}

fn hex_colour(line: &str, style_file: bool) -> bool {
    let bytes = line.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        if *byte != b'#' {
            continue;
        }
        let run = bytes
            .iter()
            .skip(index.saturating_add(1))
            .take_while(|b| b.is_ascii_hexdigit())
            .count();
        if run != 3 && run != 6 {
            continue;
        }
        let opens = index.checked_sub(1).and_then(|i| bytes.get(i));
        let closes = index
            .checked_add(run)
            .and_then(|i| i.checked_add(1))
            .and_then(|i| bytes.get(i));
        let quoted = opens == Some(&b'"') && closes == Some(&b'"');
        if quoted || (style_file && closes.is_none_or(|b| !b.is_ascii_hexdigit())) {
            return true;
        }
    }
    false
}

pub(super) fn scan_for_literals(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    for path in walk::files_with_ext(root, &SCAN_EXTS)? {
        let rel = walk::rel(root, &path);
        if walk::in_isolation_zone(&rel)
            || PRODUCTION_POINTS.contains(&rel.as_str())
            || SPELLS_COLOUR.contains(&rel.as_str())
        {
            continue;
        }
        let style_file = !rel.ends_with(".rs");
        let text = walk::read_text(&path)?;
        if rel == PLAYBACK_PAGE && crate::length::generated(&text) {
            continue;
        }
        for (number, line) in text.lines().enumerate() {
            if let Some(syntax) = literal_at(line, style_file) {
                let line_number = number.saturating_add(1);
                violations.push(Violation {
                    gate: "color",
                    location: format!("{rel}:{line_number}"),
                    rule: "colour is named once per client, in that client's theme file".to_owned(),
                    violation: format!("colour literal `{syntax}` outside a theme file"),
                    alternative: "use a token from the client's theme through its CSS \
                                  custom property"
                        .to_owned(),
                });
            }
        }
    }
    Ok(violations)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::*;

    #[test]
    fn colour_spellings_are_recognised_and_locators_are_not() {
        assert_eq!(literal_at("  color: #070A12;", true), Some("#rrggbb"));
        assert_eq!(literal_at("background: rgb(1,2,3)", true), Some("rgb("));
        assert_eq!(
            literal_at("--G0:oklch(0.145 0.018 264)", true),
            Some("oklch(")
        );
        assert_eq!(literal_at("let x = 3;", false), None);

        // The shape somebody writes in Rust when they mean a colour.
        assert_eq!(
            literal_at(r##"let bg = "#070A12";"##, false),
            Some("#rrggbb")
        );

        // A Locator fragment is not a colour. This exact line is what the
        // gate's first run tripped on.
        assert_eq!(
            literal_at(r##"format!("cas:b3-{H64}#B01-2"),"##, false),
            None
        );
        assert_eq!(literal_at("issue #4707 records the status", false), None);
        // A digest is longer than six hex digits either way.
        assert_eq!(
            literal_at(
                "// 692b5f963f99f018496b8df111314dfe1bed52ccfe1a40cb9a5975b3bc8664fe",
                true
            ),
            None
        );
    }

    /// The client names colour in exactly one file, and only there.
    ///
    /// The scan runs over a tree holding the production point and one ordinary
    /// stylesheet: the production point is silent and the other file is
    /// reported, which is the whole rule.
    #[test]
    fn the_client_names_colour_in_one_file() {
        let root = std::env::temp_dir().join(format!("color-theme-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let written = [
            (THEME, "  --color-g0: oklch(0.145 0.018 264);"),
            ("client/src/panel.css", "  color: oklch(0.145 0.018 264);"),
            (
                "crates/city/skills/playback/src/style.css",
                "--page: oklch(0.145 0.014 262);",
            ),
            (
                "crates/city/skills/playback/template.html",
                "<!-- Generated by `bun assemble.js` -->
<style>:root { --page: oklch(0.145 0.014 262); }</style>",
            ),
        ];
        for (rel, body) in written {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, body).unwrap();
        }

        let found = scan_for_literals(&root).unwrap();
        let places: Vec<&str> = found.iter().map(|v| v.location.as_str()).collect();
        assert_eq!(
            places,
            ["client/src/panel.css:1"],
            "only the file that is not the production point is a violation; got {}",
            found
                .iter()
                .map(|v| format!("{} :: {}", v.rule, v.violation))
                .collect::<Vec<_>>()
                .join(" | ")
        );
        std::fs::write(root.join(PLAYBACK_PAGE), "<style>color: rgb(1,2,3)</style>").unwrap();
        let found = scan_for_literals(&root).unwrap();
        assert_eq!(
            found
                .iter()
                .map(|v| v.location.as_str())
                .collect::<Vec<_>>(),
            [
                "client/src/panel.css:1",
                "crates/city/skills/playback/template.html:1"
            ]
        );
        std::fs::remove_dir_all(&root).unwrap();
    }
}
