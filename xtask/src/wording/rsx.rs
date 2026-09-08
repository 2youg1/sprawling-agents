// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::lex::{Kind, Lexeme, lex};
use super::{SPOKEN, Said, words_the_view_wrote};

/// One brace, and what its items mean.
struct Frame {
    /// Its items are RSX: attributes and content, not statements.
    element: bool,
    /// The item being read is `name: value`.
    attribute: bool,
    /// The next token opens a fresh item.
    fresh: bool,
    /// The token that opened the item: an identifier, or an attribute
    /// name in quotes. Empty when the item began with anything else.
    head: String,
    /// That token was an identifier rather than a string.
    head_is_name: bool,
}

impl Frame {
    fn plain() -> Frame {
        Frame {
            element: false,
            attribute: false,
            fresh: true,
            head: String::new(),
            head_is_name: false,
        }
    }

    fn element() -> Frame {
        Frame {
            element: true,
            ..Frame::plain()
        }
    }
}

/// Every literal that reaches a reader, with the words the view wrote.
pub(super) fn handed_to_a_reader(src: &str) -> Vec<Said> {
    let toks = lex(src);
    let mut stack: Vec<Frame> = vec![Frame::plain()];
    let mut out = Vec::new();
    for (index, tok) in toks.iter().enumerate() {
        let follows = toks.get(index.saturating_add(1)).map(|next| next.kind);
        let Some(top) = stack.last_mut() else {
            continue;
        };
        let fresh = top.fresh;
        if fresh && tok.kind != Kind::Close {
            top.head = tok.body.clone();
            top.head_is_name = tok.kind == Kind::Name;
            top.fresh = false;
            top.attribute = top.element
                && follows == Some(Kind::Colon)
                && matches!(tok.kind, Kind::Name | Kind::Text);
        }
        if tok.kind == Kind::Text
            && let Some(seat) = seat_of(top, fresh)
            && let Some(left) = words_the_view_wrote(&tok.body)
        {
            out.push(Said {
                line: tok.line,
                seat,
                left,
            });
        }
        step(&mut stack, &toks, index);
    }
    out
}

/// Where this literal sits, when it sits somewhere a reader can see.
fn seat_of(top: &Frame, fresh: bool) -> Option<&'static str> {
    if top.element && !top.attribute && fresh {
        return Some("a text node");
    }
    if top.attribute && !fresh && SPOKEN.contains(&top.head.as_str()) {
        return Some("a spoken attribute");
    }
    None
}

/// Push and pop the brace stack for one token.
fn step(stack: &mut Vec<Frame>, toks: &[Lexeme], index: usize) {
    let Some(tok) = toks.get(index) else {
        return;
    };
    match tok.kind {
        Kind::Brace => {
            let element = opens_an_element_body(stack, toks, index);
            stack.push(if element {
                Frame::element()
            } else {
                Frame::plain()
            });
        }
        Kind::Bracket => stack.push(Frame::plain()),
        Kind::Close => {
            if stack.len() > 1 {
                stack.pop();
            }
            if let Some(top) = stack.last_mut() {
                top.fresh = false;
            }
        }
        Kind::Break => {
            if let Some(top) = stack.last_mut() {
                top.fresh = true;
                top.attribute = false;
                top.head.clear();
                top.head_is_name = false;
            }
        }
        _ => {}
    }
}

/// Whether the brace at `index` opens an element body.
///
/// Two ways in. `rsx!` opens one wherever it appears, which is how a
/// match arm gets back into RSX. Otherwise the item has to begin with
/// an identifier inside an element body that is not reading an
/// attribute: `div {`, `Panel {`, `if x {` and `for a in b {` all do.
/// `match m {` is refused by name - its items are patterns, and a
/// pattern that matches a string is not a reader being handed one.
fn opens_an_element_body(stack: &[Frame], toks: &[Lexeme], index: usize) -> bool {
    let bang = index
        .checked_sub(1)
        .and_then(|at| toks.get(at))
        .is_some_and(|tok| tok.kind == Kind::Other);
    let macro_name = index
        .checked_sub(2)
        .and_then(|at| toks.get(at))
        .is_some_and(|tok| tok.kind == Kind::Name && tok.body == "rsx");
    if bang && macro_name {
        return true;
    }
    stack
        .last()
        .is_some_and(|top| top.element && !top.attribute && top.head_is_name && top.head != "match")
}
