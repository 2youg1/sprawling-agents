// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The parts roster: every gallery region that draws a part carries the
//! roles that part's contract names (tools/xtask/Spec.lean §8-13,
//! client/spec/Views/Parts.lean §7-10).
//!
//! **A look that forgets to spread a wire bag drops the role with it.**
//! A part is a seat that owns the wiring and a look that draws it
//! (client/Spec.lean, seats and looks): the seat hands the look a bag of
//! `role`, `aria-*` and handlers, and the look spreads it onto the
//! element it draws. A look that draws its own `<button>` in place of the
//! bag's still renders, still lines up and still passes every other
//! reading of this gate, while a screen reader meets a row of unnamed
//! buttons where a radio group stood. The roles are what the engine
//! reports for the drawn page, so this is where the swap test bites.
//!
//! **A region is found by its name, never by a class.** `gallery/case.svelte`
//! draws every fixture as `<section aria-label={label}>`, and a label opens
//! with the part's name and a middle dot (`segmented · two cells`). The
//! name is the one the source gives the part, so a fixture moved between
//! files keeps its place on the roster.
//!
//! **A part with no fixture on the gallery is judged when one arrives.**
//! The roster names a part's contract, not a count of its fixtures: the
//! line that adds the first `tabs ·` case is judged by this table without
//! touching it.
//!
//! **A case named for the empty state owes nothing.** `table · empty`
//! draws the empty-state part in the table's place, so no header is on
//! the page; a label whose state opens with the word `empty` is read as
//! that case for every part on the roster.

use super::violation;
use crate::report::Violation;
use browser::survey::Drawn;

/// The first word of the state a case draws when the part has nothing
/// to show (`table · empty`).
const EMPTY: &str = "empty";

/// What a region must hold for one part, and where the contract says so.
pub(super) struct Entry {
    /// The first word of every fixture label for this part.
    part: &'static str,
    /// The section of `client/spec/Views/Parts.lean` this row reads.
    contract: &'static str,
    needs: &'static [Need],
}

/// One role a region must draw, how many times, and whether each must be
/// named.
struct Need {
    shown: Shown,
    least: usize,
    naming: Naming,
}

/// How a required element is recognised on the drawn page.
#[derive(Clone, Copy)]
enum Shown {
    /// An explicit `role` attribute, which is what the probe records.
    Role(&'static str),
    /// One of several roles, when the contract lets the caller pick.
    AnyRole(&'static [&'static str]),
    /// A native element whose role the platform gives without an
    /// attribute.
    Tag(&'static str),
}

/// Whether the element must carry an accessible name.
#[derive(Clone, Copy)]
enum Naming {
    Named,
    /// A panel is named by the tab that controls it, through
    /// `aria-labelledby`, which the drawn page may resolve to the tab's
    /// text or to nothing while the panel is empty.
    Any,
}

const fn named(shown: Shown, least: usize) -> Need {
    Need {
        shown,
        least,
        naming: Naming::Named,
    }
}

/// The roster. Four parts are absent, each because the page this gate
/// reads cannot show what its contract names. `row`: `Row` carries no
/// role of its own (Parts.lean §7-1) and `RowList` owes keys rather than
/// roles (§7-4), so nothing a drawn page reports tells a row whose look
/// dropped its bag from one that kept it. `dialog`: a fixture that draws
/// the trigger with the sheet closed has no `<dialog>` on the drawn page.
/// `tip`: the tooltip is `display: none` until hover or focus (§7-3), and
/// the probe records only what the engine draws. `combobox`: the
/// `combobox` role is on the filter box of the open popup (§7-5), and a
/// popup drawn open on load closes as soon as another fixture takes the
/// focus, which leaves the trigger, a plain button.
pub(super) const ROSTER: &[Entry] = &[
    Entry {
        part: "segmented",
        contract: "§7-4",
        needs: &[
            named(Shown::Role("radiogroup"), 1),
            named(Shown::Role("radio"), 2),
        ],
    },
    Entry {
        part: "tabs",
        contract: "§7-4",
        needs: &[
            named(Shown::Role("tablist"), 1),
            named(Shown::Role("tab"), 2),
            Need {
                shown: Shown::Role("tabpanel"),
                least: 1,
                naming: Naming::Any,
            },
        ],
    },
    Entry {
        part: "popover",
        contract: "§7-5",
        needs: &[
            named(Shown::Role("dialog"), 1),
            named(Shown::Role("listbox"), 1),
            named(Shown::Role("option"), 1),
        ],
    },
    Entry {
        part: "progress",
        contract: "§7-1",
        needs: &[named(Shown::Role("progressbar"), 1)],
    },
    Entry {
        part: "skeleton",
        contract: "§7-1",
        needs: &[named(Shown::Role("status"), 1)],
    },
    Entry {
        part: "notice",
        contract: "§7-1",
        needs: &[named(Shown::AnyRole(&["status", "alert"]), 1)],
    },
    Entry {
        part: "table",
        contract: "§7-6",
        needs: &[named(Shown::Tag("TH"), 1)],
    },
];

/// Every region named for a part on the roster draws what that part's
/// contract needs.
pub(super) fn every_part_draws_its_roles(
    roster: &[Entry],
    drawn: &[Drawn],
    at: &str,
    out: &mut Vec<Violation>,
) {
    for (index, region) in drawn.iter().enumerate() {
        let Some(entry) = roster.iter().find(|entry| names(region, entry.part)) else {
            continue;
        };
        for need in entry.needs {
            let found = drawn
                .iter()
                .enumerate()
                .filter(|(inner, held)| within(drawn, *inner, index) && need.met_by(held))
                .count();
            if found >= need.least {
                continue;
            }
            out.push(violation(
                &format!("{at} \u{b7} {}", region.name),
                &format!(
                    "every part on the gallery draws the roles its contract names \
                     (client/spec/Views/Parts.lean {})",
                    entry.contract
                ),
                format!(
                    "the {} region draws {found} {}, and its contract needs at least {}",
                    entry.part,
                    need.called(),
                    need.least
                ),
                "spread every wire bag the seat hands its look onto the element the bag \
                 names (`<button {...cell.wire}>`): a look that draws its own element in \
                 place of the bag's drops the role, the name and the keys with it",
            ));
        }
    }
}

/// Whether `region` is a gallery case for `part` that owes the part's
/// roles: every case but the one drawing the empty state.
fn names(region: &Drawn, part: &str) -> bool {
    region.tag == "SECTION"
        && region
            .name
            .strip_prefix(part)
            .and_then(|rest| rest.strip_prefix(" \u{b7}"))
            .is_some_and(|state| !draws_the_empty_state(state))
}

/// Whether a case's state, the label after the middle dot, opens with
/// the word that names the empty state.
fn draws_the_empty_state(state: &str) -> bool {
    state
        .split(|c: char| !c.is_alphanumeric())
        .find(|word| !word.is_empty())
        == Some(EMPTY)
}

/// Whether the element at `inner` sits inside the element at `outer`,
/// read up the chain of nearest measured ancestors. The probe records an
/// ancestor before its descendants, so every step up lowers the index and
/// the walk ends.
fn within(drawn: &[Drawn], inner: usize, outer: usize) -> bool {
    let mut at = inner;
    while let Some(up) = drawn
        .get(at)
        .and_then(|held| usize::try_from(held.parent).ok())
        .filter(|up| *up < at)
    {
        if up == outer {
            return true;
        }
        at = up;
    }
    false
}

impl Need {
    fn met_by(&self, held: &Drawn) -> bool {
        let shown = match self.shown {
            Shown::Role(role) => held.role == role,
            Shown::AnyRole(roles) => roles.contains(&held.role.as_str()),
            Shown::Tag(tag) => held.tag == tag,
        };
        shown
            && match self.naming {
                Naming::Named => !held.anonymous(),
                Naming::Any => true,
            }
    }

    fn called(&self) -> String {
        let what = match self.shown {
            Shown::Role(role) => format!("`{role}`"),
            Shown::AnyRole(roles) => roles
                .iter()
                .map(|role| format!("`{role}`"))
                .collect::<Vec<_>>()
                .join(" or "),
            Shown::Tag(tag) => format!("<{}>", tag.to_lowercase()),
        };
        match self.naming {
            Naming::Named => format!("named {what}"),
            Naming::Any => what,
        }
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing, reason = "test code")]
mod tests {
    use super::*;
    use browser::survey::{Overflow, Sampled};

    fn el(tag: &str, role: &str, name: &str, parent: i64) -> Drawn {
        Drawn {
            across: Overflow::Shows,
            down: Overflow::Shows,
            sampled: Sampled::Frame,
            first_mark: -1,
            underlined: false,
            fill: None,
            class: String::new(),
            text: None,
            tag: tag.to_owned(),
            role: role.to_owned(),
            name: name.to_owned(),
            left: 0,
            top: 0,
            width: 10,
            height: 10,
            depth: 0,
            parent,
        }
    }

    fn judged(drawn: &[Drawn]) -> Vec<String> {
        let mut out = Vec::new();
        every_part_draws_its_roles(ROSTER, drawn, "at", &mut out);
        out.into_iter()
            .map(|found| format!("{} :: {}", found.location, found.violation))
            .collect()
    }

    /// The look that drew its own buttons in place of the bags: the
    /// region still renders two named controls, and the roster names
    /// both roles it lost.
    #[test]
    fn a_segmented_look_that_dropped_its_bags_is_refused() {
        let drawn = [
            el("SECTION", "-", "segmented · two cells", -1),
            el("DIV", "-", "", 0),
            el("BUTTON", "-", "Light", 1),
            el("BUTTON", "-", "Dark", 1),
        ];
        assert_eq!(
            judged(&drawn),
            [
                "at · segmented · two cells :: the segmented region draws 0 named \
                 `radiogroup`, and its contract needs at least 1",
                "at · segmented · two cells :: the segmented region draws 0 named `radio`, \
                 and its contract needs at least 2",
            ]
        );
    }

    #[test]
    fn a_segmented_look_that_spread_its_bags_passes() {
        let drawn = [
            el("SECTION", "-", "segmented · two cells", -1),
            el("DIV", "radiogroup", "Theme", 0),
            el("BUTTON", "radio", "Light", 1),
            el("BUTTON", "radio", "Dark", 1),
        ];
        assert!(judged(&drawn).is_empty());
    }

    /// The roles are counted inside the region alone: a radio group in
    /// the next case does not stand in for one this case lost, and an
    /// unnamed cell does not count as a named one.
    #[test]
    fn roles_count_inside_their_own_region_and_only_when_named() {
        let drawn = [
            el("SECTION", "-", "segmented · one cell unnamed", -1),
            el("DIV", "radiogroup", "Theme", 0),
            el("BUTTON", "radio", "-", 1),
            el("BUTTON", "radio", "Dark", 1),
            el("SECTION", "-", "segmented · full", -1),
            el("DIV", "radiogroup", "Theme", 4),
            el("BUTTON", "radio", "Light", 5),
            el("BUTTON", "radio", "Dark", 5),
        ];
        assert_eq!(
            judged(&drawn),
            [
                "at · segmented · one cell unnamed :: the segmented region draws 1 named \
                 `radio`, and its contract needs at least 2"
            ]
        );
    }

    /// The empty table draws the empty-state part and no header; a case
    /// whose state only mentions emptiness further on is still judged.
    #[test]
    fn a_case_drawing_the_empty_state_owes_no_roles() {
        let drawn = [
            el("SECTION", "-", "table · empty", -1),
            el("BUTTON", "-", "Fetch the model list", 0),
            el("SECTION", "-", "table · rows, then an empty filter", -1),
            el("BUTTON", "-", "Sort", 2),
        ];
        assert_eq!(
            judged(&drawn),
            [
                "at · table · rows, then an empty filter :: the table region draws 0 named <th>, \
              and its contract needs at least 1"
            ]
        );
    }

    /// A label that only starts with a part's letters is another part:
    /// `notices · toasts` is not a `notice ·` case.
    #[test]
    fn a_region_belongs_to_a_part_by_its_whole_first_word() {
        let drawn = [
            el("SECTION", "-", "notices · toasts", -1),
            el("SECTION", "-", "notice · inline", -1),
            el("DIV", "alert", "Key missing", 1),
        ];
        assert!(judged(&drawn).is_empty());
    }
}
