// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Render gate: a settled screen is opened in a real engine and measured
//! (v0.0.3 card V3.48).
//!
//! **This is the step the four-step method never had.** `ax` compares the
//! affordances both sides *wrote down*, and it says so itself: it is not a
//! computed tree, and looking at pixels is a person's job. That left one
//! whole class of defect with nothing watching it, because a stylesheet's
//! rules do not collide in either source file — they collide in the
//! cascade. Two rules that each read correctly where they are written laid
//! the composer out as a row and put a second left edge on every page, and
//! the tree was green through all of it: fourteen gates, 1,338 tests, and
//! a home page whose task box had floated into the top right corner.
//!
//! **What it asserts is a property, never a picture.** A screenshot
//! comparison fails on a font hint and passes on a page that is wrong in a
//! way nobody photographed. These three are the shape of a page rather
//! than its appearance, and each one is the generalisation of a defect
//! that shipped:
//!
//! 1. A page has one left edge. Every region in the centre column starts
//!    at the same x, so a heading and the panel under it cannot disagree.
//! 2. A panel's head is the top of its own panel, and starts at its left
//!    edge. A panel laid out as a row puts its title beside its body
//!    instead, which is exactly what a `form` did to the composer.
//! 3. Nothing is wider than the region that holds it.
//!
//! **A missing browser is a skip, not a red.** The engine is not in this
//! repository and cannot be: the gate says so on the line it prints and
//! judges nothing, because a gate that fails where it cannot look is a
//! gate somebody disables. `SPRAWLING_BROWSER` names one explicitly.

use std::collections::BTreeMap;
use std::path::Path;

use crate::report::{Violation, XtaskError};
use crate::walk;

mod engine;

use engine::{Engine, browser};

/// Where the settled screens live.
const SCREENS: &str = "crates/web/screens";

/// The design tokens `web::theme`'s own test writes out for the screens to
/// link. Without it a screen renders at browser defaults, which would make
/// every measurement below a measurement of nothing.
const TOKENS: &str = "target/screens/tokens.css";

/// Two boxes may differ by this many pixels and still count as aligned.
///
/// Sub-pixel layout rounds, and a border that a rule paints on one side
/// only moves a box by one. Anything larger is a decision somebody made
/// twice.
const SLACK: i64 = 1;

/// One measured box, in the page's own coordinates.
struct Box {
    kind: String,
    tag: String,
    class: String,
    left: i64,
    top: i64,
    width: i64,
    height: i64,
}

impl Box {
    /// The x this box ends at.
    fn right(&self) -> i64 {
        self.left.saturating_add(self.width)
    }

    /// What a violation calls this box.
    fn name(&self) -> String {
        format!("{}.{}", self.tag.to_lowercase(), self.class)
    }

    /// A box with no area is a container that holds nothing on this
    /// screen, and it has no position worth comparing.
    fn drawn(&self) -> bool {
        self.width > 0 && self.height > 0
    }
}

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let screens = root.join(SCREENS);
    if !screens.is_dir() {
        return Ok(Vec::new());
    }
    if !root.join(TOKENS).is_file() {
        println!("gate render: {TOKENS} is not written; run `cargo test -p web` first (skipped)");
        return Ok(Vec::new());
    }
    let Some(browser) = browser() else {
        println!("gate render: no headless browser found; set SPRAWLING_BROWSER to one (skipped)");
        return Ok(Vec::new());
    };
    let engine = Engine::new(root, browser)?;
    let mut violations = Vec::new();
    for path in walk::files_with_ext(&screens, &["html"])? {
        let rel = walk::rel(root, &path);
        let boxes = engine.measure(&path, &rel)?;
        judge(&rel, &boxes, &mut violations);
    }
    Ok(violations)
}

/// The three properties, in the order a reader meets them on the page.
fn judge(rel: &str, boxes: &[Box], out: &mut Vec<Violation>) {
    let centre = boxes.iter().find(|held| held.kind == "centre");
    one_left_edge(rel, boxes, out);
    heads_lead_their_panels(rel, boxes, out);
    if let Some(centre) = centre {
        nothing_overflows(rel, centre, boxes, out);
    }
}

/// Every region in the centre column starts at the same x.
fn one_left_edge(rel: &str, boxes: &[Box], out: &mut Vec<Violation>) {
    let mut edges: BTreeMap<i64, String> = BTreeMap::new();
    for region in boxes
        .iter()
        .filter(|held| held.kind == "region" && held.drawn())
    {
        edges.entry(region.left).or_insert_with(|| region.name());
    }
    if edges.len() < 2 {
        return;
    }
    let listed: Vec<String> = edges
        .iter()
        .map(|(left, name)| format!("{name} at x={left}"))
        .collect();
    if let (Some(first), Some(last)) = (edges.keys().next(), edges.keys().next_back())
        && last.saturating_sub(*first) <= SLACK
    {
        return;
    }
    out.push(Violation {
        gate: "render",
        location: rel.to_owned(),
        rule: "a page has one left edge: every region in the centre column starts at the same x"
            .to_owned(),
        violation: format!("this page has {}: {}", edges.len(), listed.join(", ")),
        alternative: "let one authority set the inline margins - a panel states its vertical \
                      rhythm, the region states the spine"
            .to_owned(),
    });
}

/// A panel's head is the topmost of its parts and starts at its left edge.
fn heads_lead_their_panels(rel: &str, boxes: &[Box], out: &mut Vec<Violation>) {
    let mut panel: Option<&Box> = None;
    let mut parts: Vec<&Box> = Vec::new();
    for held in boxes {
        match held.kind.as_str() {
            "panel" => {
                if let Some(open) = panel.take() {
                    head_leads(rel, open, &parts, out);
                }
                parts.clear();
                panel = Some(held);
            }
            "part" => parts.push(held),
            _ => {}
        }
    }
    if let Some(open) = panel {
        head_leads(rel, open, &parts, out);
    }
}

/// One panel's own judgement.
fn head_leads(rel: &str, panel: &Box, parts: &[&Box], out: &mut Vec<Violation>) {
    let drawn: Vec<&&Box> = parts.iter().filter(|held| held.drawn()).collect();
    let Some(head) = drawn
        .iter()
        .find(|held| held.class.split('.').any(|word| word == "panel-head"))
    else {
        return;
    };
    if let Some(above) = drawn
        .iter()
        .find(|held| head.top.saturating_sub(held.top) > SLACK)
    {
        out.push(Violation {
            gate: "render",
            location: rel.to_owned(),
            rule: "a panel's head is the top of its own panel".to_owned(),
            violation: format!(
                "{} sits at y={} while {} is at y={} inside {}",
                head.name(),
                head.top,
                above.name(),
                above.top,
                panel.name()
            ),
            alternative: "the panel grammar stacks: state the panel's own display so an \
                          element rule cannot lay its parts out in a row"
                .to_owned(),
        });
    }
    if head.left.saturating_sub(panel.left).abs() > SLACK {
        out.push(Violation {
            gate: "render",
            location: rel.to_owned(),
            rule: "a panel's head starts at its panel's left edge".to_owned(),
            violation: format!(
                "{} starts at x={} inside {} at x={}",
                head.name(),
                head.left,
                panel.name(),
                panel.left
            ),
            alternative: "give the head no inline offset of its own".to_owned(),
        });
    }
}

/// Nothing measured reaches past the region that holds it.
fn nothing_overflows(rel: &str, centre: &Box, boxes: &[Box], out: &mut Vec<Violation>) {
    for held in boxes
        .iter()
        .filter(|held| held.kind != "centre" && held.drawn())
    {
        if held.right() > centre.right().saturating_add(SLACK) {
            out.push(Violation {
                gate: "render",
                location: rel.to_owned(),
                rule: "nothing is wider than the region that holds it".to_owned(),
                violation: format!(
                    "{} ends at x={} and its region ends at x={}",
                    held.name(),
                    held.right(),
                    centre.right()
                ),
                alternative: "bound it with the page width token rather than the window".to_owned(),
            });
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests;
