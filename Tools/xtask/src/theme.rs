// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use crate::report::XtaskError;
use crate::walk;

pub(crate) const ENTRY: &str = concat!(crate::walk::client_src!(), "/theme.css");

const PARTS: &str = concat!(crate::walk::client_src!(), "/theme/");

const IMPORT_OPEN: &str = "@import \"./theme/";
const IMPORT_CLOSE: &str = "\";";

struct Part {
    rel: String,
    text: String,
}

pub(crate) struct Theme {
    entry: String,
    parts: Vec<Part>,
}

pub(crate) fn is_theme(rel: &str) -> bool {
    rel == ENTRY
        || rel
            .strip_prefix(PARTS)
            .is_some_and(|name| name.ends_with(".css") && !name.contains('/'))
}

impl Theme {

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

    pub(crate) fn declaring(&self, needle: &str) -> &str {
        self.parts
            .iter()
            .find(|part| part.text.contains(needle))
            .map_or(ENTRY, |part| part.rel.as_str())
    }
}

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
