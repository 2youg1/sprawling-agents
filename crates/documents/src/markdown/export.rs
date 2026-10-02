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

use crate::encoding::Encoding;

/// The longest version one export reads: an export's HTML, about twice
/// its source, still travels as one answer.
pub const EXPORT_BYTES_MAX: u64 = 4 * 1024 * 1024;

/// `bytes`, a Markdown version in `encoding`, as one HTML file titled
/// `title`.
///
/// # Errors
/// `E_INVALID_ARGS` for a UTF-16 version, whose tree cannot be laid
/// against its bytes (D25), for a version longer than
/// [`EXPORT_BYTES_MAX`], and for bytes that are not text in `encoding`.
pub fn export(title: &str, encoding: Encoding, bytes: &[u8]) -> Result<String, AxError> {
    let _ = (title, encoding, bytes);
    Ok(String::new())
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

    const SOURCE: &str = "# Notes & <plans>\n\nA [link](https://example.org \"t\") and \
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
        assert!(
            html.contains("<h1>Notes &amp; &lt;plans&gt;</h1>"),
            "{html}"
        );
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
