// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Wording gate: a word a reader is given comes from the phrase table
//! (web-SPEC.md section 8-61).
//!
//! **The two assertions in `web::lang` both read what a view *asks* the
//! table for.** One walks every phrase and demands some view name it;
//! the other forbids a view from spelling out a sentence the table
//! already holds. A sentence that never calls `say` is in neither
//! field of view: the table does not know it exists, so there is
//! nothing to compare it against. Three English sentences lived on the
//! cost page for a whole stage that way, and what found them was a
//! photograph of the running client.
//!
//! **This gate reads the other direction: what a view *says* by
//! itself.** It is the half only a gate can do, for the same reason
//! `color`'s literal scan is - it is a statement about every file
//! rather than about one table.
//!
//! **Two languages of position, one rule.** A `.tsx` view hands a
//! reader words through markup, which `jsx` reads; a `.ts` module hands
//! them the parts of a refusal, which `refusal` reads. Reading markup
//! only left the sentences a person meets when a city refuses them
//! - the recovery lines in `core/` - outside every reader of this rule.
//!
//! A file a generator wrote is not somebody's module, so its words are
//! judged where the generator is.
//!
//! Everything below - the waiver, the slot reader, the two-letter
//! test - is shared by both, because the question they answer is one
//! question.
//!
//! Two loops rather than one: which positions a file has is decided by
//! what the file is written in, and reading that off a path inside the
//! loop would be the same decision made later and less plainly.
//!
//! **The predicate is a position, not a vocabulary.** A first cut that
//! scanned every string literal would judge class names, wire values,
//! event kinds and format keys, and the same shape of mistake once
//! produced 79 findings that were all addresses. So this walks the RSX
//! brace structure and keeps two positions, both of which are a reader
//! being handed a word:
//!
//! 1. a **text node** - a literal standing on its own in an element
//!    body, which is what the browser paints;
//! 2. the value of a **spoken attribute** - `placeholder`, `title`,
//!    `alt` and the `aria-*` names whose value is author-supplied text,
//!    which is what a screen reader reads out.
//!
//! Everything else is excluded by where it sits rather than by a list
//! of exceptions: an attribute value is not a text node, a call
//! argument is not in an element body, a match arm is not content.
//!
//! **What remains after the city's own values are removed is the
//! question.** `"{percent}%"`, `"+{added}"` and `"{room}/"` hand the
//! reader nothing the view wrote; `"{count} waiting"` hands them an
//! English word. So the slots come out and the gate asks whether two
//! adjacent letters are left.
//!
//! **A proper noun cannot go in the table**, because `web::lang`'s own
//! test refuses a phrase whose two languages are equal, and `openai` is
//! `openai` in both. Those sites carry `wording-ok: <reason>` on the
//! line or the line above - the same mark, the same two-line rule and
//! the same trade as `lexicon-ok:`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::report::{Violation, XtaskError};
use crate::walk;

/// Where the client a reader reads lives. Nothing else in the tree
/// draws, and English in a wire value or an error code is correct. The
/// directory is `walk`'s to state, not this gate's.
use crate::walk::CLIENT_SRC as CLIENT;

/// The waiver, spelled as `lexicon`'s is.
const EXEMPT_MARK: &str = "wording-ok:";

/// Attributes whose value is author-supplied text that a person is
/// shown or read out. A closed vocabulary from HTML and ARIA, not a
/// list of the ones this tree happens to use today: the day someone
/// writes `alt`, the rule already covers it.
const SPOKEN: [&str; 8] = [
    "placeholder",
    "title",
    "alt",
    "aria-label",
    "aria-description",
    "aria-placeholder",
    "aria-roledescription",
    "aria-valuetext",
];

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let dir = root.join(CLIENT);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let phrases = keys(root)?;
    let mut violations = Vec::new();
    let markup = Rule {
        read: jsx::handed_to_a_reader,
        phrases: &phrases,
    };
    for path in walk::files_with_ext(&dir, &["tsx"])? {
        judge(root, &path, &markup, &mut violations)?;
    }
    let modules = Rule {
        read: refusal::handed_to_a_reader,
        phrases: &phrases,
    };
    for path in walk::files_with_ext(&dir, &["ts"])? {
        judge(root, &path, &modules, &mut violations)?;
    }
    Ok(violations)
}

/// Every key `lang.json` defines.
///
/// A literal that is one of them is not a word the view wrote: it is
/// the name of a phrase, and whoever it is handed to resolves it. The
/// rule is about where words come from, and a key is not a word.
fn keys(root: &Path) -> Result<BTreeSet<String>, XtaskError> {
    let path = root.join(CLIENT).join("lang.json");
    let text = walk::read_text(&path)?;
    let table: BTreeMap<String, serde_json::Value> =
        serde_json::from_str(&text).map_err(|err| XtaskError::Doc {
            file: walk::rel(root, &path),
            msg: format!("this file does not parse as JSON: {err}"),
        })?;
    Ok(table.into_keys().collect())
}

/// What one judgement needs besides the file it is about: the reader
/// its language needs, and the phrases `lang.json` defines. Both are
/// the same for every file in one run, so they travel as one value
/// rather than as two more parameters.
struct Rule<'a> {
    read: fn(&str) -> Vec<Said>,
    phrases: &'a BTreeSet<String>,
}

/// Holds one file to the rule, through the reader its language needs.
fn judge(
    root: &Path,
    path: &Path,
    rule: &Rule<'_>,
    violations: &mut Vec<Violation>,
) -> Result<(), XtaskError> {
    let location = walk::rel(root, path);
    // A test may quote a sentence to assert that a page says it. That
    // is evidence, not a second authority for the wording.
    if location.contains(".test.") {
        return Ok(());
    }
    let text = walk::read_text(path)?;
    if crate::length::generated(&text) {
        return Ok(());
    }
    let lines: Vec<&str> = text.lines().collect();
    for said in (rule.read)(drawn(&text)) {
        if waived(&lines, said.line) || rule.phrases.contains(&said.left) {
            continue;
        }
        violations.push(Violation {
            gate: "wording",
            location: format!("{location}:{}", said.line),
            rule: "a word a reader is given comes from lang.json, not from the view".to_owned(),
            violation: format!(
                "{} carries {:?}, which no phrase produced",
                said.seat,
                clipped(&said.left)
            ),
            alternative: format!(
                "add a Msg with both languages and fill its named slots; or, for a name that \
                 is the same word in both, justify inline with `{EXEMPT_MARK} <reason>`"
            ),
        });
    }
    Ok(())
}

/// The part of a module that draws, which is everything above its own
/// test module. The same cut `web::lang` makes, for the same reason: a
/// sentence quoted by a test to assert that a page says it is evidence,
/// not a second authority for the wording.
fn drawn(body: &str) -> &str {
    match body.find("#[cfg(test)]") {
        Some(at) => body.get(..at).unwrap_or(body),
        None => body,
    }
}

/// A waiver on the line itself or the line above it.
fn waived(lines: &[&str], line: usize) -> bool {
    let here = line.checked_sub(1).and_then(|at| lines.get(at));
    let above = line.checked_sub(2).and_then(|at| lines.get(at));
    [here, above]
        .into_iter()
        .flatten()
        .any(|text| text.contains(EXEMPT_MARK))
}

/// The first 60 characters, so a report of forty findings stays
/// readable and a wall of markup never becomes the message.
fn clipped(text: &str) -> String {
    let mut out: String = text.chars().take(60).collect();
    if out.chars().count() < text.chars().count() {
        out.push('…');
    }
    out
}

/// One literal a reader is handed, and the words left in it.
struct Said {
    line: usize,
    seat: &'static str,
    left: String,
}

mod jsx;
mod refusal;

// -------------------------------------------------------- the predicate

/// What a literal says once the city's own values are taken out of it,
/// or `None` when nothing of the view's own is left.
///
/// A slot carries a name, a number or an address the city produced; an
/// escape carries a character the view spelled in hexadecimal. Neither
/// is the view choosing a word. What survives both is judged by the
/// only test that matters here: two letters side by side.
fn words_the_view_wrote(literal: &str) -> Option<String> {
    let mut left = String::new();
    let mut chars = literal.chars().peekable();
    let mut depth = 0_usize;
    while let Some(here) = chars.next() {
        match here {
            '\\' => {
                if chars.next() == Some('u') && chars.peek() == Some(&'{') {
                    for inner in chars.by_ref() {
                        if inner == '}' {
                            break;
                        }
                    }
                }
            }
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
            }
            '{' => depth = depth.saturating_add(1),
            '}' => depth = depth.saturating_sub(1),
            _ if depth == 0 => left.push(here),
            _ => {}
        }
    }
    let mut run = 0_usize;
    for here in left.chars() {
        run = if here.is_ascii_alphabetic() {
            run.saturating_add(1)
        } else {
            0
        };
        if run >= 2 {
            return Some(left.trim().to_owned());
        }
    }
    None
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests;
