// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which lines of a client file are a stylesheet (tools/xtask/Spec.lean
//! §8-8 D35).
//!
//! A `.css` or `.html` file is a stylesheet from its first line to its
//! last. A `.svelte` file is markup with one stylesheet inside it, its
//! top-level `<style>` block, and the two are read by different rules:
//! in markup `#` opens a block or a fragment far more often than it
//! names a colour (`{#each}` would read as `#eac`), while in the block a
//! bare `#1a2b3c` is a colour and `transition: opacity 200ms` is a
//! duration. `color` and `motion` both ask which reading a line takes,
//! so the answer lives here once.

/// How a gate reads one line of a client file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reading {
    /// A line of a stylesheet: a `.css` or `.html` file, or a line inside
    /// a `.svelte` file's top-level `<style>` block.
    Sheet,
    /// Script, markup, or a language that is not a stylesheet.
    Code,
}

/// One line of a file: its 1-based number, its text, and its reading.
pub(crate) struct Line<'t> {
    pub(crate) number: usize,
    pub(crate) text: &'t str,
    pub(crate) reading: Reading,
}

/// Every line of `text`, which is the file at the repo-relative `rel`.
///
/// The block of a `.svelte` file is what lies between a line that starts
/// with `<style` and the next line that starts with `</style>`; the two
/// tag lines are markup, and a block opened and closed on one line is
/// read whole as a sheet. Svelte compiles only a top-level block as the
/// component's stylesheet, and a top-level tag starts its line, so a
/// `<style>` written inside a string or a template literal, which is
/// indented, stays code.
pub(crate) fn lines<'t>(rel: &str, text: &'t str) -> impl Iterator<Item = Line<'t>> {
    let whole = [".css", ".html"].iter().any(|ext| rel.ends_with(ext));
    let component = rel.ends_with(".svelte");
    text.lines()
        .enumerate()
        .scan(false, move |inside, (index, text)| {
            let reading = if whole {
                Reading::Sheet
            } else if !component {
                Reading::Code
            } else if text.starts_with("</style") {
                *inside = false;
                Reading::Code
            } else if *inside {
                Reading::Sheet
            } else if !text.starts_with("<style") {
                Reading::Code
            } else if text.contains("</style") {
                Reading::Sheet
            } else {
                *inside = true;
                Reading::Code
            };
            Some(Line {
                number: index.saturating_add(1),
                text,
                reading,
            })
        })
}

/// The names a stylesheet line declares under a custom-property prefix:
/// `--color-mine: …` declares `mine` under `--color-`, and
/// `var(--color-mine)` reads it and declares nothing. A declaration
/// starts its line or follows whitespace, `{` or `;`.
pub(crate) fn declared<'l>(line: &'l str, prefix: &'l str) -> impl Iterator<Item = &'l str> {
    line.match_indices(prefix).filter_map(move |(at, _)| {
        let opens = line
            .get(..at)
            .and_then(|head| head.bytes().next_back())
            .is_none_or(|byte| byte.is_ascii_whitespace() || matches!(byte, b'{' | b';'));
        let tail = line.get(at.saturating_add(prefix.len())..)?;
        let length = tail
            .bytes()
            .take_while(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'*'))
            .count();
        let name = tail.get(..length)?;
        let colon = tail.get(length..)?.trim_start().starts_with(':');
        (opens && colon && !name.is_empty()).then_some(name)
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    use super::{Reading, declared, lines};

    #[test]
    fn a_component_reads_its_top_level_style_block_as_a_sheet_and_the_rest_as_code() {
        let text = "<script>
  const page = `<style>.x { color: red; }</style>`;
</script>
{#each rows as row}{row}{/each}
<style>
  .a { color: var(--color-text); }
</style>
<p></p>
<style>.b {}</style>
";
        let read: Vec<(usize, Reading)> = lines("client/src/views/a.svelte", text)
            .map(|line| (line.number, line.reading))
            .collect();
        assert_eq!(
            read,
            [
                (1, Reading::Code),
                (2, Reading::Code),
                (3, Reading::Code),
                (4, Reading::Code),
                (5, Reading::Code),
                (6, Reading::Sheet),
                (7, Reading::Code),
                (8, Reading::Code),
                (9, Reading::Sheet),
            ]
        );
        assert!(lines("client/src/x.css", "a\nb").all(|line| line.reading == Reading::Sheet));
        assert!(lines("client/src/x.ts", "<style>\na").all(|line| line.reading == Reading::Code));
    }

    #[test]
    fn a_declaration_follows_a_boundary_and_a_reading_is_not_one() {
        assert_eq!(
            declared(".w { --color-mine: var(--color-accent); }", "--color-").collect::<Vec<_>>(),
            ["mine"]
        );
        assert_eq!(
            declared("--animate-*: initial;", "--animate-").collect::<Vec<_>>(),
            ["*"]
        );
        assert_eq!(
            declared("a{--ease-x :1;--ease-y:2}", "--ease-").collect::<Vec<_>>(),
            ["x", "y"]
        );
        assert_eq!(
            declared("transition: x var(--ease-arrive);", "--ease-").count(),
            0
        );
    }
}
