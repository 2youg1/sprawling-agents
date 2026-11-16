// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::violation;
use crate::report::Violation;
use browser::survey::Drawn;

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
