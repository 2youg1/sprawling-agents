// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Text the layout made unreadable.
//!
//! **Only one of the two ways text is lost is here.** Text can be lost
//! to its container, which is arithmetic over two widths the engine
//! already computed, and it can be lost to the colour behind it, which
//! needs a contrast model. This repository has one, in `xtask::color`,
//! calibrated for pairs of declared tokens on the single hue axis
//! rather than for two colours off a screen; wiring the painted pair
//! into it is one visibility change in a file this change does not own,
//! and writing a second contrast formula here would be the second
//! authority the whole instrument exists to prevent. So this module
//! measures the first, and the second is a named debt rather than a
//! duplicate.
//!
//! **A cut a person can see is not a defect.** A box that draws an
//! ellipsis says that it is holding something back, and a person can
//! widen it or hover it; a box that simply clips gives no sign at all.
//! The distinction is carried in the record rather than decided here,
//! so that a run cannot quietly reclassify one as the other.

use super::{Cut, Deviation, Drawn, Finding, Overflow};

/// No box cuts its own text without saying so.
pub(super) fn no_box_cuts_its_own_text<'a>(drawn: &'a [Drawn], out: &mut Vec<Deviation<'a>>) {
    for held in drawn.iter().filter(|held| held.shows()) {
        let Some(text) = held.text.as_ref() else {
            continue;
        };
        // A box that scrolls across is holding more than it shows on
        // purpose, and the person has the means to reach the rest.
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
