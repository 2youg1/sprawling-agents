// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which grammar a document is written in, read from its name
//! (`crates/documents/Spec.lean` D6).

use serde::{Deserialize, Serialize};

/// The grammar a document's blocks are read by.
///
/// Decided by the file name, never by the bytes: a window into a
/// Markdown file looks exactly like a plain file, and a Markdown file
/// with no heading is still Markdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Format {
    /// Blank lines separate blocks, and a code block written between
    /// two delimiter lines holds the blank lines inside it.
    Markdown,
    /// Every line is a block, and nothing in a line is structure.
    Plain,
}

/// The extensions read as Markdown, compared without regard to case.
const MARKDOWN_EXTENSIONS: [&str; 2] = ["md", "markdown"];

impl Format {
    /// The format of a file called `name`; a path's last segment decides.
    pub fn of_name(name: &str) -> Format {
        let last = name.rsplit(['/', '\\']).next().unwrap_or(name);
        let markdown = last.rsplit_once('.').is_some_and(|(stem, extension)| {
            !stem.is_empty()
                && MARKDOWN_EXTENSIONS
                    .iter()
                    .any(|known| known.eq_ignore_ascii_case(extension))
        });
        if markdown {
            Format::Markdown
        } else {
            Format::Plain
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    #[test]
    fn the_extension_of_the_last_segment_decides() {
        assert_eq!(Format::of_name("lab/Roadmap.md"), Format::Markdown);
        assert_eq!(Format::of_name("lab/NOTES.Markdown"), Format::Markdown);
        assert_eq!(Format::of_name("lab/notes.txt"), Format::Plain);
        assert_eq!(Format::of_name("lab.md/notes"), Format::Plain);
        assert_eq!(Format::of_name(".md"), Format::Plain);
    }
}
