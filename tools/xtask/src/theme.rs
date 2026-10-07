// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where the browser client's theme lives, and in which order it is read
//! (tools/xtask/Spec.lean §8-8, §8-51).
//!
//! The theme is one entry stylesheet and the parts it imports. The entry
//! is the file `main.ts` imports; each part is a contiguous piece of the
//! cascade, imported by one `@import "./theme/<name>.css";` line, and the
//! order of those lines is the order the cascade reads them in. That
//! order has exactly one home, the entry, so this module reads it there
//! rather than sorting file names: a numbered prefix on every part would
//! have been a second place to state it.
//!
//! The gates that judge colour read the theme as one text, the entry with
//! each import line replaced by the part it names, because the light
//! block restates the tokens other parts declare and the readings of the
//! two lightings are built across that boundary. A violation still names
//! the part that declares what it is about (`Theme::declaring`).

use std::path::Path;

use crate::report::XtaskError;
use crate::walk;

/// The theme's entry, the stylesheet the client imports.
pub(crate) const ENTRY: &str = concat!(crate::walk::client_src!(), "/theme.css");

/// The directory every part sits in, beside the entry.
const PARTS: &str = concat!(crate::walk::client_src!(), "/theme/");

/// How the entry imports a part: the opening up to the part's name, and
/// what follows the name.
const IMPORT_OPEN: &str = "@import \"./theme/";
const IMPORT_CLOSE: &str = "\";";

/// One file of the theme, by its repo-relative path.
struct Part {
    rel: String,
    text: String,
}

/// The entry and its parts, the entry first and the parts in the order
/// the entry imports them.
pub(crate) struct Theme {
    entry: String,
    parts: Vec<Part>,
}

/// Whether a repo-relative path is a file of the theme: the entry, or a
/// stylesheet directly in the parts directory. A stylesheet there that
/// the entry does not import is refused by `Theme::read`, so the path
/// alone answers this question.
pub(crate) fn is_theme(rel: &str) -> bool {
    rel == ENTRY
        || rel
            .strip_prefix(PARTS)
            .is_some_and(|name| name.ends_with(".css") && !name.contains('/'))
}

impl Theme {
    /// Read the entry and every part it imports.
    ///
    /// # Errors
    ///
    /// `Io` when a file cannot be read; `ThemePartMissing` when the entry
    /// imports a part that is not on disk; `ThemePartUnimported` when the
    /// parts directory holds a stylesheet the entry never imports, which
    /// would otherwise be a file every gate exempts and no page draws.
    pub(crate) fn read(root: &Path) -> Result<Self, XtaskError> {
        let entry = walk::read_text(&root.join(ENTRY))?;
        let mut parts = Vec::new();
        for name in entry.lines().filter_map(imported) {
            let rel = format!("{PARTS}{name}");
            let path = root.join(&rel);
            if !path.is_file() {
                return Err(XtaskError::ThemePartMissing {
                    entry: ENTRY.to_owned(),
                    part: rel,
                });
            }
            parts.push(Part {
                text: walk::read_text(&path)?,
                rel,
            });
        }
        let dir = root.join(PARTS);
        if dir.is_dir() {
            for path in walk::files_with_ext(&dir, &["css"])? {
                let rel = walk::rel(root, &path);
                if !parts.iter().any(|part| part.rel == rel) {
                    return Err(XtaskError::ThemePartUnimported {
                        entry: ENTRY.to_owned(),
                        part: rel,
                    });
                }
            }
        }
        Ok(Self { entry, parts })
    }

    /// The theme as one stylesheet: the entry, with each import of a part
    /// replaced by that part's text.
    pub(crate) fn inlined(&self) -> String {
        let mut parts = self.parts.iter();
        self.entry
            .lines()
            .map(|line| match imported(line) {
                Some(_) => parts.next().map_or("", |part| part.text.trim_end()),
                None => line,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The file that holds `needle` first, in cascade order: the part
    /// that declares a token or opens a block, for a violation to name.
    /// The entry when no part holds it.
    pub(crate) fn declaring(&self, needle: &str) -> &str {
        self.parts
            .iter()
            .find(|part| part.text.contains(needle))
            .map_or(ENTRY, |part| part.rel.as_str())
    }
}

/// The part a line of the entry imports, by its file name.
fn imported(line: &str) -> Option<&str> {
    line.trim()
        .strip_prefix(IMPORT_OPEN)?
        .strip_suffix(IMPORT_CLOSE)
        .filter(|name| name.ends_with(".css") && !name.contains(['/', '"']))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    use super::*;

    fn tree(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("theme-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (rel, body) in files {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, body).unwrap();
        }
        root
    }

    /// The parts are read in the order the entry imports them, not the
    /// order their names sort in, and inlining puts each where its
    /// import stood.
    #[test]
    fn the_entry_decides_the_order_and_each_part_stands_where_it_is_imported() {
        let root = tree(
            "order",
            &[
                (
                    ENTRY,
                    "/* head */\n@import \"tailwindcss\";\n@import \"./theme/b.css\";\n\
                     @import \"./theme/a.css\";\n",
                ),
                ("client/src/theme/a.css", "a {}\n"),
                ("client/src/theme/b.css", "b {}\n"),
            ],
        );
        let theme = Theme::read(&root).unwrap();
        assert_eq!(
            theme.inlined(),
            "/* head */\n@import \"tailwindcss\";\nb {}\na {}"
        );
        assert_eq!(
            [theme.declaring("a {"), theme.declaring("head")],
            ["client/src/theme/a.css", ENTRY]
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn an_import_with_no_file_and_a_file_with_no_import_are_both_refused() {
        let missing = tree("missing", &[(ENTRY, "@import \"./theme/gone.css\";\n")]);
        let unimported = tree(
            "unimported",
            &[
                (ENTRY, "@import \"./theme/a.css\";\n"),
                ("client/src/theme/a.css", "a {}\n"),
                ("client/src/theme/dead.css", "b {}\n"),
            ],
        );
        let refused = [&missing, &unimported].map(|root| match Theme::read(root) {
            Err(XtaskError::ThemePartMissing { part, .. }) => format!("missing {part}"),
            Err(XtaskError::ThemePartUnimported { part, .. }) => format!("unimported {part}"),
            Err(other) => panic!("unexpected error: {other}"),
            Ok(_) => "read".to_owned(),
        });
        assert_eq!(
            refused,
            [
                "missing client/src/theme/gone.css",
                "unimported client/src/theme/dead.css"
            ]
        );
        for root in [missing, unimported] {
            std::fs::remove_dir_all(&root).unwrap();
        }
    }

    #[test]
    fn the_theme_is_the_entry_and_the_stylesheets_directly_beside_it() {
        assert_eq!(
            [
                ENTRY,
                "client/src/theme/tokens-colour.css",
                "client/src/theme/deeper/x.css",
                "client/src/theme/notes.md",
                "client/src/panel.css",
            ]
            .map(is_theme),
            [true, true, false, false, false]
        );
    }
}
