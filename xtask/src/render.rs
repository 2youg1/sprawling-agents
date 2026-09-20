// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Render gate: the client's own gallery is opened in a real engine, and
//! what it drew is measured (xtask-SPEC.md sections 8-13 and 8-14).
//!
//! **This is the step the method never had.** A stylesheet's rules do not
//! collide in either source file — they collide in the cascade. Two rules
//! that each read correctly where they are written laid the composer out
//! as a row and put a second left edge on every page, and the tree was
//! green through all of it: fourteen gates, 1,338 tests, and a home page
//! whose task box had floated into the top right corner.
//!
//! **The accessibility half used to be its own gate and is now these
//! three properties.** `ax` compared the affordances both sides *wrote
//! down*, and said in its own header why: a computed tree needs a
//! browser, and a gate that cannot run offline stops running. That
//! trade expired when the gate learnt to open a browser. Roles,
//! accessible names and landmarks are read off the page as drawn, which
//! is what a person using a screen reader actually meets.
//!
//! **What it asserts is a property, never a picture.** A screenshot
//! comparison fails on a font hint and passes on a page that is wrong in
//! a way nobody photographed.
//!
//! **The page is opened once per pass, and a pass is a width and a
//! lighting.** A property that holds at one width is a property about
//! that width; the three columns are laid out by container queries, so
//! the same source draws a different page at 768 and at 2560. Which
//! passes a run makes lives in `pass.rs` (section 8-16).
//!
//! **A missing bundle, a missing browser or a gallery that drew
//! nothing is a violation, and each says which.** A gate that goes
//! quiet when it cannot find its subject reports green for a page
//! nobody looked at, which is how a broken instrument reads as a
//! passing product. The first two name the command that supplies what
//! is missing — `just build-web`, or a Chromium-family browser at
//! `SPRAWLING_BROWSER` — and the third names the gallery itself as the
//! defect, so the three are never collapsed into one message.

use std::collections::BTreeMap;
use std::path::Path;

use crate::report::{Violation, XtaskError};

mod announced;
mod engine;
mod marks;
mod pass;
mod probe;

use announced::{every_control_is_announceable, every_landmark_is_named, one_first_heading};
use engine::{Measured, Opening, browser, measure};
use marks::{no_key_is_underlined, rows_share_a_first_mark};
use pass::Pass;

/// The client bundle this gate opens: the same one `just dist` embeds.
const BUNDLE: &str = "target/web-dist";

/// The route that draws every state worth looking at, on fixtures.
const GALLERY: &str = "#/gallery";

/// Where a finding is located when no pass could be made at all.
const EVERY_PASS: &str = "in every pass";

/// Two boxes may differ by this many pixels and still count as aligned.
///
/// Sub-pixel layout rounds, and a border that a rule paints on one side
/// only moves a box by one. Anything larger is a decision somebody made
/// twice.
const SLACK: i64 = 1;

/// One element as the engine drew it.
pub(super) struct Drawn {
    pub(super) tag: String,
    pub(super) role: String,
    pub(super) name: String,
    pub(super) left: i64,
    pub(super) top: i64,
    pub(super) width: i64,
    pub(super) height: i64,
    pub(super) depth: i64,
    /// Index of the nearest measured ancestor, or `-1` at the top.
    pub(super) parent: i64,
    /// Whether this element's own overflow lets its contents extend past
    /// its box, across and down.
    ///
    /// A scrolling container is the one legitimate way a child is drawn
    /// outside its parent: the page is longer than the window and the
    /// person scrolls. Without this the rule below reads every page with
    /// more content than one screen as a layout defect, which is every
    /// page this product has.
    pub(super) scrolls_across: bool,
    pub(super) scrolls_down: bool,
    /// The centre x of the first painted element inside this one, or
    /// `-1` when it holds none. What a person's eye lands on first in a
    /// row: the dot, the glyph, the tick.
    pub(super) first_mark: i64,
    /// Whether a line is drawn under this element's text, by its own
    /// rule or by one it inherits from a box that holds it.
    pub(super) underlined: bool,
}

impl Drawn {
    fn right(&self) -> i64 {
        self.left.saturating_add(self.width)
    }

    fn bottom(&self) -> i64 {
        self.top.saturating_add(self.height)
    }

    /// What a violation calls this element.
    fn called(&self) -> String {
        let tag = self.tag.to_lowercase();
        if self.name.is_empty() || self.name == "-" {
            return tag;
        }
        format!("{tag} `{}`", self.name)
    }

    /// An element with no area holds nothing on this page, and has no
    /// position worth comparing.
    fn drawn(&self) -> bool {
        self.width > 0 && self.height > 0
    }

    /// Controls a person operates, which are the ones that must be
    /// announceable.
    ///
    /// By role as well as by tag: this client draws a building as a `<g>`
    /// with `role="link"`, and an element that says it is a control is
    /// one, whatever it is made of.
    fn operable(&self) -> bool {
        matches!(
            self.tag.as_str(),
            "BUTTON" | "A" | "INPUT" | "TEXTAREA" | "SELECT"
        ) || matches!(
            self.role.as_str(),
            "button" | "link" | "textbox" | "checkbox" | "radio" | "tab" | "menuitem" | "switch"
        )
    }

    /// The regions a screen reader offers as a way to jump.
    fn landmark(&self) -> bool {
        matches!(
            self.tag.as_str(),
            "MAIN" | "NAV" | "ASIDE" | "HEADER" | "FOOTER"
        ) || matches!(
            self.role.as_str(),
            "navigation" | "main" | "complementary" | "banner" | "contentinfo" | "dialog"
        )
    }

    fn anonymous(&self) -> bool {
        self.name.is_empty() || self.name == "-"
    }
}

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let bundle = root.join(BUNDLE);
    if !bundle.join("index.html").is_file() {
        return Ok(vec![violation(
            EVERY_PASS,
            "the client bundle this gate measures is built",
            format!("{BUNDLE}/index.html is not built, so no page was measured"),
            "run `just build-web`",
        )]);
    }
    let Some(browser) = browser() else {
        return Ok(vec![violation(
            EVERY_PASS,
            "a real engine draws the page this gate measures",
            "no headless browser was found, so no page was measured".to_owned(),
            "install a Chromium-family browser, or point `SPRAWLING_BROWSER` at one",
        )]);
    };
    let opening = Opening {
        root,
        browser: &browser,
        bundle: &bundle,
        route: GALLERY,
    };
    let mut violations = Vec::new();
    for pass in pass::wanted()? {
        judge(pass, &measure(&opening, pass)?, &mut violations);
    }
    Ok(violations)
}

/// Every property, over one opening of the page.
///
/// The pass reports itself first: a lighting the page did not take is a
/// pass that measured some other page, and reading seven properties off
/// it would report green for a page nobody looked at.
fn judge(pass: &Pass, measured: &Measured, out: &mut Vec<Violation>) {
    let at = pass.called();
    if measured.drawn.is_empty() {
        out.push(violation(
            &at,
            "the gallery draws every state this gate measures",
            format!("{GALLERY} drew nothing measurable"),
            "repair the gallery route so it renders its fixtures",
        ));
        return;
    }
    if let Some(difference) = pass.disagrees(&measured.reported) {
        out.push(violation(
            &at,
            "the page draws itself in the conditions the pass asked for",
            difference,
            "the lighting is the `data-theme` attribute `theme.css` selects on, and the forced \
             mode is the engine's own switch: a pass that cannot set one measures a page it did \
             not ask for",
        ));
        return;
    }
    let drawn = &measured.drawn;
    every_control_is_announceable(drawn, &at, out);
    every_landmark_is_named(drawn, &at, out);
    one_first_heading(drawn, &at, out);
    one_left_edge(drawn, &at, out);
    nothing_escapes_what_holds_it(drawn, &at, out);
    rows_share_a_first_mark(drawn, &at, out);
    no_key_is_underlined(drawn, &at, out);
}

/// One finding, told where it was found: the same page at two widths is
/// two places a reader has to be sent to.
pub(super) fn violation(at: &str, rule: &str, subject: String, alternative: &str) -> Violation {
    Violation {
        gate: "render",
        location: format!("{BUNDLE} {GALLERY} {at}"),
        rule: rule.to_owned(),
        violation: subject,
        alternative: alternative.to_owned(),
    }
}

/// Every region in the main column starts at the same x.
fn one_left_edge(drawn: &[Drawn], at: &str, out: &mut Vec<Violation>) {
    let Some(main) = drawn.iter().position(|held| held.tag == "MAIN") else {
        return;
    };
    let Ok(main_index) = i64::try_from(main) else {
        return;
    };
    let mut edges: BTreeMap<i64, String> = BTreeMap::new();
    for region in drawn
        .iter()
        .filter(|held| held.tag == "SECTION" && held.parent == main_index && held.drawn())
    {
        edges.entry(region.left).or_insert_with(|| region.called());
    }
    if edges.len() < 2 {
        return;
    }
    if let (Some(first), Some(last)) = (edges.keys().next(), edges.keys().next_back())
        && last.saturating_sub(*first) <= SLACK
    {
        return;
    }
    let listed: Vec<String> = edges
        .iter()
        .map(|(left, name)| format!("{name} at x={left}"))
        .collect();
    out.push(violation(
        at,
        "a page has one left edge: every region in the main column starts at the same x",
        format!("this page has {}: {}", edges.len(), listed.join(", ")),
        "let one authority set the inline margins - a region states its vertical rhythm, the \
         column states the spine",
    ));
}

/// Nothing is drawn outside the box that holds it.
///
/// This is the generalisation of the defect the gate was written for: a
/// box that has floated out of its container is still painted, still
/// passes every test, and is the one thing a person sees immediately.
fn nothing_escapes_what_holds_it(drawn: &[Drawn], at: &str, out: &mut Vec<Violation>) {
    for held in drawn.iter().filter(|held| held.drawn()) {
        let Some(parent) = parent_of(drawn, held) else {
            continue;
        };
        if !parent.drawn() || parent.depth >= held.depth {
            continue;
        }
        // A container that scrolls is allowed to hold more than it
        // shows, in the axis it scrolls: what is below the fold is not
        // painted over anything.
        let escapes = (!parent.scrolls_across
            && (held.left.saturating_add(SLACK) < parent.left
                || held.right() > parent.right().saturating_add(SLACK)))
            || (!parent.scrolls_down
                && (held.top.saturating_add(SLACK) < parent.top
                    || held.bottom() > parent.bottom().saturating_add(SLACK)));
        if escapes {
            out.push(violation(
                at,
                "nothing is drawn outside the box that holds it",
                format!(
                    "{} ({},{} {}x{}) leaves {} ({},{} {}x{})",
                    held.called(),
                    held.left,
                    held.top,
                    held.width,
                    held.height,
                    parent.called(),
                    parent.left,
                    parent.top,
                    parent.width,
                    parent.height
                ),
                "size the container to its contents, or let the contents scroll inside it. A box \
                 that overflows is painted over whatever is next to it",
            ));
        }
    }
}

fn parent_of<'a>(drawn: &'a [Drawn], held: &Drawn) -> Option<&'a Drawn> {
    usize::try_from(held.parent)
        .ok()
        .and_then(|at| drawn.get(at))
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
