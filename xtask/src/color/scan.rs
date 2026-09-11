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

/// The colour production point: the client names colour once, and this is
/// where. A table with one row rather than a bare constant, because the
/// question it answers - which files the scan walks past - is not the
/// question `THEME` answers, and a second client would add a row here
/// without touching which file the token assertions read.
const PRODUCTION_POINTS: [&str; 1] = [THEME];

/// The files of this gate that spell colour. Same shape and same reason as
/// `xtask/lexicon.toml` being outside the lexicon scan - a checker has to be
/// able to spell what it forbids. `tables.rs` joined them when the token
/// tables moved into the stylesheet: reading an `oklch()` value means naming
/// the function that holds it.
const SPELLS_COLOUR: [&str; 4] = [
    "xtask/src/color.rs",
    "xtask/src/color/scan.rs",
    "xtask/src/color/tables.rs",
    "xtask/src/color/tests.rs",
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
