// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

#[derive(Clone, Copy)]
pub(super) struct Frame {
    pub(super) width: u32,
    pub(super) height: u32,
}

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

#[derive(Clone, Copy)]
pub(super) enum Lighting {
    Dark,
    Light,
}

impl Lighting {
    pub(super) const ALL: [Lighting; 2] = [Lighting::Dark, Lighting::Light];

    pub(super) fn name(self) -> &'static str {
        match self {
            Lighting::Dark => "dark",
            Lighting::Light => "light",
        }
    }

    pub(super) fn scheme(self) -> u8 {
        match self {
            Lighting::Dark => 0,
            Lighting::Light => 1,
        }
    }
}

pub(super) struct Shot {
    pub(super) route: String,
    pub(super) fold: usize,
    pub(super) folds: usize,
    pub(super) frame: Frame,
    pub(super) lighting: Lighting,

    pub(super) states: Vec<String>,
}

impl Shot {

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

pub(super) fn missing(dir: &Path, shots: &[Shot]) -> Vec<String> {
    shots
        .iter()
        .map(Shot::file)
        .filter(|file| !std::fs::metadata(dir.join(file)).is_ok_and(|meta| meta.len() > 0))
        .collect()
}

pub(super) fn index(shots: &[Shot]) -> String {
    shots.iter().fold(
        String::from(
            "# Screenshots\n\nEvery page `cargo xtask shots` drew, at each width and lighting \
             (Tools/xtask/Spec.lean §8-44).\n\n\
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
