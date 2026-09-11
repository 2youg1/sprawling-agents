// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Reading the two positions in TSX where a view hands a reader a word.
//!
//! **This is a scanner over the two shapes, not a parser of the
//! language.** A full TSX parser exists in the client's own toolchain and
//! writing a second one here would be a worse copy of it. What this needs
//! is narrower than a parse: JSX puts a reader's words in exactly two
//! places, and both are recognisable from the characters around them.
//!
//! 1. A **text node** - a run between a tag that closed and the next tag
//!    or expression that opens, which is what the browser paints.
//! 2. A **spoken attribute** written as a string - `aria-label="…"` and
//!    its seven relatives, which is what a screen reader reads out.
//!
//! Everything a view writes for the machine sits somewhere else: a class
//! list, a route, a wire value and an event name are attribute values
//! that are not spoken, or expressions, or arguments to a call.
//!
//! **A text run counts only when a tag closes it.** `>` is also how a
//! type argument list ends, and `createSignal<View>(DEFAULT_VIEW);` has
//! words after its `>` exactly the way an element does. What separates
//! them is what comes next: an element's text is followed by the tag that
//! ends it, and a generic's is followed by the rest of a statement. The
//! first run of this scanner reported ten of those, every one a type.
//!
//! **The known limits, stated rather than discovered.** A text node
//! broken across lines is read line by line, so its first line carries no
//! closing tag and is not seen. Only `.tsx` is read, because both
//! positions are JSX positions and a `.ts` module hands a reader nothing
//! directly. Both limits are quiet rather than noisy, which is the
//! failure this gate can afford: the words a view draws are also drawn on
//! `#/gallery`, where `render` reads them off the page.

use super::{SPOKEN, Said, words_the_view_wrote};

/// Every literal that reaches a reader, with the words the view wrote.
pub(super) fn handed_to_a_reader(src: &str) -> Vec<Said> {
    let mut out = Vec::new();
    for (index, line) in src.lines().enumerate() {
        let number = index.saturating_add(1);
        let code = before_a_comment(line);
        for said in text_nodes(code) {
            if let Some(left) = words_the_view_wrote(&said) {
                out.push(Said {
                    line: number,
                    seat: "a text node",
                    left,
                });
            }
        }
        for said in spoken_attributes(code) {
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

/// The code on a line, with a trailing `//` comment removed.
///
/// A comment is prose for the next person, and holding it to the phrase
/// table would be holding the wrong thing: the reader it addresses is a
/// contributor, not somebody using the client.
fn before_a_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut quote: Option<u8> = None;
    let mut index: usize = 0;
    while let Some(byte) = bytes.get(index) {
        match quote {
            Some(open) if *byte == open => quote = None,
            Some(_) => {}
            None if matches!(*byte, b'"' | b'\'' | b'`') => quote = Some(*byte),
            None if *byte == b'/' && bytes.get(index.saturating_add(1)) == Some(&b'/') => {
                return line.get(..index).unwrap_or(line);
            }
            None => {}
        }
        index = index.saturating_add(1);
    }
    line
}

/// The runs of text that sit between a tag and the next tag or
/// expression.
///
/// `=>` is not a tag closing: an arrow is the one spelling of `>` that
/// routinely has words after it, and reading it as markup would report
/// every callback in the client.
fn text_nodes(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = code.as_bytes();
    let mut index: usize = 0;
    while let Some(byte) = bytes.get(index) {
        if *byte != b'>' {
            index = index.saturating_add(1);
            continue;
        }
        let before = index.checked_sub(1).and_then(|at| bytes.get(at));
        if before == Some(&b'=') {
            index = index.saturating_add(1);
            continue;
        }
        let start = index.saturating_add(1);
        let rest = code.get(start..).unwrap_or("");
        let end = rest.find(['<', '{', '}']).unwrap_or(rest.len());
        // Only a closing tag ends a text node. Anything else after `>`
        // is the rest of an expression.
        if rest.get(end..).is_some_and(|tail| tail.starts_with('<'))
            && let Some(run) = rest.get(..end)
        {
            out.push(run.to_owned());
        }
        index = start.saturating_add(end);
    }
    out
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
