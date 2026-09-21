// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! From a box on the screen back to the line that drew it
//! (xtask-SPEC.md section 8-26).
//!
//! **A finding is an edit, not a complaint** - the module heading says
//! so, and until this file existed the report fell one step short of
//! it. It named the box by its class attribute and left the reader to
//! grep:
//!
//! ```text
//! the left edge of div `已就绪的数量` reads 579 … class="h-snug min-w-0 flex-1 …"
//! ```
//!
//! A person can search for that. An agent pays for the search, and the
//! search is ambiguous the moment two components share a utility run.
//! With the line named, the next step is one `edit` call:
//!
//! ```text
//! client/src/views/machine.tsx:118 · the left edge … reads 579
//! ```
//!
//! **The match is by the longest literal, and it is honest about
//! missing.** A class attribute on the screen is what the cascade
//! resolved; in the source it is often several literals with a
//! condition between them, so no exact key exists. What does exist is
//! the longest quoted run the source spells, and a painted attribute
//! that contains it came from that line or from one that copied it.
//! Where nothing matches, the reading says nothing rather than
//! guessing: a wrong location costs more than an absent one, because a
//! reader who opens the wrong file trusts the next location less.

use std::collections::BTreeMap;
use std::path::Path;

use crate::report::XtaskError;
use crate::walk::{self, CLIENT_SRC};

/// The file kinds a class literal can be written in.
const DRAWN_IN: [&str; 3] = ["tsx", "ts", "html"];

/// How long a literal has to be before it identifies anything.
///
/// A run of three utilities is about this long, and shorter runs -
/// `flex`, `truncate`, `min-w-0` - appear in fifty files each and would
/// name whichever the walker reached first. The point of a location is
/// that it is right, so a short literal buys nothing and costs trust.
const ENOUGH: usize = 16;

/// Every class literal the client spells, and where.
///
/// The empty index is a legitimate state and not a failure: an
/// instrument pointed at a page whose sources are not beside it can
/// still take every reading, and each of them simply names no line.
#[derive(Default)]
pub(crate) struct Sources {
    /// Longest literal first, so the first match is the best one.
    spelled: Vec<(String, String)>,
}

impl Sources {
    /// Read the client once. A survey opens the same page five times,
    /// and five walks of the source tree would be four too many.
    pub(crate) fn index(root: &Path) -> Result<Sources, XtaskError> {
        let mut found: BTreeMap<String, String> = BTreeMap::new();
        for path in walk::files_with_ext(&root.join(CLIENT_SRC), &DRAWN_IN)? {
            let rel = walk::rel(root, &path);
            let text = walk::read_text(&path)?;
            for (index, line) in text.lines().enumerate() {
                for literal in literals(line) {
                    if literal.chars().count() < ENOUGH || !literal.contains(' ') {
                        continue;
                    }
                    let at = index.saturating_add(1);
                    found
                        .entry(literal)
                        .or_insert_with(|| format!("{rel}:{at}"));
                }
            }
        }
        let mut spelled: Vec<(String, String)> = found.into_iter().collect();
        // Longest first, and by the literal itself where two are equal,
        // so that two runs of this instrument report the same line.
        spelled.sort_by(|left, right| {
            right
                .0
                .len()
                .cmp(&left.0.len())
                .then_with(|| left.0.cmp(&right.0))
        });
        Ok(Sources { spelled })
    }

    /// The line that drew a box, when one literal in the tree accounts
    /// for enough of what the box is wearing.
    pub(crate) fn locate(&self, class: &str) -> Option<&str> {
        if class.is_empty() {
            return None;
        }
        self.spelled
            .iter()
            .find(|(literal, _)| class.contains(literal.as_str()))
            .map(|(_, at)| at.as_str())
    }
}

/// Every double-quoted run on one line.
///
/// Only double quotes: a class attribute in this client is written
/// `class="…"` or `` `…${…}…` ``, and the backtick case is a template
/// whose static halves are what a painted attribute contains. Taking
/// the halves between the interpolations would need a parser; taking
/// the double-quoted runs covers the plain case, which is most of the
/// tree, and the template case degrades to no location rather than to
/// a wrong one.
fn literals(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(open) = rest.find('"') {
        let Some(tail) = rest.get(open.saturating_add(1)..) else {
            break;
        };
        let Some(close) = tail.find('"') else {
            break;
        };
        if let Some(inner) = tail.get(..close)
            && !inner.is_empty()
        {
            out.push(inner.to_owned());
        }
        let Some(next) = tail.get(close.saturating_add(1)..) else {
            break;
        };
        rest = next;
    }
    out
}
