// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One Markdown version written as an HTML file that stands on its own
//! (`crates/documents/Spec.lean` D33).
//!
//! The version is read by the one grammar into the tree a preview
//! carries, so links were already judged (D23) and nesting already
//! capped (D24); this module only writes that tree as elements. The file
//! runs no script and reaches nothing outside itself: a content security
//! policy opens it, the style is inline, and an image is a link to where
//! it lives rather than a request made by whoever opens the file.

use kernel::{AxCode, AxError};

use super::lowering;
use super::tree::{Align, Block, Check, Inline, ListItem, Order, Row, Spacing};
use crate::encoding::Encoding;
use crate::span::offset;

/// The longest version one export reads: an export's HTML, about twice
/// its source, still travels as one answer.
pub const EXPORT_BYTES_MAX: u64 = 4 * 1024 * 1024;

/// What the file may load: its own inline style, and nothing else.
const POLICY: &str = "default-src 'none'; style-src 'unsafe-inline'";

/// A reading measure and the plainest marks, so the file reads without
/// the page's stylesheet; the system's own colours, so it reads in either
/// lighting and names no colour of its own.
const STYLE: &str = "body{max-width:42rem;margin:2rem auto;padding:0 1rem;font:16px/1.6 system-ui,sans-serif;color-scheme:light dark}pre,code{font-family:ui-monospace,monospace}pre{overflow-x:auto;padding:.5rem;border:1px solid GrayText}table{border-collapse:collapse}td,th{border:1px solid GrayText;padding:.25rem .5rem}blockquote{margin-left:0;padding-left:1rem;border-left:3px solid GrayText}.footnote{font-size:.9em}";

/// The six heading elements, by level.
const HEADINGS: [&str; 6] = ["h1", "h2", "h3", "h4", "h5", "h6"];

/// `bytes`, a Markdown version in `encoding`, as one HTML file titled
/// `title`.
///
/// # Errors
/// `E_INVALID_ARGS` for a UTF-16 version, whose tree cannot be laid
/// against its bytes (D25), for a version longer than
/// [`EXPORT_BYTES_MAX`], and for bytes that are not text in `encoding`.
pub fn export(title: &str, encoding: Encoding, bytes: &[u8]) -> Result<String, AxError> {
    match encoding {
        Encoding::Utf16Le | Encoding::Utf16Be => return Err(refused(title, "is UTF-16")),
        Encoding::Utf8 | Encoding::Utf8Bom => {}
    }
    if offset(bytes.len()) > EXPORT_BYTES_MAX {
        return Err(refused(title, "is longer than one export reads"));
    }
    let text = encoding.decode(bytes)?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let mut page = Page(String::with_capacity(text.len().saturating_mul(2)));
    page.open(title);
    page.blocks(&lowering::blocks(text, 0));
    page.0.push_str("</body>\n</html>\n");
    Ok(page.0)
}

fn refused(title: &str, why: &str) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "export a document",
        format!("{title} {why}"),
    )
    .with_recovery("export a UTF-8 Markdown version of at most 4 MiB")
}

/// The file as it is written, element by element.
struct Page(String);

impl Page {
    fn open(&mut self, title: &str) {
        self.0
            .push_str("<!doctype html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n");
        self.0
            .push_str("<meta http-equiv=\"Content-Security-Policy\" content=\"");
        self.0.push_str(POLICY);
        self.0
            .push_str("\">\n<meta name=\"viewport\" content=\"width=device-width\">\n<title>");
        self.text(title);
        self.0.push_str("</title>\n<style>");
        self.0.push_str(STYLE);
        self.0.push_str("</style>\n</head>\n<body>\n");
    }

    fn blocks(&mut self, blocks: &[Block]) {
        for block in blocks {
            self.block(block);
        }
    }

    fn block(&mut self, block: &Block) {
        match block {
            Block::Heading { level, inline, .. } => {
                let index = usize::from(level.saturating_sub(1));
                let tag = HEADINGS.get(index).copied().unwrap_or("h6");
                self.element(tag, |page| page.inlines(inline));
                self.0.push('\n');
            }
            Block::Paragraph { inline, .. } => {
                self.element("p", |page| page.inlines(inline));
                self.0.push('\n');
            }
            Block::List {
                order,
                spacing,
                items,
                ..
            } => self.list(*order, *spacing, items),
            Block::Quote { blocks, .. } => {
                self.0.push_str("<blockquote>\n");
                self.blocks(blocks);
                self.0.push_str("</blockquote>\n");
            }
            Block::Code { text, .. } => {
                self.0.push_str("<pre><code>");
                self.text(text);
                self.0.push_str("</code></pre>\n");
            }
            Block::Table {
                align, head, body, ..
            } => self.table(align, head, body),
            Block::Rule { .. } => self.0.push_str("<hr>\n"),
            Block::Footnote { name, blocks, .. } => {
                self.0.push_str("<div class=\"footnote\" id=\"fn-");
                self.text(name);
                self.0.push_str("\"><sup>");
                self.text(name);
                self.0.push_str("</sup>\n");
                self.blocks(blocks);
                self.0.push_str("</div>\n");
            }
            Block::Unsupported { source, .. } => {
                self.0.push_str("<pre class=\"source\">");
                self.text(source);
                self.0.push_str("</pre>\n");
            }
        }
    }

    fn list(&mut self, order: Order, spacing: Spacing, items: &[ListItem]) {
        let close = match order {
            Order::Bullet => {
                self.0.push_str("<ul>\n");
                "</ul>\n"
            }
            Order::Ordered { start } => {
                self.0.push_str("<ol start=\"");
                self.0.push_str(&start.to_string());
                self.0.push_str("\">\n");
                "</ol>\n"
            }
        };
        for item in items {
            self.0.push_str("<li>");
            match item.check {
                Check::NotATask => {}
                Check::Open => self.0.push_str("<input type=\"checkbox\" disabled> "),
                Check::Done => self
                    .0
                    .push_str("<input type=\"checkbox\" disabled checked> "),
            }
            for block in &item.blocks {
                match (spacing, block) {
                    // A tight list's paragraphs are the item's own text.
                    (Spacing::Tight, Block::Paragraph { inline, .. }) => self.inlines(inline),
                    (Spacing::Tight | Spacing::Loose, other) => self.block(other),
                }
            }
            self.0.push_str("</li>\n");
        }
        self.0.push_str(close);
    }

    fn table(&mut self, align: &[Align], head: &Row, body: &[Row]) {
        self.0.push_str("<table>\n<thead>\n");
        self.row("th", align, head);
        self.0.push_str("</thead>\n<tbody>\n");
        for row in body {
            self.row("td", align, row);
        }
        self.0.push_str("</tbody>\n</table>\n");
    }

    fn row(&mut self, cell: &str, align: &[Align], row: &Row) {
        self.0.push_str("<tr>");
        for (column, content) in row.cells.iter().enumerate() {
            let side = match align.get(column).copied().unwrap_or(Align::None) {
                Align::None => "",
                Align::Left => " style=\"text-align:left\"",
                Align::Center => " style=\"text-align:center\"",
                Align::Right => " style=\"text-align:right\"",
            };
            self.0.push('<');
            self.0.push_str(cell);
            self.0.push_str(side);
            self.0.push('>');
            self.inlines(content);
            self.close(cell);
        }
        self.0.push_str("</tr>\n");
    }

    fn inlines(&mut self, inlines: &[Inline]) {
        for inline in inlines {
            self.inline(inline);
        }
    }

    fn inline(&mut self, inline: &Inline) {
        match inline {
            Inline::Text(text) => self.text(text),
            Inline::Code(code) => self.element("code", |page| page.text(code)),
            Inline::Emphasis(inner) => self.element("em", |page| page.inlines(inner)),
            Inline::Strong(inner) => self.element("strong", |page| page.inlines(inner)),
            Inline::Strikethrough(inner) => self.element("del", |page| page.inlines(inner)),
            Inline::Link {
                target,
                title,
                content,
            } => {
                self.link(target, title);
                self.inlines(content);
                self.0.push_str("</a>");
            }
            // Never fetched: the file would reach outside itself (D33).
            Inline::Image { target, title, alt } => {
                self.link(target, title);
                self.text(alt);
                self.0.push_str("</a>");
            }
            Inline::FootnoteReference { name } => {
                self.0.push_str("<sup><a href=\"#fn-");
                self.text(name);
                self.0.push_str("\">");
                self.text(name);
                self.0.push_str("</a></sup>");
            }
            Inline::SoftBreak => self.0.push('\n'),
            Inline::LineBreak => self.0.push_str("<br>\n"),
            Inline::Unsupported { source, .. } => {
                self.0.push_str("<code class=\"source\">");
                self.text(source);
                self.0.push_str("</code>");
            }
        }
    }

    fn link(&mut self, target: &str, title: &str) {
        self.0.push_str("<a href=\"");
        self.text(target);
        self.0.push('"');
        if !title.is_empty() {
            self.0.push_str(" title=\"");
            self.text(title);
            self.0.push('"');
        }
        self.0.push('>');
    }

    fn element(&mut self, tag: &str, inner: impl FnOnce(&mut Page)) {
        self.0.push('<');
        self.0.push_str(tag);
        self.0.push('>');
        inner(self);
        self.close(tag);
    }

    fn close(&mut self, tag: &str) {
        self.0.push_str("</");
        self.0.push_str(tag);
        self.0.push('>');
    }

    /// Text and attribute values alike: the five characters HTML reads
    /// as markup are written as references.
    fn text(&mut self, text: &str) {
        for character in text.chars() {
            match character {
                '&' => self.0.push_str("&amp;"),
                '<' => self.0.push_str("&lt;"),
                '>' => self.0.push_str("&gt;"),
                '"' => self.0.push_str("&quot;"),
                '\'' => self.0.push_str("&#39;"),
                other => self.0.push(other),
            }
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

    const SOURCE: &str = "# Notes & 5 < 6\n\nA [link](https://example.org \"t\") and \
        [bad](javascript:alert(1)) and ![a cat](cat.png).\n\n\
        - [x] done\n- [ ] open\n\n\
        | a | b |\n|:-|-:|\n| 1 | 2 |\n\n\
        > quoted\n\n```rust\nfn main() {}\n```\n\n$x^2$ beside <b>raw</b>.\n\n\
        Said[^n].\n\n[^n]: A note.\n";

    /// Everything the grammar reads becomes an element, text is escaped,
    /// and nothing in the file runs or reaches outside it.
    #[test]
    fn an_export_is_one_page_with_no_script_and_no_reach_outside() {
        let html = export("Notes.md", Encoding::Utf8, SOURCE.as_bytes()).unwrap();
        assert!(html.starts_with("<!doctype html>"), "{html}");
        assert!(html.contains("<title>Notes.md</title>"), "{html}");
        assert!(
            html.contains("content=\"default-src 'none'; style-src 'unsafe-inline'\""),
            "{html}"
        );
        assert!(html.contains("<h1>Notes &amp; 5 &lt; 6</h1>"), "{html}");
        assert!(
            html.contains("<a href=\"https://example.org\" title=\"t\">link</a>"),
            "{html}"
        );
        assert!(
            !html.contains("javascript:"),
            "a refused scheme leaves its text: {html}"
        );
        assert!(html.contains("bad"), "{html}");
        assert!(html.contains("<a href=\"cat.png\">a cat</a>"), "{html}");
        assert!(!html.contains("<img"), "an image is never fetched: {html}");
        assert!(
            html.contains("<input type=\"checkbox\" disabled checked>"),
            "{html}"
        );
        assert!(
            html.contains("<input type=\"checkbox\" disabled>"),
            "{html}"
        );
        assert!(
            html.contains("<th style=\"text-align:left\">a</th>"),
            "{html}"
        );
        assert!(
            html.contains("<td style=\"text-align:right\">2</td>"),
            "{html}"
        );
        assert!(html.contains("<blockquote>"), "{html}");
        assert!(
            html.contains("<pre><code>fn main() {}\n</code></pre>"),
            "{html}"
        );
        assert!(
            html.contains("<code class=\"source\">$x^2$</code>"),
            "{html}"
        );
        assert!(
            html.contains("&lt;b&gt;"),
            "raw HTML is shown as its source: {html}"
        );
        assert!(html.contains("<a href=\"#fn-n\">n</a>"), "{html}");
        assert!(html.contains("id=\"fn-n\""), "{html}");
        assert!(!html.to_ascii_lowercase().contains("<script"), "{html}");
    }

    /// The mark is not text, and a UTF-16 version or one too long to
    /// answer in one export is refused rather than half written.
    #[test]
    fn a_marked_version_drops_its_mark_and_what_cannot_be_laid_is_refused() {
        let marked = [b"\xEF\xBB\xBF".as_slice(), b"# Title\n"].concat();
        let html = export("a.md", Encoding::Utf8Bom, &marked).unwrap();
        assert!(html.contains("<h1>Title</h1>"), "{html}");
        assert!(!html.contains('\u{feff}'), "{html}");
        let refused = export("a.md", Encoding::Utf16Le, b"\xFF\xFEa\0").unwrap_err();
        assert_eq!(refused.code(), &AxCode::InvalidArgs);
        let long = vec![b'a'; usize::try_from(EXPORT_BYTES_MAX).unwrap() + 1];
        let refused = export("a.md", Encoding::Utf8, &long).unwrap_err();
        assert_eq!(refused.code(), &AxCode::InvalidArgs);
    }
}
