// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The mark a key is not allowed to carry.
//!
//! **The other reading that used to live here has moved.** Where a
//! row's first painted box starts is geometry, and geometry is measured
//! once, by `xtask::survey`; this file kept the half that is not a
//! position at all - whether a decoration reached a key - and it stays
//! beside the gate because it is a property of this client's own
//! stylesheet rather than a measurement any page could be put through.
//!
//! **The underline is read as it is drawn at rest.** A decoration a
//! rule paints only while a pointer rests on a row is not in a dumped
//! document, so this measures the page nobody is touching. That is an
//! under-report and never a false one.

use super::violation;
use crate::report::Violation;
use browser::survey::Drawn;

/// No key is drawn with a line under it.
///
/// A key is a face, not a link. The stylesheet underlines a link under
/// the pointer, and the mark inside it went with it, so `g c` on the
/// rail read as something to click.
pub(super) fn no_key_is_underlined(drawn: &[Drawn], at: &str, out: &mut Vec<Violation>) {
    for held in drawn
        .iter()
        .filter(|held| held.tag == "KBD" && held.drawn() && held.underlined)
    {
        out.push(violation(
            at,
            "no key is drawn with a line under it",
            format!(
                "{} at x={} y={} carries an underline",
                held.called(),
                held.left,
                held.top
            ),
            "a decoration reaches every in-flow descendant and a `text-decoration: none` on the \
             key does not stop it: keep the key in its own atomic box, or do not decorate what \
             holds it",
        ));
    }
}
