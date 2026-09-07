// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The text: pieces and their classes.

/// One document, split into what is marked and what is not.
///
/// `kernel::markdown` guarantees its spans are ordered, disjoint and on
/// character boundaries, so this walks them once and never decides
/// anything: a piece is either inside a span or between two of them.
/// Carries the offset as a key, because two pieces of a document can say
/// the same words.
#[must_use]
pub fn pieces(text: &str) -> Vec<(usize, Option<channels::Token>, String)> {
    let mut split: Vec<(usize, Option<channels::Token>, String)> = Vec::new();
    let mut at = 0usize;
    for span in channels::markdown(text) {
        let (Ok(start), Ok(len)) = (usize::try_from(span.start), usize::try_from(span.len)) else {
            continue;
        };
        let end = start.saturating_add(len);
        if let Some(plain) = text.get(at..start)
            && !plain.is_empty()
        {
            split.push((at, None, plain.to_owned()));
        }
        if let Some(marked) = text.get(start..end) {
            split.push((start, Some(span.token), marked.to_owned()));
            at = end;
        }
    }
    if let Some(rest) = text.get(at..)
        && !rest.is_empty()
    {
        split.push((at, None, rest.to_owned()));
    }
    split
}

/// The class one token takes.
///
/// Names the token and not a colour: what a heading looks like is
/// `web::theme`'s to say, and it says it in lightness and weight because
/// this design has two chromatic tokens and both already mean something
/// else.
#[must_use]
pub fn class_of(token: channels::Token) -> &'static str {
    match token {
        channels::Token::Heading => "tok heading",
        channels::Token::Strong => "tok strong",
        channels::Token::Emphasis => "tok emphasis",
        channels::Token::Code => "tok code",
        channels::Token::Fence => "tok fence",
        channels::Token::Meta => "tok meta",
        channels::Token::Link => "tok link",
        channels::Token::Marker => "tok marker",
        channels::Token::Quote => "tok quote",
    }
}
