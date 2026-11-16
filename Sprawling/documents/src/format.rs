// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Format {

    Markdown,

    Plain,
}

const MARKDOWN_EXTENSIONS: [&str; 2] = ["md", "markdown"];

impl Format {

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
