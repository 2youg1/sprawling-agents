// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One opening of the gallery: how wide the window is, how the page is
//! lit, and whether the engine is in a forced-colour mode (xtask-SPEC.md
//! section 8-16).
//!
//! **A property that holds at one width is not a property.** The three
//! columns are drawn by container queries, so what a rule collapses at
//! depends on how much room the column got, and a layout that is right
//! at 1440 can be wrong at 768 and at 2560 without a line of source
//! changing. The widths are named here once, and the run opens the page
//! at each of them.
//!
//! **What the pass asked for is checked against what the page drew.**
//! A pass that silently failed to light the page measures the dark page
//! twice and reports green for a lighting nobody looked at, which is the
//! failure this gate exists to refuse.

use crate::report::XtaskError;

/// A window with no room for a side column beside the main one.
const NARROW: u32 = 768;
/// The width most of this product is read at.
const READING: u32 = 1280;
/// A window wide enough to expose a column that stretches without a
/// limit.
const WIDE: u32 = 2560;

/// The widths the gallery is judged at, in CSS pixels.
const WIDTHS: [u32; 3] = [NARROW, READING, WIDE];

/// The width the lighting and forced-colour passes are drawn at.
///
/// Lighting and forced colours change paint and borders rather than
/// which column survives, so they are judged once, at the width this
/// product is most often read at. Widening the window is what the three
/// entries above are for.
const PAINTED_AT: u32 = READING;

/// The window height every pass uses, in CSS pixels. Height decides how
/// much scrolls, and nothing this gate asserts is about the fold.
pub(super) const HEIGHT: u32 = 1200;

/// The flag that narrows a run to one width, for the loop a person is in
/// while they change a stylesheet.
const FLAG: &str = "--width";

/// How the page is lit, as the root element spells it.
#[derive(PartialEq, Eq)]
pub(super) enum Lighting {
    Dark,
    Light,
}

/// Whether the engine replaces the page's colours with the system's, the
/// way Windows high contrast does.
#[derive(PartialEq, Eq)]
pub(super) enum Colours {
    AsAuthored,
    Forced,
}

impl Lighting {
    /// The value `data-theme` carries on the root element for this
    /// lighting, which is what `theme.css` selects on.
    pub(super) fn attribute(&self) -> &'static str {
        match *self {
            Lighting::Dark => "dark",
            Lighting::Light => "light",
        }
    }
}

impl Colours {
    fn called(&self) -> &'static str {
        match *self {
            Colours::AsAuthored => "the colours it authored",
            Colours::Forced => "forced colours",
        }
    }
}

/// One opening of the gallery.
pub(super) struct Pass {
    pub(super) width: u32,
    pub(super) lighting: Lighting,
    pub(super) colours: Colours,
}

/// Every opening one run makes: the three widths as the page is shipped,
/// then the two conditions section 9.0 of the roadmap closes on.
const PASSES: [Pass; 5] = [
    Pass {
        width: NARROW,
        lighting: Lighting::Dark,
        colours: Colours::AsAuthored,
    },
    Pass {
        width: READING,
        lighting: Lighting::Dark,
        colours: Colours::AsAuthored,
    },
    Pass {
        width: WIDE,
        lighting: Lighting::Dark,
        colours: Colours::AsAuthored,
    },
    Pass {
        width: PAINTED_AT,
        lighting: Lighting::Light,
        colours: Colours::AsAuthored,
    },
    Pass {
        width: PAINTED_AT,
        lighting: Lighting::Dark,
        colours: Colours::Forced,
    },
];

impl Pass {
    /// The `data-theme` value the root element carries for this pass.
    pub(super) fn theme(&self) -> &'static str {
        self.lighting.attribute()
    }

    /// Whether the engine is asked to replace the page's colours with
    /// the system's.
    pub(super) fn draws_forced_colours(&self) -> bool {
        self.colours == Colours::Forced
    }

    /// What a violation calls the pass it was found in.
    pub(super) fn called(&self) -> String {
        format!(
            "at {}px, {} in {}",
            self.width,
            self.lighting.attribute(),
            self.colours.called()
        )
    }

    /// What the page reported about itself, where it differs from what
    /// this pass asked for.
    ///
    /// **A forced-colour pass is not asked which way it is lit.** The
    /// mode replaces the page's colours with the system's, and the
    /// engine reports the system's scheme rather than the one the
    /// stylesheet declared - measured on this repository's own engine,
    /// which answers `light` under a forced-colour Windows theme
    /// whatever `data-theme` says.
    pub(super) fn disagrees(&self, reported: &Reported) -> Option<String> {
        if reported.colours != self.colours {
            return Some(format!(
                "the pass asked for {} and the engine drew {}",
                self.colours.called(),
                reported.colours.called()
            ));
        }
        if self.draws_forced_colours() {
            return None;
        }
        if reported.lighting != self.lighting {
            return Some(format!(
                "the pass asked for the {} page and the page drew itself {}",
                self.lighting.attribute(),
                reported.lighting.attribute()
            ));
        }
        None
    }
}

/// What the page said about itself while it was being measured.
pub(super) struct Reported {
    lighting: Lighting,
    colours: Colours,
}

impl Reported {
    /// Read from the two words the probe writes: whether the engine
    /// reports `forced-colors: active`, and the `color-scheme` the root
    /// element computed to.
    ///
    /// `color-scheme` rather than a token of this stylesheet's own: the
    /// light block declares it beside the eleven rungs that make the page
    /// light, so a page that is lit is a page that says so here.
    pub(super) fn read(forced: &str, scheme: &str) -> Reported {
        Reported {
            lighting: if scheme.contains("light") {
                Lighting::Light
            } else {
                Lighting::Dark
            },
            colours: if forced == "1" {
                Colours::Forced
            } else {
                Colours::AsAuthored
            },
        }
    }
}

/// The passes this run judges: all of them, or the ones drawn at the one
/// width a person named.
///
/// The flag is read here rather than by the dispatcher because its name,
/// the widths it accepts and the refusal a fourth width earns are all
/// facts about this gate, and a dispatcher that parsed it would be their
/// second home.
pub(super) fn wanted() -> Result<Vec<&'static Pass>, XtaskError> {
    let Some(asked) = asked_for() else {
        return Ok(PASSES.iter().collect());
    };
    let width = asked.parse::<u32>().ok();
    let chosen: Vec<&'static Pass> = PASSES
        .iter()
        .filter(|pass| width == Some(pass.width))
        .collect();
    if chosen.is_empty() {
        let offered: Vec<String> = WIDTHS.iter().map(u32::to_string).collect();
        return Err(XtaskError::Cmd {
            cmd: format!("cargo xtask render {FLAG} {asked}"),
            msg: format!(
                "this gate draws the gallery at {}px; {asked} is not one of them",
                offered.join("px, ")
            ),
        });
    }
    Ok(chosen)
}

/// The value a person wrote after `--width`, when they wrote one.
fn asked_for() -> Option<String> {
    let mut args = std::env::args();
    while let Some(arg) = args.next() {
        if arg == FLAG {
            return args.next();
        }
    }
    None
}

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "test code")]
mod tests;
