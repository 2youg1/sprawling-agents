// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The slash menu: what Tab offers for the line being typed, and the one
//! the cursor is on (`crates/sprawling/spec/Console.lean` §8-11).
//!
//! Pure. Tab opens it on the first fit, in the order `wire::Slash`
//! lists the verbs, so `/w` Tab Enter is `/web`; Tab and Down move the
//! cursor on, Shift+Tab and Up move it back, and Enter takes the fit
//! under it. A fit that leaves a line ready to carry - a verb that takes
//! nothing, or an argument - is carried at once; a verb that takes an
//! argument is written out with its space for the argument to follow.

use console_ffi::scene::Choice;

/// How many entries the menu shows at once; the rest are counted.
const SHOWN: usize = 5;

/// What choosing a fit does with the line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Then {
    /// The line is complete: it is carried as Enter carries a line.
    Carry,
    /// The line waits for an argument.
    Wait,
}

/// One thing Tab can put on the line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Fit {
    /// The line it leaves.
    pub(crate) line: String,
    /// What the menu shows for it.
    pub(crate) spelling: String,
    pub(crate) takes: &'static str,
    pub(crate) summary: &'static str,
    pub(crate) then: Then,
}

/// The fits for a verb being typed: every slash verb the CLI offers
/// whose spelling begins with it.
pub(crate) fn verbs(typed: &str) -> Vec<Fit> {
    super::super::language::slash_verbs()
        .filter(|verb| verb.spelling().starts_with(typed))
        .map(|verb| {
            let (line, then) = if verb.takes().is_empty() {
                (verb.spelling().to_owned(), Then::Carry)
            } else {
                (format!("{} ", verb.spelling()), Then::Wait)
            };
            Fit {
                line,
                spelling: verb.spelling().to_owned(),
                takes: verb.takes(),
                summary: verb.says(),
                then,
            }
        })
        .collect()
}

/// The fits for an argument being typed after `verb`: every offered
/// argument that begins with it.
pub(crate) fn arguments(verb: &str, begun: &str, offered: &[String]) -> Vec<Fit> {
    offered
        .iter()
        .filter(|argument| argument.starts_with(begun))
        .map(|argument| Fit {
            line: format!("{verb} {argument}"),
            spelling: argument.clone(),
            takes: "",
            summary: "",
            then: Then::Carry,
        })
        .collect()
}

/// An open menu: its fits, and the one under the cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Menu {
    fits: Vec<Fit>,
    chosen: usize,
}

impl Menu {
    /// A menu on the first of `fits`, or none when nothing fits.
    pub(crate) fn of(fits: Vec<Fit>) -> Option<Menu> {
        (!fits.is_empty()).then_some(Menu { fits, chosen: 0 })
    }

    pub(crate) fn next(&mut self) {
        self.chosen = match self.chosen.checked_add(1) {
            Some(next) if next < self.fits.len() => next,
            Some(_) | None => 0,
        };
    }

    pub(crate) fn previous(&mut self) {
        self.chosen = self
            .chosen
            .checked_sub(1)
            .unwrap_or_else(|| self.fits.len().saturating_sub(1));
    }

    pub(crate) fn chosen(&self) -> Option<&Fit> {
        self.fits.get(self.chosen)
    }

    /// The entries drawn: at most [`SHOWN`], placed so the cursor's
    /// entry is among them, which of them the cursor is on, and how many
    /// are left out.
    pub(crate) fn window(&self) -> (Vec<Choice<'_>>, usize, usize) {
        let last_start = self.fits.len().saturating_sub(SHOWN);
        let start = self
            .chosen
            .saturating_sub(SHOWN.saturating_sub(1))
            .min(last_start);
        let shown: Vec<Choice<'_>> = self
            .fits
            .iter()
            .skip(start)
            .take(SHOWN)
            .map(|fit| Choice {
                spelling: &fit.spelling,
                takes: fit.takes,
                summary: fit.summary,
            })
            .collect();
        let more = self.fits.len().saturating_sub(shown.len());
        (shown, self.chosen.saturating_sub(start), more)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn w_then_tab_then_enter_is_web() {
        let menu = Menu::of(verbs("/w")).unwrap();
        let chosen = menu.chosen().unwrap();
        assert_eq!(
            (chosen.line.as_str(), chosen.then),
            (wire::Slash::Web.spelling(), Then::Carry)
        );
        assert_eq!(menu.window().0.len(), 2);
    }

    #[test]
    fn the_window_follows_the_cursor_and_counts_the_rest() {
        let mut menu = Menu::of(verbs("/")).unwrap();
        let total = verbs("/").len();
        for _ in 0..7 {
            menu.next();
        }
        let (shown, chosen, more) = menu.window();
        assert_eq!(shown[chosen].spelling, verbs("/")[7].spelling);
        assert_eq!((shown.len(), more), (SHOWN, total - SHOWN));
        menu.previous();
        for _ in 0..total {
            menu.next();
        }
        assert_eq!(menu.chosen().unwrap().spelling, verbs("/")[6].spelling);
    }
}
