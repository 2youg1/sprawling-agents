// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

/// A token, reduced to the seven distinctions the brace walk needs.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    /// A string literal.
    Text,
    /// An identifier, including `if`, `for` and `match`.
    Name,
    /// `{`, the only delimiter that can open an element body.
    Brace,
    /// `(` or `[`, which never can.
    Bracket,
    /// `}`, `)` or `]`.
    Close,
    /// `,` or `;`: one item ends and the next begins.
    Break,
    /// A single `:`, which makes the item before it an attribute name.
    Colon,
    /// Everything else, `::` and `=>` included.
    Other,
}

pub(super) struct Lexeme {
    pub(super) kind: Kind,
    /// The identifier, or a string literal without its quotes.
    pub(super) body: String,
    pub(super) line: usize,
}

pub(super) fn lex(src: &str) -> Vec<Lexeme> {
    let chars: Vec<char> = src.chars().collect();
    let mut out: Vec<Lexeme> = Vec::new();
    let mut at = 0_usize;
    let mut line = 1_usize;
    while let Some(&here) = chars.get(at) {
        let next = chars.get(at.saturating_add(1)).copied();
        if here == '\n' {
            line = line.saturating_add(1);
            at = at.saturating_add(1);
        } else if here.is_whitespace() {
            at = at.saturating_add(1);
        } else if here == '/' && next == Some('/') {
            at = skip_to(&chars, at, '\n');
        } else if here == '/' && next == Some('*') {
            let (to, crossed) = skip_block_comment(&chars, at);
            line = line.saturating_add(crossed);
            at = to;
        } else if here == '\'' {
            at = skip_char_literal(&chars, at);
        } else if let Some((body, to, crossed)) = read_string(&chars, at) {
            out.push(Lexeme {
                kind: Kind::Text,
                body,
                line,
            });
            line = line.saturating_add(crossed);
            at = to;
        } else if here.is_alphabetic() || here == '_' {
            let (body, to) = read_name(&chars, at);
            out.push(Lexeme {
                kind: Kind::Name,
                body,
                line,
            });
            at = to;
        } else {
            let (kind, width) = punctuation(here, next);
            out.push(Lexeme {
                kind,
                body: String::new(),
                line,
            });
            at = at.saturating_add(width);
        }
    }
    out
}

fn punctuation(here: char, next: Option<char>) -> (Kind, usize) {
    match (here, next) {
        (':', Some(':')) => (Kind::Other, 2),
        ('=', Some('>')) => (Kind::Other, 2),
        (':', _) => (Kind::Colon, 1),
        ('{', _) => (Kind::Brace, 1),
        ('(' | '[', _) => (Kind::Bracket, 1),
        ('}' | ')' | ']', _) => (Kind::Close, 1),
        (',' | ';', _) => (Kind::Break, 1),
        _ => (Kind::Other, 1),
    }
}

fn skip_to(chars: &[char], from: usize, stop: char) -> usize {
    let mut at = from;
    while let Some(&here) = chars.get(at) {
        if here == stop {
            return at;
        }
        at = at.saturating_add(1);
    }
    at
}

fn skip_block_comment(chars: &[char], from: usize) -> (usize, usize) {
    let mut at = from.saturating_add(2);
    let mut crossed = 0_usize;
    while let Some(&here) = chars.get(at) {
        if here == '\n' {
            crossed = crossed.saturating_add(1);
        }
        if here == '*' && chars.get(at.saturating_add(1)) == Some(&'/') {
            return (at.saturating_add(2), crossed);
        }
        at = at.saturating_add(1);
    }
    (at, crossed)
}

/// `'a'` and `'\n'` are consumed whole; `'static` is left for the name
/// reader, which spells it as an ordinary identifier and is right to.
fn skip_char_literal(chars: &[char], from: usize) -> usize {
    let escaped = chars.get(from.saturating_add(1)) == Some(&'\\');
    let closes_at_two = chars.get(from.saturating_add(2)) == Some(&'\'');
    if !escaped && !closes_at_two {
        return from.saturating_add(1);
    }
    let mut at = from.saturating_add(1);
    while let Some(&here) = chars.get(at) {
        if here == '\\' {
            at = at.saturating_add(2);
            continue;
        }
        if here == '\'' {
            return at.saturating_add(1);
        }
        at = at.saturating_add(1);
    }
    at
}

/// A string literal starting at `from`, in any of its three spellings.
///
/// Returns its body, the index after it, and how many lines it crossed.
/// `None` when nothing starts here - which is what tells `r#type` from
/// `r#"..."#`: a raw identifier has no quote after its hashes.
fn read_string(chars: &[char], from: usize) -> Option<(String, usize, usize)> {
    let mut at = from;
    let mut hashes = 0_usize;
    if chars.get(at) == Some(&'b') || chars.get(at) == Some(&'r') {
        let raw = chars.get(at) == Some(&'r');
        at = at.saturating_add(1);
        if raw {
            while chars.get(at) == Some(&'#') {
                hashes = hashes.saturating_add(1);
                at = at.saturating_add(1);
            }
        }
        if chars.get(at) != Some(&'"') {
            return None;
        }
        if hashes > 0 {
            return read_raw(chars, at, hashes);
        }
    }
    if chars.get(at) != Some(&'"') {
        return None;
    }
    at = at.saturating_add(1);
    let mut body = String::new();
    let mut crossed = 0_usize;
    while let Some(&here) = chars.get(at) {
        if here == '\\' {
            body.push(here);
            if let Some(&escaped) = chars.get(at.saturating_add(1)) {
                body.push(escaped);
            }
            at = at.saturating_add(2);
            continue;
        }
        if here == '"' {
            return Some((body, at.saturating_add(1), crossed));
        }
        if here == '\n' {
            crossed = crossed.saturating_add(1);
        }
        body.push(here);
        at = at.saturating_add(1);
    }
    Some((body, at, crossed))
}

fn read_raw(chars: &[char], quote: usize, hashes: usize) -> Option<(String, usize, usize)> {
    let mut at = quote.saturating_add(1);
    let mut body = String::new();
    let mut crossed = 0_usize;
    while let Some(&here) = chars.get(at) {
        if here == '"' {
            let closed = (1..=hashes).all(|step| chars.get(at.saturating_add(step)) == Some(&'#'));
            if closed {
                return Some((body, at.saturating_add(hashes).saturating_add(1), crossed));
            }
        }
        if here == '\n' {
            crossed = crossed.saturating_add(1);
        }
        body.push(here);
        at = at.saturating_add(1);
    }
    Some((body, at, crossed))
}

fn read_name(chars: &[char], from: usize) -> (String, usize) {
    let mut at = from;
    let mut body = String::new();
    while let Some(&here) = chars.get(at) {
        if !(here.is_alphanumeric() || here == '_') {
            break;
        }
        body.push(here);
        at = at.saturating_add(1);
    }
    (body, at)
}
