// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The repository scan: the half only a gate can do, because it is a
//! statement about every file rather than about one table.

use std::path::Path;

use crate::report::{Violation, XtaskError};
use crate::sheet::{self, Reading};
use crate::theme;
use crate::walk;

/// Each client names colour in one theme. The browser client's is the
/// entry and the parts it imports (`theme::is_theme`); playback owns its
/// own palette because the exported page cannot load the browser
/// client's CSS.
const PLAYBACK_THEME: &str = "crates/city/skills/playback/src/style.css";

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

/// Extensions worth scanning: Rust, the two file kinds that carry style,
/// and the client's components, whose `<style>` block is a stylesheet
/// (`sheet::lines`).
const SCAN_EXTS: [&str; 4] = ["rs", "css", "html", "svelte"];

/// Colour spellings that must not appear outside the theme.
///
/// Hex is judged differently per language, because `#` means different
/// things in each. In style files a bare `#1a2b3c` is a colour. In Rust,
/// and in a component's script and markup, it is far more often a
/// Locator fragment, an issue number or a block (`{#each}`), so only a
/// fully-quoted `"#1a2b3c"` counts - the shape somebody actually writes
/// when they mean a colour. The first run of this gate caught
/// `cas:b3-…#B01-2` and taught this distinction.
pub(super) fn literal_at(line: &str, reading: Reading) -> Option<&'static str> {
    for syntax in ["oklch(", "rgb(", "rgba(", "hsl(", "hsla("] {
        if line.contains(syntax) {
            return Some(syntax);
        }
    }
    if hex_colour(line, reading) {
        return Some("#rrggbb");
    }
    None
}

fn hex_colour(line: &str, reading: Reading) -> bool {
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
        if quoted || (reading == Reading::Sheet && closes.is_none_or(|b| !b.is_ascii_hexdigit())) {
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
            || theme::is_theme(&rel)
            || rel == PLAYBACK_THEME
            || SPELLS_COLOUR.contains(&rel.as_str())
        {
            continue;
        }
        let text = walk::read_text(&path)?;
        if rel == PLAYBACK_PAGE && crate::length::generated(&text) {
            continue;
        }
        for line in sheet::lines(&rel, &text) {
            if let Some(syntax) = literal_at(line.text, line.reading) {
                violations.push(Violation {
                    gate: "color",
                    location: format!("{rel}:{}", line.number),
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
pub(super) mod tests {
    use super::*;

    #[test]
    fn colour_spellings_are_recognised_and_locators_are_not() {
        assert_eq!(
            literal_at("  color: #070A12;", Reading::Sheet),
            Some("#rrggbb")
        );
        assert_eq!(
            literal_at("background: rgb(1,2,3)", Reading::Sheet),
            Some("rgb(")
        );
        assert_eq!(
            literal_at("--G0:oklch(0.145 0.018 264)", Reading::Sheet),
            Some("oklch(")
        );
        assert_eq!(literal_at("let x = 3;", Reading::Code), None);

        // The shape somebody writes in Rust when they mean a colour.
        assert_eq!(
            literal_at(r##"let bg = "#070A12";"##, Reading::Code),
            Some("#rrggbb")
        );

        // A Locator fragment is not a colour. This exact line is what the
        // gate's first run tripped on.
        assert_eq!(
            literal_at(r##"format!("cas:b3-{H64}#B01-2"),"##, Reading::Code),
            None
        );
        assert_eq!(
            literal_at("issue #4707 records the status", Reading::Code),
            None
        );
        // A digest is longer than six hex digits either way.
        assert_eq!(
            literal_at(
                "// 692b5f963f99f018496b8df111314dfe1bed52ccfe1a40cb9a5975b3bc8664fe",
                Reading::Sheet
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
            (theme::ENTRY, "  --color-g0: oklch(0.145 0.018 264);"),
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

    /// The theme is its entry and every part the entry imports: a token
    /// declared in a part is the theme naming a colour, while a
    /// stylesheet beside the parts that no entry imports is not.
    #[test]
    fn a_part_the_theme_imports_names_colour_as_the_theme() {
        let root = std::env::temp_dir().join(format!("color-theme-part-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let written = [
            (
                "client/src/theme.css",
                "@import \"tailwindcss\";
@import \"./theme/tokens-colour.css\";
",
            ),
            (
                "client/src/theme/tokens-colour.css",
                "@theme {
  --color-g0: oklch(0.145 0.014 250);
}
",
            ),
            ("client/src/panel.css", "  color: oklch(0.145 0.018 264);"),
        ];
        for (rel, body) in written {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, body).unwrap();
        }

        let found = scan_for_literals(&root).unwrap();
        assert_eq!(
            found
                .iter()
                .map(|v| v.location.as_str())
                .collect::<Vec<_>>(),
            ["client/src/panel.css:1"]
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// A `.svelte` file is markup with one stylesheet inside it. The
    /// markup is judged as Rust is, where `#` is far more often a
    /// fragment or a block (`{#each}` would read as the colour `#eac`),
    /// and the top-level `<style>` block as a stylesheet is.
    #[test]
    fn a_component_style_is_judged_as_a_stylesheet_and_its_markup_is_not() {
        let root = std::env::temp_dir().join(format!("color-svelte-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let path = root.join("client/src/views/panel.svelte");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, SVELTE_FIXTURE).unwrap();

        let found = scan_for_literals(&root).unwrap();
        assert_eq!(
            found
                .iter()
                .map(|v| v.location.as_str())
                .collect::<Vec<_>>(),
            ["client/src/views/panel.svelte:7"]
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// The component the color gate's tests share: markup that only
    /// looks like colour, then a `<style>` block with one literal (line
    /// 7), one rung (8), one misspelt role (9) and one colour declared
    /// outside the theme (10).
    pub(in crate::color) const SVELTE_FIXTURE: &str = "<script lang=\"ts\">
  const rows = [1];
</script>
{#each rows as row}<a href=\"#/gallery\">{row}</a>{/each}
<div class=\"[stop-color:var(--color-accent)]\"></div>
<style>
  .x { color: #1a2b3c; }
  .y { background: var(--color-g3); }
  .z { border-color: var(--color-raisd); }
  .w { --color-mine: var(--color-accent); }
</style>
";
}
