// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Reading the two positions in a Svelte template where a view hands a
//! reader a word.
//!
//! **This is a scanner over the two shapes, not a parser of the
//! language.** A full parser exists in the client's own toolchain and
//! writing a second one here would be a worse copy of it. What this
//! needs is narrower than a parse: a template puts a reader's words in
//! exactly two places, and both are recognisable from the characters
//! around them.
//!
//! 1. A **text node** - a run between a tag that closed and the next
//!    tag or expression hole that opens, which is what the browser
//!    paints.
//! 2. A **spoken attribute** written as a string - `aria-label="…"` and
//!    its seven relatives, which is what a screen reader reads out.
//!
//! **What the template does not draw is cut before either is read.**
//! A `<script>` block is TypeScript and a `<style>` block is CSS: both
//! hold plenty of literals and none of them reach a reader's eyes, so
//! the scanner walks the line as segments and keeps only what sits
//! outside those blocks and outside `<!-- … -->` comments. This is why
//! the file carries no `//` cutting either: in markup a slash-slash
//! begins nothing, and cutting at it would swallow the URLs a text
//! node legitimately carries.
//!
//! **The known limits, stated rather than discovered.** A text run
//! interrupted by an expression hole keeps only what precedes the
//! hole, and a run broken across lines is read line by line - both are
//! quiet rather than noisy, which is the failure this gate can afford:
//! the words a view draws are also drawn on `#/gallery`, where `render`
//! reads them off the page.

use super::{SPOKEN, Said, words_the_view_wrote};

/// Every literal in the drawn part of a template, with the words the
/// view wrote.
pub(super) fn handed_to_a_reader(src: &str) -> Vec<Said> {
    let mut out = Vec::new();
    let mut block: Option<&'static str> = None;
    let mut comment = false;
    for (index, line) in src.lines().enumerate() {
        let number = index.saturating_add(1);
        let drawn = visible(line, &mut block, &mut comment);
        for said in text_nodes(&drawn) {
            if let Some(left) = words_the_view_wrote(&said) {
                out.push(Said {
                    line: number,
                    seat: "a text node",
                    left,
                });
            }
        }
        for said in spoken_attributes(&drawn) {
            if let Some(left) = words_the_view_wrote(&said) {
                out.push(Said {
                    line: number,
                    seat: "a spoken attribute",
                    left,
                });
            }
        }
    }
    out
}

/// The part of a line the browser draws: everything outside a
/// `<script>`/`<style>` block and outside an HTML comment. Both states
/// carry across lines, which is the whole reason they are the caller's
/// rather than this function's.
fn visible(line: &str, block: &mut Option<&'static str>, comment: &mut bool) -> String {
    let mut out = String::new();
    let mut rest = line;
    loop {
        if *comment {
            match rest.find("-->") {
                Some(at) => {
                    *comment = false;
                    rest = rest.get(at.saturating_add(3)..).unwrap_or("");
                }
                None => return out,
            }
            continue;
        }
        if let Some(kind) = *block {
            let close = format!("</{kind}>");
            match rest.find(&close) {
                Some(at) => {
                    *block = None;
                    rest = rest.get(at.saturating_add(close.len())..).unwrap_or("");
                }
                None => return out,
            }
            continue;
        }
        // Not inside either: the earliest opener wins. An opener that
        // is not followed by a tag boundary is ordinary text - the word
        // `scripting` starts with the same six characters - so it is
        // kept and the scan resumes after it.
        const OPENERS: [&str; 3] = ["<!--", "<script", "<style"];
        let next = OPENERS
            .iter()
            .filter_map(|opener| rest.find(opener).map(|at| (at, *opener)))
            .min_by_key(|(at, _)| *at);
        let Some((at, opener)) = next else {
            out.push_str(rest);
            return out;
        };
        if let Some(head) = rest.get(..at) {
            out.push_str(head);
        }
        let after = rest.get(at.saturating_add(opener.len())..).unwrap_or("");
        if opener == "<!--" {
            *comment = true;
        } else if after.starts_with('>')
            || after
                .chars()
                .next()
                .is_some_and(|mark| mark.is_whitespace() || mark == '/')
        {
            *block = Some(if opener == "<script" {
                "script"
            } else {
                "style"
            });
        } else {
            out.push_str(opener);
        }
        rest = after;
    }
}

/// The runs of text that sit between a tag and the next tag or
/// expression hole.
///
/// `=>` is not a tag closing: an arrow is the one spelling of `>` that
/// routinely has words after it, and reading it as markup would report
/// every callback in the client.
fn text_nodes(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = code.as_bytes();
    let mut index: usize = 0;
    // What each still-open `<` on this line was: an element, or a type
    // argument list inside an inline expression. A `<` written
    // directly after an identifier character opens type arguments -
    // `Choice<V>`, `held<View>` - because a tag never spells an element
    // that way: the `<` of a tag follows whitespace, a bracket, a brace
    // or the start of the line.
    let mut opened: Vec<Opening> = Vec::new();
    while let Some(byte) = bytes.get(index) {
        let after = bytes.get(index.saturating_add(1));
        if *byte == b'<' {
            // `<=` compares; it neither opens an element nor a type
            // argument list, and pushing it would leave the stack one
            // deep for the rest of the line.
            if after != Some(&b'=') {
                let before = index.checked_sub(1).and_then(|at| bytes.get(at));
                opened.push(match before {
                    Some(mark) if mark.is_ascii_alphanumeric() || *mark == b'_' => {
                        Opening::TypeArgs
                    }
                    _ => Opening::Element,
                });
            }
            index = index.saturating_add(1);
            continue;
        }
        if *byte != b'>' {
            index = index.saturating_add(1);
            continue;
        }
        let before = index.checked_sub(1).and_then(|at| bytes.get(at));
        // `=>` is an arrow and `>=` is a comparison. Neither closes a
        // tag, and both routinely have words after them.
        if before == Some(&b'=') || after == Some(&b'=') {
            index = index.saturating_add(1);
            continue;
        }
        if opened.pop() == Some(Opening::TypeArgs) {
            index = index.saturating_add(1);
            continue;
        }
        let start = index.saturating_add(1);
        let rest = code.get(start..).unwrap_or("");
        let end = rest.find(['<', '{', '}']).unwrap_or(rest.len());
        // Only a closing tag ends a text node. Anything else after `>`
        // is the rest of an expression hole.
        if rest.get(end..).is_some_and(|tail| tail.starts_with('<'))
            && let Some(run) = rest.get(..end)
        {
            out.push(run.to_owned());
        }
        index = start.saturating_add(end);
    }
    out
}

/// What a `<` opened, which decides whether the `>` that closes it can
/// begin a run of text.
#[derive(PartialEq, Eq)]
enum Opening {
    Element,
    TypeArgs,
}

/// The value of a spoken attribute, when it is written as a string
/// rather than taken from an expression.
fn spoken_attributes(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    for name in SPOKEN {
        let mark = format!("{name}=\"");
        let mut from: usize = 0;
        while let Some(found) = code.get(from..).and_then(|rest| rest.find(&mark)) {
            let opens = from.saturating_add(found).saturating_add(mark.len());
            let Some(rest) = code.get(opens..) else { break };
            let Some(close) = rest.find('"') else { break };
            if let Some(value) = rest.get(..close) {
                out.push(value.to_owned());
            }
            from = opens.saturating_add(close);
        }
    }
    out
}
