// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::report::XtaskError;

const NARROW: u32 = 768;

const READING: u32 = 1280;

const WIDE: u32 = 2560;

const WIDTHS: [u32; 3] = [NARROW, READING, WIDE];

const PAINTED_AT: u32 = READING;

pub(super) const HEIGHT: u32 = 1200;

const FLAG: &str = "--width";

#[derive(PartialEq, Eq)]
pub(super) enum Lighting {
    Dark,
    Light,
}

#[derive(PartialEq, Eq)]
pub(super) enum Colours {
    AsAuthored,
    Forced,
}

impl Lighting {

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

    fn slug(&self) -> &'static str {
        match *self {
            Colours::AsAuthored => "authored",
            Colours::Forced => "forced",
        }
    }
}

pub(super) struct Pass {
    pub(super) width: u32,
    pub(super) lighting: Lighting,
    pub(super) colours: Colours,
}

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

    pub(super) fn theme(&self) -> &'static str {
        self.lighting.attribute()
    }

    pub(super) fn draws_forced_colours(&self) -> bool {
        self.colours == Colours::Forced
    }

    pub(super) fn slug(&self) -> String {
        format!("{}-{}-{}", self.width, self.theme(), self.colours.slug())
    }

    pub(super) fn called(&self) -> String {
        format!(
            "at {}px, {} in {}",
            self.width,
            self.lighting.attribute(),
            self.colours.called()
        )
    }

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

pub(super) struct Reported {
    lighting: Lighting,
    colours: Colours,
}

impl Reported {

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
