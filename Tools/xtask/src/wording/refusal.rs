// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::{Said, words_the_view_wrote};

const REFUSAL_FIELDS: [&str; 4] = ["action", "subject", "recovery", "reason"];

const REFUSAL_TYPE: &str = "AxError";

pub(super) fn handed_to_a_reader(src: &str) -> Vec<Said> {
    let tokens = tokens(src);
    let refusing = refusal_makers(&tokens);
    let mut out = Vec::new();

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

fn refusal_makers<'t>(tokens: &[Token<'t>]) -> Vec<&'t str> {
    let mut out = Vec::new();

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

enum Token<'t> {
    Word(&'t str),

    Mark(char),

    Text { value: String, line: usize },
}

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

    #[test]
    fn a_field_of_a_refusal_is_read() {
        let src = r#"
          const refused = { kind: "refused", recovery: "reload the page" };
        "#;
        assert_eq!(found(src), ["a refusal field: reload the page"]);
    }

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

    #[test]
    fn a_comment_and_an_ordinary_call_are_left_alone() {
        let src = r#"
          // the city did not say why, so say that
          log("the city did not say why");
        "#;
        assert!(found(src).is_empty(), "{:?}", found(src));
    }

    #[test]
    fn a_slot_is_not_a_word_the_module_wrote() {
        let src = "const at = { subject: `${addr}/${room}` };";
        assert!(found(src).is_empty(), "{:?}", found(src));
        let mixed = "const at = { subject: `this page speaks wire v${WIRE_V}` };";
        assert_eq!(found(mixed).len(), 1, "{:?}", found(mixed));
    }

    #[test]
    fn a_finding_names_the_line_the_string_opens_on() {
        let src = "const a = 1;\nconst b = { recovery: \"reload the page\" };\n";
        assert_eq!(handed_to_a_reader(src)[0].line, 2);
    }
}
