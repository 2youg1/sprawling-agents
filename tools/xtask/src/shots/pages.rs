// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which pictures one run takes, what each is called, and which of them
//! the run failed to write (tools/xtask/Spec.lean §8-44).
//!
//! **The pages are the client's, read rather than listed.** A route is
//! a key of the `BARE` table in `client/src/core/route.ts`, the table
//! the client reads a bare fragment through, so a route the front end
//! adds is photographed without a line changing here.

use std::path::Path;

/// One window a page is drawn in, in CSS pixels.
#[derive(Clone, Copy)]
pub(super) struct Frame {
    pub(super) width: u32,
    pub(super) height: u32,
}

/// The two windows every page is drawn in: a laptop's and a desktop
/// monitor's, the widths a person reviewing a screen asks for.
pub(super) const FRAMES: [Frame; 2] = [
    Frame {
        width: 1440,
        height: 900,
    },
    Frame {
        width: 1920,
        height: 1080,
    },
];

/// How a page is lit.
#[derive(Clone, Copy)]
pub(super) enum Lighting {
    Dark,
    Light,
}

impl Lighting {
    pub(super) const ALL: [Lighting; 2] = [Lighting::Dark, Lighting::Light];

    /// The word a picture's file name and the index carry.
    pub(super) fn name(self) -> &'static str {
        match self {
            Lighting::Dark => "dark",
            Lighting::Light => "light",
        }
    }

    /// The value the engine's `preferredColorScheme` setting takes for
    /// this lighting, which a page left at its default lighting follows.
    pub(super) fn scheme(self) -> u8 {
        match self {
            Lighting::Dark => 0,
            Lighting::Light => 1,
        }
    }
}

/// One picture: a route drawn in one window and one lighting, its main
/// region scrolled to one fold.
pub(super) struct Shot {
    pub(super) route: String,
    pub(super) fold: usize,
    pub(super) folds: usize,
    pub(super) frame: Frame,
    pub(super) lighting: Lighting,
    /// The names of the states whose section begins in this fold.
    pub(super) states: Vec<String>,
}

impl Shot {
    /// `<route>-<fold>-<width>-<lighting>.png`, the fold counted from one.
    pub(super) fn file(&self) -> String {
        format!(
            "{}-{:02}-{}-{}.png",
            self.route.replace('/', "-"),
            self.fold.saturating_add(1),
            self.frame.width,
            self.lighting.name()
        )
    }
}

/// The route heads of the `BARE` table in `route.ts`, in its order,
/// without the empty head, which spells the same page as `talk`.
pub(super) fn routes_in(route_ts: &str) -> Vec<String> {
    route_ts
        .lines()
        .skip_while(|line| !line.starts_with("const BARE"))
        .skip(1)
        .take_while(|line| line.trim() != "};")
        .filter(|line| !line.trim_start().starts_with("//"))
        .filter_map(|line| line.split_once(':'))
        .map(|(key, _)| key.trim().trim_matches('"').to_owned())
        .filter(|key| !key.is_empty())
        .collect()
}

/// Every picture of one route in one window: each fold in each
/// lighting. `folds` holds, per fold, the states that begin in it.
pub(super) fn shots_of(route: &str, frame: Frame, folds: &[Vec<String>]) -> Vec<Shot> {
    folds
        .iter()
        .enumerate()
        .flat_map(|(fold, states)| {
            Lighting::ALL.map(|lighting| Shot {
                route: route.to_owned(),
                fold,
                folds: folds.len(),
                frame,
                lighting,
                states: states.clone(),
            })
        })
        .collect()
}

/// The pictures of `shots` that `dir` does not hold, or holds empty.
/// An engine that exits without writing its picture leaves no file, and
/// one that fails half way can leave an empty one; both are missing.
pub(super) fn missing(dir: &Path, shots: &[Shot]) -> Vec<String> {
    shots
        .iter()
        .map(Shot::file)
        .filter(|file| !std::fs::metadata(dir.join(file)).is_ok_and(|meta| meta.len() > 0))
        .collect()
}

/// The index a person reads the pictures through, one row per picture.
pub(super) fn index(shots: &[Shot]) -> String {
    shots.iter().fold(
        String::from(
            "# Screenshots\n\nEvery page `cargo xtask shots` drew, at each width and lighting \
             (tools/xtask/Spec.lean §8-44).\n\n\
             | page | fold | width | lighting | picture | states in this fold |\n\
             |---|---|---|---|---|---|\n",
        ),
        |mut text, shot| {
            let file = shot.file();
            let states = shot
                .states
                .iter()
                .map(|state| state.replace('|', "\\|"))
                .collect::<Vec<String>>()
                .join("; ");
            text.push_str(&format!(
                "| `#/{}` | {} of {} | {} | {} | [{file}]({file}) | {states} |\n",
                shot.route,
                shot.fold.saturating_add(1),
                shot.folds,
                shot.frame.width,
                shot.lighting.name(),
            ));
            text
        },
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests {
    use super::*;

    /// Each route of a fixture table gets a picture in both windows and
    /// both lightings, and a directory short of one picture names it.
    #[test]
    fn every_route_gets_four_pictures_and_a_missing_one_is_named() {
        let route_ts = "export const LENSES = [];\n\
                        const BARE: Readonly<Record<string, View>> = {\n  \
                        \"\": DEFAULT_VIEW,\n  \
                        talk: DEFAULT_VIEW,\n  \
                        // a comment with: a colon\n  \
                        city: { kind: \"city\" },\n\
                        };\n\
                        const OLD = {\n  live: DEFAULT_VIEW,\n};\n";
        let routes = routes_in(route_ts);
        assert_eq!(routes, ["talk", "city"]);
        let shots: Vec<Shot> = routes
            .iter()
            .flat_map(|route| FRAMES.map(|frame| shots_of(route, frame, &[Vec::new()])))
            .flatten()
            .collect();
        for route in &routes {
            assert_eq!(
                shots.iter().filter(|shot| &shot.route == route).count(),
                4,
                "{route}"
            );
        }
        let dir = std::env::temp_dir().join(format!("xtask-shots-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (last, written) = shots.split_last().unwrap();
        for shot in written {
            std::fs::write(dir.join(shot.file()), b"png").unwrap();
        }
        std::fs::write(dir.join(last.file()), b"").unwrap();
        let absent = missing(&dir, &shots);
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(absent, [last.file()]);
    }
}
