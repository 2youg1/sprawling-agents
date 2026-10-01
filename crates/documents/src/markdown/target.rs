// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which link and image targets a preview lets a page follow
//! (`crates/documents/spec/Markdown.lean` `admitted`, D23).
//!
//! The rule is here once so a page and an export never judge a target
//! twice. It reads a target the way a browser parses an `href` - tabs
//! and line breaks removed, leading control characters and spaces
//! dropped - because a judgement made on a different reading than the
//! one that is followed lets `java\tscript:` through.

/// The schemes a target may name besides being relative.
const ADMITTED_SCHEMES: [&str; 3] = ["http", "https", "mailto"];

/// Whether `target` may be followed: relative, or one of
/// [`ADMITTED_SCHEMES`].
pub(super) fn admitted(target: &str) -> bool {
    let parsed: String = target
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();
    let parsed = parsed.trim_start_matches(|c: char| c <= ' ');
    scheme_of(parsed).is_none_or(|scheme| {
        ADMITTED_SCHEMES
            .iter()
            .any(|admitted| admitted.eq_ignore_ascii_case(scheme))
    })
}

/// The RFC 3986 scheme a target opens with; a target without one is
/// relative.
fn scheme_of(target: &str) -> Option<&str> {
    let (scheme, _) = target.split_once(':')?;
    let mut chars = scheme.chars();
    let first = chars.next()?;
    (first.is_ascii_alphabetic()
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')))
    .then_some(scheme)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, reason = "test code")]
mod tests {
    use super::admitted;

    /// Targets are judged as a browser reads an `href`.
    #[test]
    fn a_target_is_judged_as_a_browser_reads_it() {
        let targets = [
            "https://x.org",
            "HTTP://x.org",
            "mailto:someone@x.org",
            "./doc.md",
            "#top",
            "a/b:c",
            "javascript:alert(1)",
            "JaVaScRiPt:alert(1)",
            " \u{1}javascript:alert(1)",
            "java\tscript:alert(1)",
            "data:image/png;base64,AA",
            "file:///etc/hosts",
            "C:\\Windows",
        ];
        let judged: Vec<(&str, bool)> = targets.iter().map(|t| (*t, admitted(t))).collect();
        assert_eq!(
            judged,
            vec![
                ("https://x.org", true),
                ("HTTP://x.org", true),
                ("mailto:someone@x.org", true),
                ("./doc.md", true),
                ("#top", true),
                ("a/b:c", true),
                ("javascript:alert(1)", false),
                ("JaVaScRiPt:alert(1)", false),
                (" \u{1}javascript:alert(1)", false),
                ("java\tscript:alert(1)", false),
                ("data:image/png;base64,AA", false),
                ("file:///etc/hosts", false),
                ("C:\\Windows", false),
            ]
        );
    }
}
