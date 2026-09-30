// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The three properties a screen reader meets: every control says what
//! it is, every landmark says which region it is, and the page has one
//! first heading (xtask-SPEC.md section 8-13).
//!
//! They were the gate called `ax` until a browser could be opened, and
//! they are the three that read the page's names rather than its
//! geometry, which is why they sit together.

use super::violation;
use crate::report::Violation;
use browser::survey::Drawn;

/// Every control a person can operate says what it is.
///
/// The first run of this property found four: the composer's own text
/// box, on every fixture that draws one. It is the control the whole page
/// exists for, and to a screen reader it was an unlabelled edit field.
pub(super) fn every_control_is_announceable(drawn: &[Drawn], at: &str, out: &mut Vec<Violation>) {
    for held in drawn
        .iter()
        .filter(|held| held.operable() && held.drawn() && held.anonymous())
    {
        let what = if held.role == "-" {
            held.tag.to_lowercase()
        } else {
            format!("{} as {}", held.tag.to_lowercase(), held.role)
        };
        out.push(violation(
            at,
            "every control a person can operate has an accessible name",
            format!(
                "a {what} at x={} y={} is announced as nothing",
                held.left, held.top
            ),
            "give it an `aria-label` from the phrase table, or let it contain the words it \
             already shows. A control with no name is a control a screen reader can only \
             call `button`",
        ));
    }
}

pub(super) fn every_landmark_is_named(drawn: &[Drawn], at: &str, out: &mut Vec<Violation>) {
    for held in drawn
        .iter()
        .filter(|held| held.landmark() && held.drawn() && held.anonymous())
    {
        out.push(violation(
            at,
            "every landmark says which region it is",
            format!("a <{}> offers no name to jump to", held.tag.to_lowercase()),
            "give the landmark an `aria-label` from the phrase table: two unnamed regions are \
             two entries that read the same in the jump list",
        ));
    }
}

/// One first heading, so a reader arriving by keyboard lands somewhere.
pub(super) fn one_first_heading(drawn: &[Drawn], at: &str, out: &mut Vec<Violation>) {
    let headings: Vec<&Drawn> = drawn
        .iter()
        .filter(|held| held.tag == "H1" && held.drawn())
        .collect();
    if headings.len() == 1 {
        return;
    }
    let named: Vec<String> = headings.iter().map(|held| held.called()).collect();
    out.push(violation(
        at,
        "a page has exactly one first heading",
        format!("this page has {}: {}", headings.len(), named.join(", ")),
        "one <h1> names the page; the parts under it are <h2>. A page with none gives a reader \
         nothing to land on, and a page with two disagrees with itself about what it is",
    ));
}
