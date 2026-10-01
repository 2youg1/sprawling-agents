// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How long a Zig function is, and how many of a Zig file's lines are
//! production, read from Zig's tokens (xtask-SPEC.md sections 8-48 and
//! 12-15).
//!
//! **Counting braces is exact in Zig once four kinds of token are
//! skipped**, which is the reason it is wrong for Rust and right here.
//! The language reference says it of comments: "There are no multiline
//! comments in Zig ... This allows Zig to have the property that each
//! line of code can be tokenized out of context." A string or a
//! character literal cannot hold a newline, and the multiline string,
//! a line opened by `\\`, runs to the end of its line as a comment
//! does. So a line comment, a string, a character literal and a
//! multiline string line are skipped, and every brace and parenthesis
//! left is structure.
//!
//! The one place a brace comes before a function's body is its return
//! type, and there it follows `error`, a container keyword (with an
//! optional parenthesised argument, as in `union(enum)`), `switch (…)`,
//! or a block label. Those groups are passed over, and the first other
//! `{` opens the body.

use std::iter::Peekable;
use std::ops::RangeInclusive;
use std::str::Chars;

use super::measurement::Body;

/// The words after which a `{` in a return type opens a type rather
/// than the function's body (the grammar's `ErrorSetDecl`,
/// `ContainerDeclType` and `SwitchExpr`).
const OPENS_TYPE: [&str; 6] = ["error", "struct", "enum", "union", "opaque", "switch"];

/// What the scanner keeps of a Zig file.
#[derive(Debug, PartialEq, Eq)]
enum Kind {
    /// An identifier, a keyword or a number.
    Word(String),
    /// One character of punctuation; only braces, parentheses, `;` and
    /// `:` decide anything, and the others end a run of words.
    Mark(char),
    /// A string or a character literal, whose content is never structure.
    Literal,
}

#[derive(Debug)]
struct Token {
    kind: Kind,
    line: usize,
}

impl Token {
    fn is_word(&self, word: &str) -> bool {
        matches!(&self.kind, Kind::Word(found) if found == word)
    }

    fn is_mark(&self, mark: char) -> bool {
        self.kind == Kind::Mark(mark)
    }
}

/// Every named function with a body, from its `fn` to the line of its
/// closing brace. A prototype (an `extern` declaration) has no body and
/// is not measured, and neither is a function inside a `test`
/// declaration, which only `zig test` compiles.
pub(super) fn measure(text: &str) -> Vec<Body> {
    let tokens = tokens(text);
    let tests = test_spans(&tokens);
    tokens
        .iter()
        .enumerate()
        .filter(|(at, token)| token.is_word("fn") && !tests.iter().any(|span| span.contains(at)))
        .filter_map(|(at, token)| {
            let name = match &tokens.get(at.checked_add(1)?)?.kind {
                Kind::Word(name) => name.clone(),
                Kind::Mark(_) | Kind::Literal => return None,
            };
            let end = body_end(&tokens, at.checked_add(2)?)?;
            Some(Body {
                name,
                line: token.line,
                lines: end.line.saturating_sub(token.line).saturating_add(1),
            })
        })
        .collect()
}

/// The lines of a Zig file the file budget counts: every line, less the
/// lines each `test` declaration spans, because a test is not
/// production code the budget prices.
pub(super) fn production_lines(text: &str) -> usize {
    let tokens = tokens(text);
    let spans = test_spans(&tokens);
    let test_lines = spans
        .iter()
        .filter(|span| {
            !spans
                .iter()
                .any(|outer| outer != *span && outer.contains(span.start()))
        })
        .filter_map(|span| {
            let first = tokens.get(*span.start())?.line;
            let last = tokens.get(*span.end())?.line;
            Some(last.saturating_sub(first).saturating_add(1))
        })
        .fold(0_usize, usize::saturating_add);
    text.lines().count().saturating_sub(test_lines)
}

/// The token that ends the body of the function whose parameter list
/// opens at `params`, or `None` for a prototype. A body left open runs
/// to the last token, since `zig fmt --check` and the build refuse that
/// file before anyone reads this measurement.
fn body_end(tokens: &[Token], params: usize) -> Option<&Token> {
    if !tokens.get(params)?.is_mark('(') {
        return None;
    }
    let mut at = closing(tokens, params)?;
    // Whether the token just read makes the next `{` a type's braces.
    let mut typed = false;
    loop {
        at = at.checked_add(1)?;
        let token = tokens.get(at)?;
        match &token.kind {
            Kind::Mark(';') => return None,
            Kind::Mark('{') if !typed => {
                return closing(tokens, at)
                    .and_then(|close| tokens.get(close))
                    .or_else(|| tokens.last());
            }
            Kind::Mark('{') => {
                at = closing(tokens, at)?;
                typed = false;
            }
            // `union(enum)` and `switch (x)`: the group belongs to the
            // word before it, and the brace after it is still a type's.
            Kind::Mark('(') => at = closing(tokens, at)?,
            Kind::Mark(':') => typed = true,
            Kind::Word(word) => typed = OPENS_TYPE.contains(&word.as_str()),
            Kind::Mark(_) | Kind::Literal => typed = false,
        }
    }
}

/// The index of the token that closes the brace or parenthesis at
/// `open`.
fn closing(tokens: &[Token], open: usize) -> Option<usize> {
    let (opener, closer) = match tokens.get(open)?.kind {
        Kind::Mark('(') => ('(', ')'),
        Kind::Mark('{') => ('{', '}'),
        Kind::Word(_) | Kind::Mark(_) | Kind::Literal => return None,
    };
    let mut depth = 0_usize;
    for (at, token) in tokens.iter().enumerate().skip(open) {
        if token.is_mark(opener) {
            depth = depth.saturating_add(1);
        } else if token.is_mark(closer) {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(at);
            }
        }
    }
    None
}

/// Where each `test` declaration lies: from the keyword to the brace
/// that closes its block. `test` is a keyword, so it names nothing
/// else; its optional name is a string or an identifier.
fn test_spans(tokens: &[Token]) -> Vec<RangeInclusive<usize>> {
    tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.is_word("test"))
        .filter_map(|(at, _)| {
            let (open, _) = tokens
                .iter()
                .enumerate()
                .skip(at.checked_add(1)?)
                .take(2)
                .find(|(_, token)| token.is_mark('{'))?;
            Some(at..=closing(tokens, open)?)
        })
        .collect()
}

/// The file as words and punctuation, each with its one-based line,
/// with comments, strings, character literals and multiline string
/// lines left out.
fn tokens(text: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut line = 1_usize;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\n' => line = line.saturating_add(1),
            '/' if chars.peek() == Some(&'/') => to_line_end(&mut chars),
            '\\' if chars.peek() == Some(&'\\') => to_line_end(&mut chars),
            '"' | '\'' => {
                past_literal(&mut chars, c);
                out.push(Token {
                    kind: Kind::Literal,
                    line,
                });
            }
            c if c.is_alphanumeric() || c == '_' => {
                let mut word = String::from(c);
                while let Some(next) = chars.next_if(|&n| n.is_alphanumeric() || n == '_') {
                    word.push(next);
                }
                out.push(Token {
                    kind: Kind::Word(word),
                    line,
                });
            }
            c if c.is_whitespace() => {}
            c => out.push(Token {
                kind: Kind::Mark(c),
                line,
            }),
        }
    }
    out
}

/// Consumes the rest of the line, leaving its newline to be counted.
fn to_line_end(chars: &mut Peekable<Chars<'_>>) {
    while chars.next_if(|&n| n != '\n').is_some() {}
}

/// Consumes a literal up to its closing `quote`, an escape included. A
/// literal cannot cross a line, so the newline ends one left open.
fn past_literal(chars: &mut Peekable<Chars<'_>>, quote: char) {
    while let Some(c) = chars.next_if(|&n| n != '\n') {
        if c == quote || (c == '\\' && chars.next_if(|&n| n != '\n').is_none()) {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{measure, production_lines};

    fn measured(text: &str) -> Vec<(String, usize, usize)> {
        measure(text)
            .into_iter()
            .map(|body| (body.name, body.line, body.lines))
            .collect()
    }

    /// A return type may hold braces of its own; the body is the block
    /// after it, as `leaf.zig`'s `open` and an error set show.
    #[test]
    fn braces_in_a_return_type_are_not_the_body() {
        let text = "fn open() union(enum) { held: u8, ended: u8 } {\n    return .{ .held = 0 };\n}\n\
                    fn fails() error{Oops}!void {\n}\n";
        assert_eq!(
            measured(text),
            [("open".to_owned(), 1, 3), ("fails".to_owned(), 4, 2)]
        );
    }

    /// The counting bug the Rust measurement names, in Zig's spellings:
    /// a brace in a character, in a string, in a multiline string line
    /// and in a comment is not a block.
    #[test]
    fn a_brace_inside_a_literal_or_a_comment_is_not_a_block() {
        let text = "// fn fake() void {\nfn detect() void {\n    const c = '{';\n    \
                    const s = \"}\\\"{\";\n    const m =\n        \\\\ }}}\n    ;\n}\nfn after() void {}\n";
        assert_eq!(
            measured(text),
            [("detect".to_owned(), 2, 7), ("after".to_owned(), 9, 1)]
        );
    }

    /// A prototype has no body, and a function type has no name.
    #[test]
    fn a_prototype_and_a_function_type_are_not_measured() {
        let text = "extern \"user32\" fn GetDC(window: ?HWND) callconv(.winapi) ?HDC;\n\
                    const Proc = *const fn (u8) callconv(.winapi) void;\n";
        assert!(measured(text).is_empty());
    }

    /// A `test` declaration, named or not, is neither counted into the
    /// file nor measured, as a `#[cfg(test)]` item is not.
    #[test]
    fn a_test_declaration_is_neither_counted_nor_measured() {
        let text = "const a = 0;\ntest \"named\" {\n    const S = struct {\n        \
                    fn inner() void {}\n    };\n    _ = S;\n}\ntest {\n}\n";
        assert!(measured(text).is_empty());
        assert_eq!(production_lines(text), 1);
    }
}
