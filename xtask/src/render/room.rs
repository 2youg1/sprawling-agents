// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Whether words had room to stand and a popover room to open
//! (xtask-SPEC.md section 8-38).
//!
//! **Both are defects every other reading passes.** A notice squeezed
//! to one glyph a line has not overflowed and has not left its
//! container, and a list cut to a sliver by the box that holds it is
//! still inside that box: containment and cut text are green on both,
//! and a person reads neither.

use browser::survey::{Drawn, Overflow};

use super::violation;
use crate::report::Violation;

/// How many glyphs wide a sentence must be, and how many glyphs tall a
/// box must reach before it counts as folded onto more than one line.
const CRUSHED_EMS: i64 = 2;

/// No sentence is set narrower than two glyphs and folded onto lines.
///
/// Words that carry their own line breaks were folded by their author,
/// not by the engine: a code gutter is one number a line by design.
///
/// A name of one glyph is left alone: a close button is one glyph in a
/// box its padding makes taller than wide, and that is its shape.
pub(super) fn no_text_is_crushed(drawn: &[Drawn], at: &str, out: &mut Vec<Violation>) {
    for held in drawn.iter().filter(|held| held.shows()) {
        let Some(run) = &held.text else { continue };
        let glyph = i64::from(run.px_x100).checked_div(100).unwrap_or(0);
        let room = glyph.saturating_mul(CRUSHED_EMS);
        if glyph == 0
            || held.name.chars().nth(1).is_none()
            || held.name.contains('\n')
            || held.width >= room
        {
            continue;
        }
        if held.height < room {
            continue;
        }
        out.push(violation(
            at,
            "no sentence is set narrower than two glyphs and folded onto lines",
            format!(
                "{} at x={} y={} is {} px wide at a {glyph} px glyph, and {} px tall: at least \
                 {} lines",
                held.called(),
                held.left,
                held.top,
                held.width,
                held.height,
                held.height.checked_div(glyph).unwrap_or(0)
            ),
            "give the text a minimum width, or let the row it sits in wrap its buttons below it \
             instead of taking the width from the words",
        ));
    }
}

/// Every popover shows at least its first option.
///
/// Visible height is the popover's box cut by every measured ancestor
/// that does not let its contents show downward; a list with no option
/// measured is not judged, because there is nothing to compare it with.
pub(super) fn every_popover_shows_an_option(drawn: &[Drawn], at: &str, out: &mut Vec<Violation>) {
    for (index, popover) in drawn
        .iter()
        .enumerate()
        .filter(|(_, held)| matches!(held.role.as_str(), "listbox" | "menu"))
    {
        let Some(option) = drawn.iter().find(|held| {
            matches!(held.role.as_str(), "option" | "menuitem")
                && held.height > 0
                && holds(drawn, index, held)
        }) else {
            continue;
        };
        let visible = visible_height(drawn, popover);
        if visible >= option.height {
            continue;
        }
        out.push(violation(
            at,
            "every popover shows at least its first option",
            format!(
                "{} at x={} y={} shows {visible} px of a {} px option",
                popover.called(),
                popover.left,
                popover.top,
                option.height
            ),
            "let the box that holds the popover show it (no clipping overflow on the way up), or \
             place the popover outside that box",
        ));
    }
}

/// Whether the element at `ancestor` holds `held`, by the measured
/// parent chain.
fn holds(drawn: &[Drawn], ancestor: usize, held: &Drawn) -> bool {
    let mut up = usize::try_from(held.parent).ok();
    while let Some(at) = up {
        if at == ancestor {
            return true;
        }
        up = drawn
            .get(at)
            .and_then(|next| usize::try_from(next.parent).ok());
    }
    false
}

/// The height of `popover` left after every ancestor cut it.
///
/// A clipping holder cuts by position. A scrolling holder cuts only by
/// its own height, because a person can scroll the rest into view: a
/// list scrolled away above the fold is reachable, a list taller than
/// its window is not wholly.
fn visible_height(drawn: &[Drawn], popover: &Drawn) -> i64 {
    let (mut top, mut bottom) = (popover.top, popover.top.saturating_add(popover.height));
    let mut window = popover.height;
    let mut up = usize::try_from(popover.parent).ok();
    while let Some(holder) = up.and_then(|at| drawn.get(at)) {
        match holder.down {
            Overflow::Shows => {}
            Overflow::Clips => {
                top = top.max(holder.top);
                bottom = bottom.min(holder.top.saturating_add(holder.height));
            }
            Overflow::Scrolls => window = window.min(holder.height),
        }
        up = usize::try_from(holder.parent).ok();
    }
    bottom.saturating_sub(top).max(0).min(window)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests;
