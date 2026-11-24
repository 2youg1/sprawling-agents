// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use crate::report::{Violation, XtaskError};
use crate::sheet::{self, Reading};
use crate::theme;
use crate::walk;

const PLAYBACK_THEME: &str = "Sprawling/city/skills/playback/src/style.css";

const PLAYBACK_PAGE: &str = "Sprawling/city/skills/playback/template.html";

const SPELLS_COLOUR: [&str; 8] = [
    "Tools/xtask/src/color.rs",
    "Tools/xtask/src/color/glass.rs",
    "Tools/xtask/src/color/roles.rs",
    "Tools/xtask/src/color/scan.rs",
    "Tools/xtask/src/color/tables.rs",
    "Tools/xtask/src/color/tests.rs",
    "Sprawling/browser/src/survey/probe.rs",
    "Sprawling/browser/src/survey/tests.rs",
];

const SCAN_EXTS: [&str; 4] = ["rs", "css", "html", "tsx"];

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
            || rel.starts_with(crate::source::EARLIER_RELEASES)
            || SPELLS_COLOUR.contains(&rel.as_str())
        {
            continue;
        }
        let text = walk::read_text(&path)?;
        if rel == PLAYBACK_PAGE && walk::generated(&text) {
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

        assert_eq!(
            literal_at(r##"let bg = "#070A12";"##, Reading::Code),
            Some("#rrggbb")
        );

        assert_eq!(
            literal_at(r##"format!("cas:b3-{H64}#B01-2"),"##, Reading::Code),
            None
        );
        assert_eq!(
            literal_at("issue #4707 records the status", Reading::Code),
            None
        );
        assert_eq!(
            literal_at(
                "// 692b5f963f99f018496b8df111314dfe1bed52ccfe1a40cb9a5975b3bc8664fe",
                Reading::Sheet
            ),
            None
        );
    }

    #[test]
    fn the_client_names_colour_in_one_file() {
        let root = std::env::temp_dir().join(format!("color-theme-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let written = [
            (theme::ENTRY, "  --color-g0: oklch(0.145 0.018 264);"),
            ("RefRain/src/panel.css", "  color: oklch(0.145 0.018 264);"),
            (
                "Sprawling/city/skills/playback/src/style.css",
                "--page: oklch(0.145 0.014 262);",
            ),
            (
                "Sprawling/city/skills/playback/template.html",
                "<!-- Generated by `bun assemble.js` -->
<style>:root { --page: oklch(0.145 0.014 262); }</style>",
            ),
            (
                "Sprawling/app/tests/cities/v0.0.12/.sprawling/library/shipped/playback/src/style.css",
                "--page: oklch(0.145 0.014 262);",
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
            ["RefRain/src/panel.css:1"],
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
                "RefRain/src/panel.css:1",
                "Sprawling/city/skills/playback/template.html:1"
            ]
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_part_the_theme_imports_names_colour_as_the_theme() {
        let root = std::env::temp_dir().join(format!("color-theme-part-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let written = [
            (
                "RefRain/src/theme.css",
                "@import \"tailwindcss\";
@import \"./theme/tokens-colour.css\";
",
            ),
            (
                "RefRain/src/theme/tokens-colour.css",
                "@theme {
  --color-g0: oklch(0.145 0.014 250);
}
",
            ),
            ("RefRain/src/panel.css", "  color: oklch(0.145 0.018 264);"),
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
            ["RefRain/src/panel.css:1"]
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_native_stylesheet_and_its_view_are_read_with_their_own_syntax() {
        let root = std::env::temp_dir().join(format!("color-native-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let path = root.join("RefRain/src/views/panel.css");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, STYLE_FIXTURE).unwrap();
        std::fs::write(
            path.with_extension("tsx"),
            r##"<a href="#/gallery" class="[stop-color:var(--color-accent)]" />"##,
        )
        .unwrap();

        let found = scan_for_literals(&root).unwrap();
        assert_eq!(
            found
                .iter()
                .map(|v| v.location.as_str())
                .collect::<Vec<_>>(),
            ["RefRain/src/views/panel.css:1"]
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    pub(in crate::color) const STYLE_FIXTURE: &str = ".x { color: #1a2b3c; }
.y { background: var(--color-g3); }
.z { border-color: var(--color-raisd); }
.w { --color-mine: var(--color-accent); }
";
}
