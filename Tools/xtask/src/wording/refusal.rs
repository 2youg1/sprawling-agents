// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Reading the positions in a plain `.ts` module where a reader is
//! handed a word: the three parts of a refusal.
//!
//! **A `.ts` module draws nothing, and it still speaks.** The gate read
//! `.tsx` only, on the argument that both JSX positions are JSX
//! positions — and the sentences a person meets when a city refuses
//! them live in `core/`, where there is no markup at all. What a broken
//! socket tells somebody to do next was therefore the half of the rule
//! nothing held.
//!
//! **Two positions, both of them a refusal.** `AxError` is this
//! product's one shape for telling a person what failed and what to do
//! about it, so the words a `.ts` module hands a reader arrive as its
//! parts:
//!
//! 1. a **refusal field** — a string written against `action`,
//!    `subject` or `recovery`, which is the shape a refusal is built in;
//! 2. a **refusal argument** — a string passed to a function this module
//!    declares as returning `AxError`, which is the shape a refusal is
//!    built by.
//!
//! Everything else a `.ts` module writes as a string sits somewhere
//! else: a wire value, a storage key, an import path and a class name
//! are none of them in either seat.
//!
//! **The known limit, stated rather than discovered.** A refusal built
//! by a function that hands its own arguments to another one is read at
//! the outer call only, so a sentence passed through two hops arrives
//! unjudged. Widening that needs the call graph a type checker already
//! has, and this is a scanner.

use super::{Said, words_the_view_wrote};

/// What a refusal is spelled with: the three parts of `AxError` a
/// person reads, and the `reason` an enrolment is refused with, which
/// is the same seat under the name that shape gives it. `code` and
/// `nearby` are absent because neither is prose.
const REFUSAL_FIELDS: [&str; 4] = ["action", "subject", "recovery", "reason"];

/// What the client's one refusal type is called.
const REFUSAL_TYPE: &str = "AxError";

/// Every literal that reaches a reader, with the words the module wrote.
pub(super) fn handed_to_a_reader(src: &str) -> Vec<Said> {
    let tokens = tokens(src);
    let refusing = refusal_makers(&tokens);
    let mut out = Vec::new();
    // One entry per open parenthesis: whether its arguments are the
    // parts of a refusal.
    let mut calls: Vec<bool> = Vec::new();
    let mut word: Option<&str> = None;
    let mut key: Option<&str> = None;
    for token in &tokens {
        match *token {
            Token::Word(found) => word = Some(found),
            Token::Mark(mark) => {
                if mark == '(' {
                    calls.push(word.is_some_and(|name| refusing.contains(&name)));
                } else if mark == ')' {
                    calls.pop();
                } else if mark == ':' {
                    key = word;
                } else if matches!(mark, ',' | ';' | '{' | '}') {
                    key = None;
                }
                word = None;
            }
            Token::Text { ref value, line } => {
                let seat = if key.is_some_and(|name| REFUSAL_FIELDS.contains(&name)) {
                    "a refusal field"
                } else if calls.last() == Some(&true) {
                    "a refusal argument"
                } else {
                    continue;
                };
                if let Some(left) = words_the_view_wrote(value) {
                    out.push(Said { line, seat, left });
                }
            }
        }
    }
    out
}

/// The functions this module builds a refusal with, named by their
/// return type rather than by a list kept here: `function mismatch(…):
/// AxError` and `const refuse = (…): AxError =>` both declare one.
fn refusal_makers<'t>(tokens: &[Token<'t>]) -> Vec<&'t str> {
    let mut out = Vec::new();
    // The identifier before the parameter list, and whether that list
    // has closed: a return type is written after it, so the name to
    // keep is the one seen before it opened.
    let mut name: Option<&'t str> = None;
    let mut declared = false;
    let mut depth = 0_usize;
    for token in tokens {
        match *token {
            Token::Word(found) => {
                if depth > 0 {
                    continue;
                }
                if declared && found == REFUSAL_TYPE {
                    out.extend(name.take());
                } else {
                    name = Some(found);
                }
                declared = false;
            }
            Token::Mark(mark) => {
                if mark == '(' {
                    depth = depth.saturating_add(1);
                } else if mark == ')' {
                    depth = depth.saturating_sub(1);
                    declared = depth == 0;
                } else if matches!(mark, '{' | '}' | ';') {
                    name = None;
                    declared = false;
                }
            }
            Token::Text { .. } => {}
        }
    }
    out
}

/// What the scanner reads a module as.
enum Token<'t> {
    /// An identifier or a keyword.
    Word(&'t str),
    /// Any single character that is not part of a word or a string.
    Mark(char),
    /// A string literal, with the line it opened on.
    Text { value: String, line: usize },
}

/// Splits a module into words, marks and strings, dropping comments.
///
/// A comment addresses a contributor rather than somebody using the
/// client, which is the cut `jsx` makes for the same reason.
fn tokens(src: &str) -> Vec<Token<'_>> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let mut at = 0_usize;
    let mut line = 1_usize;
    while let Some(byte) = bytes.get(at).copied() {
        let next = bytes.get(at.saturating_add(1)).copied();
        if byte == b'\n' {
            line = line.saturating_add(1);
            at = at.saturating_add(1);
        } else if byte == b'/' && next == Some(b'/') {
            at = after_line_comment(src, at);
        } else if byte == b'/' && next == Some(b'*') {
            let (to, over) = after_block_comment(src, at);
            line = line.saturating_add(over);
            at = to;
        } else if matches!(byte, b'"' | b'\'' | b'`') {
            let (value, to, over) = string_from(src, at, byte);
            out.push(Token::Text { value, line });
            line = line.saturating_add(over);
            at = to;
        } else if byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$') {
            let to = word_end(bytes, at);
            if let Some(word) = src.get(at..to) {
                out.push(Token::Word(word));
            }
            at = to;
        } else {
            if !byte.is_ascii_whitespace() {
                out.push(Token::Mark(char::from(byte)));
            }
            at = at.saturating_add(1);
        }
    }
    out
}

fn word_end(bytes: &[u8], from: usize) -> usize {
    let mut at = from;
    while bytes
        .get(at)
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'_' | b'$'))
    {
        at = at.saturating_add(1);
    }
    at
}

fn after_line_comment(src: &str, from: usize) -> usize {
    match src.get(from..).and_then(|rest| rest.find('\n')) {
        Some(end) => from.saturating_add(end),
        None => src.len(),
    }
}

/// Where a `/* … */` comment ends, and how many lines it crossed.
fn after_block_comment(src: &str, from: usize) -> (usize, usize) {
    let opened = from.saturating_add(2);
    let Some(rest) = src.get(opened..) else {
        return (src.len(), 0);
    };
    let Some(end) = rest.find("*/") else {
        return (src.len(), rest.lines().count().saturating_sub(1));
    };
    let inside = rest.get(..end).unwrap_or_default();
    (
        opened.saturating_add(end).saturating_add(2),
        inside.matches('\n').count(),
    )
}

/// The body of a string literal, where it ends, and how many lines it
/// crossed. An escape never closes a string; a template literal's
/// `${…}` stays in the body, where the slot reader takes it out.
fn string_from(src: &str, from: usize, quote: u8) -> (String, usize, usize) {
    let bytes = src.as_bytes();
    let mut at = from.saturating_add(1);
    let mut value = String::new();
    let mut over = 0_usize;
    while let Some(byte) = bytes.get(at).copied() {
        if byte == b'\\' {
            at = at.saturating_add(2);
            continue;
        }
        if byte == quote {
            return (value, at.saturating_add(1), over);
        }
        if byte == b'\n' {
            over = over.saturating_add(1);
        }
        if let Some(text) = src.get(at..at.saturating_add(1)) {
            value.push_str(text);
        }
        at = at.saturating_add(1);
    }
    (value, src.len(), over)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::handed_to_a_reader;

    fn found(src: &str) -> Vec<String> {
        handed_to_a_reader(src)
            .into_iter()
            .map(|said| format!("{}: {}", said.seat, said.left))
            .collect()
    }

    /// The half of the rule nothing held: the sentence a person meets
    /// when the socket cannot read what the city sent.
    #[test]
    fn the_parts_of_a_refusal_are_words_a_reader_is_given() {
        let src = r#"
          function refusal(action: string, subject: string): AxError {
            return { code: "E_WIRE_MISMATCH", action, subject, nearby: [] };
          }
          const OUT_OF_ORDER = refusal(
            "join this city's control surface",
            "the server streamed frames before completing the handshake",
          );
        "#;
        assert_eq!(
            found(src),
            [
                "a refusal argument: join this city's control surface",
                "a refusal argument: the server streamed frames before completing the handshake",
            ]
        );
    }

    /// A refusal built as an object states its parts by name.
    #[test]
    fn a_field_of_a_refusal_is_read() {
        let src = r#"
          const refused = { kind: "refused", recovery: "reload the page" };
        "#;
        assert_eq!(found(src), ["a refusal field: reload the page"]);
    }

    /// The shape that would make this gate noise: everything a module
    /// writes for a machine rather than for a person.
    #[test]
    fn wire_values_keys_and_imports_are_not_words_a_reader_was_given() {
        let src = r#"
          import { Frame } from "./wire";
          const KEY = "sprawling.prefs";
          send({ kind: "run_dispatch", at: "lab/desk" });
          if (frame.kind === "welcome") { return "opening"; }
        "#;
        assert!(found(src).is_empty(), "{:?}", found(src));
    }

    /// A comment addresses a contributor, and a call that is not a
    /// refusal is not a seat.
    #[test]
    fn a_comment_and_an_ordinary_call_are_left_alone() {
        let src = r#"
          // the city did not say why, so say that
          log("the city did not say why");
        "#;
        assert!(found(src).is_empty(), "{:?}", found(src));
    }

    /// A template literal hands a reader whatever is left once the
    /// city's own values come out of it.
    #[test]
    fn a_slot_is_not_a_word_the_module_wrote() {
        let src = "const at = { subject: `${addr}/${room}` };";
        assert!(found(src).is_empty(), "{:?}", found(src));
        let mixed = "const at = { subject: `this page speaks wire v${WIRE_V}` };";
        assert_eq!(found(mixed).len(), 1, "{:?}", found(mixed));
    }

    /// The line a person opens is the line the string opened on.
    #[test]
    fn a_finding_names_the_line_the_string_opens_on() {
        let src = "const a = 1;\nconst b = { recovery: \"reload the page\" };\n";
        assert_eq!(handed_to_a_reader(src)[0].line, 2);
    }
}
