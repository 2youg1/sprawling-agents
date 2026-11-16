// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::{Cut, Deviation, Drawn, Finding, Overflow};

pub(super) fn no_box_cuts_its_own_text<'a>(drawn: &'a [Drawn], out: &mut Vec<Deviation<'a>>) {
    for held in drawn.iter().filter(|held| held.shows()) {
        let Some(text) = held.text.as_ref() else {
            continue;
        };

        if held.across == Overflow::Scrolls {
            continue;
        }
        let Cut::Hidden { shown, needs } = text.cut else {
            continue;
        };
        out.push(Deviation {
            at: held,
            finding: Finding::TextCut { shown, needs },
        });
    }
}
