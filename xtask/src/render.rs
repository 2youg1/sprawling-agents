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
//! **The geometry is no longer here.** The left edge of the regions in
//! the main column, the first mark of the rows of a navigation column
//! and containment were three population judgements written beside this
//! gate; they are now three of the readings `xtask::survey` takes, and
//! this gate refuses the ones that reading marks as refusable
//! (section 8-26). A page's geometry with two homes is the defect that
//! instrument exists to prevent, and a gate is a consumer of a
//! measurement rather than a second place to define one.
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

use std::path::Path;

use crate::report::{Violation, XtaskError};
use crate::survey::{self, Deviation, Standing};

mod announced;
mod engine;
mod marks;
mod pass;
mod probe;

use announced::{every_control_is_announceable, every_landmark_is_named, one_first_heading};
use engine::{Measured, Opening, browser, measure};
use marks::no_key_is_underlined;
use pass::Pass;

/// The route that draws every state worth looking at, on fixtures.
const GALLERY: &str = "#/gallery";

/// Where a finding is located when no pass could be made at all.
const EVERY_PASS: &str = "in every pass";

/// The flag that prints everything the instrument read, not only what
/// this gate refuses.
const SURVEY: &str = "--survey";

/// The flag that asks for the same readings as named fields rather
/// than as sentences, for a reader that will act on them rather than
/// read them.
const TAGGED: &str = "--tagged";

/// The flag that opens a route other than the gallery.
const ROUTE: &str = "--route";

/// What one run of this gate is for.
///
/// Both errands take the same readings off the same page. What differs
/// is how much of the reading reaches a person: a gate prints what it
/// refuses, and a survey prints the rest as well, for the loop in which
/// somebody is changing the page rather than defending it.
enum Errand {
    Gate,
    Survey(survey::Shape),
}

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    // Where the bundle lands is the build script's statement, read
    // rather than repeated: this gate and the binary must open one
    // directory (xtask-SPEC.md section 8-18).
    let bundle = crate::bundle::dist(root)?;
    if !bundle.join("index.html").is_file() {
        return Ok(vec![violation(
            EVERY_PASS,
            "the client bundle this gate measures is built",
            format!(
                "{}/index.html is not built, so no page was measured",
                bundle.display()
            ),
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
    let route = asked_for(ROUTE).unwrap_or_else(|| GALLERY.to_owned());
    let errand = if flagged(SURVEY) {
        Errand::Survey(if flagged(TAGGED) {
            survey::Shape::Tagged
        } else {
            survey::Shape::Prose
        })
    } else {
        Errand::Gate
    };
    // The client's own sources, read once for the whole run: every
    // finding names the line that drew it, and a survey opens the same
    // page five times.
    let sources = survey::Sources::index(root)?;
    let opening = Opening {
        root,
        browser: &browser,
        bundle: &bundle,
        route: &route,
    };
    let mut violations = Vec::new();
    for pass in pass::wanted()? {
        let at = format!("{route} {}", pass.called());
        let measured = measure(&opening, pass)?;
        if let Some(refused) = unusable(pass, &measured, &at) {
            violations.push(refused);
            continue;
        }
        let page = &measured.page;
        every_control_is_announceable(&page.drawn, &at, &mut violations);
        every_landmark_is_named(&page.drawn, &at, &mut violations);
        one_first_heading(&page.drawn, &at, &mut violations);
        no_key_is_underlined(&page.drawn, &at, &mut violations);
        let readings = survey::judge(page);
        for reading in readings.iter().filter(refusable) {
            // A refusal names the line as well as the page: a gate that
            // says what is wrong and not where costs its reader the
            // search every time it goes red.
            let located = match sources.locate(&reading.at.class) {
                Some(found) => format!("{at} \u{b7} {found}"),
                None => at.clone(),
            };
            violations.push(violation(
                &located,
                &survey::rule(reading),
                survey::says(reading),
                &survey::remedy(reading),
            ));
        }
        if let Errand::Survey(shape) = errand {
            print!("{}", survey::written(page, &at, &readings, &sources, shape));
        }
    }
    Ok(violations)
}

/// Whether a reading this instrument took is one this gate refuses.
fn refusable(reading: &&Deviation<'_>) -> bool {
    matches!(reading.standing(), Standing::Refused)
}

/// Why one opening cannot be judged at all, when it cannot.
///
/// The pass reports itself first: a lighting the page did not take is a
/// pass that measured some other page, and reading the properties off
/// it would report green for a page nobody looked at.
fn unusable(pass: &Pass, measured: &Measured, at: &str) -> Option<Violation> {
    if measured.page.drawn.is_empty() {
        return Some(violation(
            at,
            "the gallery draws every state this gate measures",
            format!("{at} drew nothing measurable"),
            "repair the gallery route so it renders its fixtures",
        ));
    }
    pass.disagrees(&measured.reported).map(|difference| {
        violation(
            at,
            "the page draws itself in the conditions the pass asked for",
            difference,
            "the lighting is the `data-theme` attribute `theme.css` selects on, and the forced \
             mode is the engine's own switch: a pass that cannot set one measures a page it did \
             not ask for",
        )
    })
}

/// One finding, told where it was found: the same page at two widths is
/// two places a reader has to be sent to.
pub(super) fn violation(at: &str, rule: &str, subject: String, alternative: &str) -> Violation {
    Violation {
        gate: "render",
        location: at.to_owned(),
        rule: rule.to_owned(),
        violation: subject,
        alternative: alternative.to_owned(),
    }
}

/// Whether a person wrote one of this gate's switches on the command
/// line.
///
/// Read here rather than by the dispatcher for the reason `pass.rs`
/// gives for `--width`: the names of these switches and what they mean
/// are facts about this gate, and a dispatcher that parsed them would
/// be their second home.
fn flagged(flag: &str) -> bool {
    std::env::args().any(|arg| arg == flag)
}

/// The value a person wrote after one of this gate's switches.
fn asked_for(flag: &str) -> Option<String> {
    let mut args = std::env::args();
    while let Some(arg) = args.next() {
        if arg == flag {
            return args.next();
        }
    }
    None
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
