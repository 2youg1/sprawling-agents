// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use browser::survey::{self, Deviation, Standing};

use crate::report::{Violation, XtaskError};

mod announced;
pub(crate) mod engine;
mod looks;
mod marks;
mod parts;
mod pass;
pub(crate) mod probe;
mod room;
mod sources;
mod talk;

use announced::{every_control_is_announceable, every_landmark_is_named, one_first_heading};
use engine::{Measured, Opening, browser, measure};
use marks::no_key_is_underlined;
use parts::{ROSTER, every_part_draws_its_roles};
use pass::Pass;
use room::{every_popover_shows_an_option, no_text_is_crushed};

const GALLERY: &str = "gallery";

const EVERY_PASS: &str = "in every pass";

const SURVEY: &str = "--survey";

const TAGGED: &str = "--tagged";

const ROUTE: &str = "--route";

const BUNDLE: &str = "--bundle";

enum Errand {
    Gate,
    Survey(survey::Shape),
}

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let bundle = match asked_for(BUNDLE) {
        Some(dir) => root.join(dir),
        None => crate::bundle::dist(root)?,
    };

    let mut violations = looks::check(root)?;
    if !bundle.join("index.html").is_file() {
        violations.push(violation(
            EVERY_PASS,
            "the client bundle this gate measures is built",
            format!(
                "{}/index.html is not built, so no page was measured",
                bundle.display()
            ),
            "run `just build-web`",
        ));
        return Ok(violations);
    }
    let Some(browser) = browser() else {
        violations.push(violation(
            EVERY_PASS,
            "a real engine draws the page this gate measures",
            "no headless browser was found, so no page was measured".to_owned(),
            "install a Chromium-family browser, or point `SPRAWLING_BROWSER` at one",
        ));
        return Ok(violations);
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

    let sources = sources::index(root)?;

    let talk = if route == GALLERY {
        Some((talk::Words::read(root)?, crate::budget::register(root)?))
    } else {
        None
    };
    let opening = Opening {
        root,
        browser: &browser,
        bundle: &bundle,
        route: &route,
    };
    for (pass, measured) in measure_every(&opening, pass::wanted()?)? {
        let at = format!("{route} {}", pass.called());
        if let Some(refused) = unusable(pass, &measured, &at) {
            violations.push(refused);
            continue;
        }
        let page = &measured.page;
        every_control_is_announceable(&page.drawn, &at, &mut violations);
        every_landmark_is_named(&page.drawn, &at, &mut violations);
        one_first_heading(&page.drawn, &at, &mut violations);
        no_key_is_underlined(&page.drawn, &at, &mut violations);
        no_text_is_crushed(&page.drawn, &at, &mut violations);
        every_popover_shows_an_option(&page.drawn, &at, &mut violations);
        every_part_draws_its_roles(ROSTER, &page.drawn, &at, &mut violations);
        if let Some((words, register)) = &talk {
            let counted = talk::the_conversation_page_holds_its_controls(
                &page.drawn,
                &at,
                words,
                &mut violations,
            );
            talk::talk_controls_within_register(counted, register, &at, &mut violations);
        }
        let readings = survey::judge(page);
        for reading in readings.iter().filter(refusable) {
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
            let survey = survey::Survey {
                page,
                at: &at,
                found: &readings,
                sources: &sources,
            };
            print!("{}", survey::written(survey, shape));
        }
    }
    Ok(violations)
}

const PROCESSORS_PER_PASS: usize = 3;
const MAX_AT_ONCE: usize = 2;

fn measure_every<'p>(
    opening: &Opening,
    passes: Vec<&'p Pass>,
) -> Result<Vec<(&'p Pass, Measured)>, XtaskError> {
    let at_once = match std::thread::available_parallelism() {
        Ok(processors) => passes_at_once(processors.get()),
        Err(_) => 1,
    };
    let mut measured = Vec::with_capacity(passes.len());
    for group in passes.chunks(at_once) {
        measured.extend(std::thread::scope(|scope| {
            let running: Vec<_> = group
                .iter()
                .map(|&pass| (pass, scope.spawn(move || measure(opening, pass))))
                .collect();
            running
                .into_iter()
                .map(|(pass, handle)| {
                    let opened = handle.join().map_err(|_| XtaskError::Cmd {
                        cmd: format!("cargo xtask render, {}", pass.called()),
                        msg: "the thread that opened this pass panicked".to_owned(),
                    })??;
                    Ok((pass, opened))
                })
                .collect::<Result<Vec<_>, XtaskError>>()
        })?);
    }
    Ok(measured)
}

fn passes_at_once(processors: usize) -> usize {
    (processors / PROCESSORS_PER_PASS).clamp(1, MAX_AT_ONCE)
}

fn refusable(reading: &&Deviation<'_>) -> bool {
    matches!(reading.standing(), Standing::Refused)
}

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

pub(super) fn violation(at: &str, rule: &str, subject: String, alternative: &str) -> Violation {
    Violation {
        gate: "render",
        location: at.to_owned(),
        rule: rule.to_owned(),
        violation: subject,
        alternative: alternative.to_owned(),
    }
}

fn flagged(flag: &str) -> bool {
    std::env::args().any(|arg| arg == flag)
}

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
