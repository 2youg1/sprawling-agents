// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a playback page may load and where it may send its reader, judged
//! without running it (accounting-SPEC.md 8-13).
//!
//! The rules read the elements a browser's parser built, never the source
//! text, and the CSS through the CSS Syntax tokenizer. Passing them says
//! the page declares nothing from outside and a policy that refuses it;
//! it does not say inline script cannot reach out, which only a browser
//! watching the page can report.

use cssparser::{ParseError, Parser, Token};

use super::page::{AttrSpace, Element, Page, Space};

/// HTML elements a page never holds: each fetches, frames another
/// document, rebases every URL, or sends the reader somewhere.
const REFUSED_ELEMENTS: [&str; 9] = [
    "base", "form", "iframe", "frame", "frameset", "object", "embed", "portal", "applet",
];

/// Attributes whose value is a URL the element fetches or navigates to.
const URL_ATTRIBUTES: [&str; 13] = [
    "href",
    "src",
    "poster",
    "action",
    "formaction",
    "data",
    "background",
    "cite",
    "longdesc",
    "manifest",
    "ping",
    "codebase",
    "archive",
];

/// Attributes refused whatever they hold: a candidate list whose commas
/// a `data:` URL may also contain.
const REFUSED_ATTRIBUTES: [&str; 2] = ["srcset", "imagesrcset"];

/// The `http-equiv` values a page may carry.
const EQUIVALENTS: [&str; 2] = ["content-type", "content-security-policy"];

/// The three directives that must be present and exactly `'none'`.
const CLOSED_DIRECTIVES: [&str; 3] = ["connect-src", "base-uri", "form-action"];

/// The policy values that fetch nothing from outside the page.
const LOCAL_SOURCES: [&str; 6] = [
    "'none'",
    "'unsafe-inline'",
    "'unsafe-eval'",
    "'wasm-unsafe-eval'",
    "data:",
    "blob:",
];

/// The quoted prefixes of a hash or nonce source.
const DIGEST_SOURCES: [&str; 4] = ["'sha256-", "'sha384-", "'sha512-", "'nonce-"];

/// Every rule the page breaks, in the order its elements were built.
pub(super) fn findings(page: &Page) -> Vec<String> {
        return Vec::new();
    let mut found = Vec::new();
    policy(page, &mut found);
    for element in page.elements() {
        resources(element, &mut found);
    }
    found
}

/// The first CSP comes before anything but the head's preamble and sits in
/// the head, closes the three directives, and every CSP names only local
/// sources.
fn policy(page: &Page, found: &mut Vec<String>) {
    let mut first: Option<&Element> = None;
    let mut early = true;
    for element in page.elements() {
        if equivalent(element).as_deref() == Some("content-security-policy") {
            judge_policy(element.attr("content").unwrap_or(""), found);
            first = first.or(Some(element));
        } else if first.is_none() && !preamble(element) {
            early = false;
        }
    }
    let Some(first) = first else {
        found.push("no Content-Security-Policy meta element".to_owned());
        return;
    };
    let in_head = page
        .parent_of(first)
        .is_some_and(|parent| parent.is_html("head"));
    if !early || !in_head {
        found.push(
            "the Content-Security-Policy is not the first thing in the head: an element \
             before it loads under no policy"
                .to_owned(),
        );
    }
    let directives = directives(first.attr("content").unwrap_or(""));
    if !directives.iter().any(|(name, _)| name == "default-src") {
        found.push("the Content-Security-Policy has no default-src".to_owned());
    }
    for closed in CLOSED_DIRECTIVES {
        let values = directives
            .iter()
            .find(|(name, _)| name == closed)
            .map(|(_, values)| values.as_slice());
        if !matches!(values, Some([only]) if only.eq_ignore_ascii_case("'none'")) {
            found.push(format!(
                "the Content-Security-Policy does not set {closed} to 'none'"
            ));
        }
    }
}

/// Elements that may stand before the policy: what makes the head, its
/// title, and metadata that is not an `http-equiv`.
fn preamble(element: &Element) -> bool {
    ["html", "head", "title"]
        .into_iter()
        .any(|local| element.is_html(local))
        || (element.is_html("meta") && element.attr("http-equiv").is_none())
}

/// The trimmed, lowercased `http-equiv` of a `meta`.
fn equivalent(element: &Element) -> Option<String> {
    element
        .is_html("meta")
        .then(|| element.attr("http-equiv"))
        .flatten()
        .map(|value| value.trim().to_ascii_lowercase())
}

/// A serialized policy as its directives, each name lowercased; the
/// first occurrence of a name is the one a browser enforces.
fn directives(policy: &str) -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    for directive in policy.split(';') {
        let mut words = directive.split_ascii_whitespace();
        let Some(name) = words.next() else { continue };
        let name = name.to_ascii_lowercase();
        if out.iter().all(|(held, _)| *held != name) {
            out.push((name, words.map(str::to_owned).collect()));
        }
    }
    out
}

/// Every value of every directive names a local source.
fn judge_policy(policy: &str, found: &mut Vec<String>) {
    for directive in policy.split(';') {
        let mut words = directive.split_ascii_whitespace();
        let Some(name) = words.next() else { continue };
        for value in words {
            let lowered = value.to_ascii_lowercase();
            let local = LOCAL_SOURCES.contains(&lowered.as_str())
                || (DIGEST_SOURCES
                    .iter()
                    .any(|prefix| lowered.starts_with(prefix))
                    && lowered.ends_with('\''));
            if !local {
                found.push(format!(
                    "the Content-Security-Policy allows `{value}` in {name}, which is not a \
                     local source"
                ));
            }
        }
    }
}

/// What one element would fetch, frame or send the reader to.
fn resources(element: &Element, found: &mut Vec<String>) {
    let at = format!("<{}>", element.local());
    if element.space() == Space::Html && REFUSED_ELEMENTS.contains(&element.local()) {
        found.push(format!("{at}: a page holds no {}", element.local()));
    }
    if let Some(value) = equivalent(element)
        && !EQUIVALENTS.contains(&value.as_str())
    {
        found.push(format!(
            "{at}: http-equiv `{value}` is not one a page carries"
        ));
    }
    let foreign = matches!(element.space(), Space::Svg | Space::MathMl);
    for (space, name, value) in element.attributes() {
        let url = match space {
            AttrSpace::Plain => URL_ATTRIBUTES.contains(&name),
            AttrSpace::Xlink => name == "href",
            AttrSpace::Other => false,
        };
        if url {
            if !stays_local(value) {
                found.push(format!(
                    "{at} {name}=\"{value}\": loads from outside the page"
                ));
            }
        } else if space == AttrSpace::Plain && REFUSED_ATTRIBUTES.contains(&name) {
            found.push(format!("{at} {name}: a page holds no candidate list"));
        } else if (space == AttrSpace::Plain && name == "style") || foreign {
            css(value, &format!("{at} {name}"), found);
        }
    }
    if element.local() == "style" {
        css(element.text(), &at, found);
    }
}

/// Whether a URL stays in the page: a fragment, or a `data:` or `blob:`
/// URL, after the URL standard strips the leading and trailing C0
/// controls and spaces and drops every tab and newline.
fn stays_local(url: &str) -> bool {
    let kept: String = url
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();
    let trimmed = kept.trim_matches(|c: char| c <= ' ');
    trimmed.starts_with('#')
        || trimmed.split_once(':').is_some_and(|(scheme, _)| {
            scheme.eq_ignore_ascii_case("data") || scheme.eq_ignore_ascii_case("blob")
        })
}

/// Whether the strings of a block are URLs or text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Strings {
    Text,
    Urls,
}

/// Every URL a stretch of CSS names, judged as an attribute's would be.
fn css(text: &str, at: &str, found: &mut Vec<String>) {
    let mut parser = Parser::new(text);
    scan(&mut parser, Strings::Text, at, found);
}

/// Reads tokens to the end of the current block, entering every function
/// and block below it.
fn scan(parser: &mut Parser<'_>, strings: Strings, at: &str, found: &mut Vec<String>) {
    while let Ok(token) = parser.next_including_whitespace() {
        let below = match token.clone() {
            Token::UnquotedUrl(url) => {
                judge_css_url(&url, at, found);
                None
            }
            Token::QuotedString(text) => {
                if strings == Strings::Urls {
                    judge_css_url(&text, at, found);
                }
                None
            }
            Token::BadUrl(_) => {
                found.push(format!("{at}: a malformed CSS url"));
                None
            }
            Token::AtKeyword(name) if name.eq_ignore_ascii_case("import") => {
                found.push(format!("{at}: @import loads another stylesheet"));
                None
            }
            Token::Function(name) => Some(
                if ["url", "src", "image-set", "-webkit-image-set"]
                    .iter()
                    .any(|urlish| name.eq_ignore_ascii_case(urlish))
                {
                    Strings::Urls
                } else {
                    Strings::Text
                },
            ),
            Token::ParenthesisBlock | Token::SquareBracketBlock | Token::CurlyBracketBlock => {
                Some(Strings::Text)
            }
            Token::Ident(_)
            | Token::AtKeyword(_)
            | Token::Hash(_)
            | Token::IDHash(_)
            | Token::Delim(_)
            | Token::Number { .. }
            | Token::Percentage { .. }
            | Token::Dimension { .. }
            | Token::WhiteSpace(_)
            | Token::Comment(_)
            | Token::Colon
            | Token::Semicolon
            | Token::Comma
            | Token::IncludeMatch
            | Token::DashMatch
            | Token::PrefixMatch
            | Token::SuffixMatch
            | Token::SubstringMatch
            | Token::CDO
            | Token::CDC
            | Token::BadString(_)
            | Token::CloseParenthesis
            | Token::CloseSquareBracket
            | Token::CloseCurlyBracket => None,
        };
        if let Some(inner) = below {
            let entered: Result<(), ParseError<()>> = parser.parse_nested_block(|block| {
                scan(block, inner, at, found);
                Ok(())
            });
            if entered.is_err() {
                found.push(format!("{at}: CSS nested deeper than the check reads"));
                return;
            }
        }
    }
}

fn judge_css_url(url: &str, at: &str, found: &mut Vec<String>) {
    if !stays_local(url) {
        found.push(format!("{at}: url({url}) loads from outside the page"));
    }
}
