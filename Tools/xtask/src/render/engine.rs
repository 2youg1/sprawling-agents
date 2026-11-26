// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

pub(super) mod devtools;
pub(crate) mod launch;

use std::path::{Path, PathBuf};
use std::time::Duration;

use super::pass::{Pass, Reported};
use crate::report::XtaskError;
use browser::survey::probe::{Read, declared};
use browser::survey::{Page, PaintSource};

const PATIENCE: Duration = Duration::from_secs(300);

const CHROMIUM: &str = include_str!("../../../../Sprawling/app/src/doctor/family/chromium.tsv");

pub(crate) fn browser() -> Option<PathBuf> {
    if let Some(named) = std::env::var_os("SPRAWLING_BROWSER") {
        let path = PathBuf::from(named);
        if path.is_file() {
            return Some(path);
        }
    }
    if let Some(installed) = in_components() {
        return Some(installed);
    }
    chromium_members().find_map(|(program, places)| {
        places
            .filter_map(installation_path)
            .find(|path| path.is_file())
            .or_else(|| on_path(program))
    })
}

fn installation_path(place: &str) -> Option<PathBuf> {
    let path = PathBuf::from(place);
    if path.is_absolute() {
        return Some(path);
    }
    let relative = place.strip_prefix(r"%LOCALAPPDATA%\")?;
    let user = PathBuf::from(std::env::var_os("LOCALAPPDATA")?);
    user.is_absolute().then(|| user.join(relative))
}

fn chromium_members() -> impl Iterator<Item = (&'static str, std::str::Split<'static, char>)> {
    let column = if cfg!(windows) {
        2
    } else if cfg!(target_os = "macos") {
        3
    } else {
        4
    };
    CHROMIUM
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(move |line| {
            let fields: Vec<&'static str> = line.split('\t').collect();
            let program = fields.get(1)?;
            let places = fields.get(column)?;
            Some((*program, places.split('|')))
        })
}

fn in_components() -> Option<PathBuf> {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))?;
    let components = PathBuf::from(home).join(".sprawling").join("components");
    const UNDER: [&str; 4] = [
        "firefox/firefox.exe",
        "firefox/firefox",
        "chromium/chrome.exe",
        "chromium/chrome",
    ];
    UNDER
        .into_iter()
        .map(|leaf| components.join(leaf))
        .find(|path| path.is_file())
}

fn on_path(program: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths)
        .flat_map(|dir| [dir.join(program), dir.join(format!("{program}.exe"))])
        .find(|candidate| candidate.is_file())
}

pub(super) struct Opening<'a> {
    pub(super) root: &'a Path,
    pub(super) browser: &'a Path,
    pub(super) bundle: &'a Path,
    pub(super) route: &'a str,
}

pub(super) struct Measured {
    pub(super) page: Page,
    pub(super) reported: Reported,
}

pub(super) fn read_measured(
    read: Read,
    pass: &Pass,
    command: &str,
) -> Result<Measured, XtaskError> {
    let vocabulary = declared(&read.declared);
    if vocabulary.is_empty() {
        return Err(XtaskError::Cmd {
            cmd: command.to_owned(),
            msg: "the probe read no declared words off the root element; the engine does not \
                  enumerate custom properties, so nothing can be judged against them"
                .to_owned(),
        });
    }
    let Some((forced, scheme)) = read.conditions() else {
        return Err(XtaskError::Cmd {
            cmd: command.to_owned(),
            msg: format!(
                "the probe wrote `{}`, which names neither a forced-colour \
                 mode nor a colour scheme",
                read.conditions,
            ),
        });
    };

    Ok(Measured {
        page: read.page(if pass.draws_forced_colours() {
            PaintSource::TheSystem
        } else {
            PaintSource::ThePage
        }),
        reported: Reported::read(forced, scheme),
    })
}

pub(crate) fn preloads<'a>(body: &str, chunks: impl IntoIterator<Item = &'a str>) -> String {
    chunks
        .into_iter()
        .filter(|chunk| !body.contains(&format!("./assets/{chunk}")))
        .map(|chunk| {
            format!(
                "<link rel=\"modulepreload\" crossorigin href=\"./assets/{chunk}\">
"
            )
        })
        .collect()
}

pub(crate) fn url_of(path: &Path) -> String {
    let absolute = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let text = absolute.display().to_string();
    let cleaned = text
        .strip_prefix(r"\\?\")
        .unwrap_or(&text)
        .replace('\\', "/");
    if cleaned.starts_with('/') {
        format!("file://{cleaned}")
    } else {
        format!("file:///{cleaned}")
    }
}

#[cfg(test)]
mod tests {
    use super::preloads;

    #[test]
    fn a_chunk_the_page_does_not_load_is_preloaded() {
        let body = r#"<script type="module" crossorigin src="./assets/index-A.js"></script>"#;
        assert_eq!(
            preloads(body, ["gallery-B.js", "index-A.js"]),
            "<link rel=\"modulepreload\" crossorigin href=\"./assets/gallery-B.js\">
"
        );
    }
}
