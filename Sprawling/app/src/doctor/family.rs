// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod chromium;
mod gecko;
mod webkit;

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::probe::on_search_path;
use super::{Absence, Detection, Need, PerPlatform, Platform, Presence, Recipe, Requirement, Tier};
use super::{Group, registry};

pub(crate) const BROWSER_VARIABLE: &str = "SPRAWLING_BROWSER";

pub(crate) const GECKO: &str = "gecko";
pub(crate) const CHROMIUM: &str = "chromium";
pub(crate) const WEBKIT: &str = "webkit";

const LOCAL_APP_DATA: &str = "LOCALAPPDATA";
pub(super) const LOCAL_APP_DATA_PREFIX: &str = r"%LOCALAPPDATA%\";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Family {
    Gecko,

    Chromium,

    WebKit,
}

impl Family {
    #[cfg(test)]
    pub(crate) const ALL: [Family; 3] = [Family::Gecko, Family::Chromium, Family::WebKit];

    pub(crate) fn members(self) -> &'static [Member] {
        match self {
            Family::Gecko => gecko::MEMBERS,
            Family::Chromium => chromium::MEMBERS,
            Family::WebKit => webkit::MEMBERS,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Confidence {
    Tried,

    NeedsConfirmation,

    Experimental,
}

impl Confidence {
    pub(crate) fn caveat(self) -> &'static str {
        match self {
            Confidence::Tried => "",
            Confidence::NeedsConfirmation => ", confirm it by hand once",
            Confidence::Experimental => ", experimental",
        }
    }
}

pub(crate) struct Member {
    pub(crate) name: &'static str,

    pub(crate) program: &'static str,
    pub(crate) homepage: &'static str,
    pub(crate) confidence: Confidence,

    pub(crate) places: PerPlatform<&'static [&'static str]>,

    pub(crate) start_menu: &'static str,
}

pub(crate) const GECKO_ROW: Requirement = Requirement {
    name: GECKO,
    tier: Tier::Use,
    need: Need::OneOf(Group::BrowserEngine),
    enables: "the browser tool, driven without a driver; Firefox, Zen, LibreWolf, \
              Waterfox, Floorp and Firefox Developer all answer here, and Tor Browser \
              answers too but wants confirming by hand",
    detect: Detection::Family(Family::Gecko),
    homepage: Some("https://www.mozilla.org/firefox/new/"),
    recipe: PerPlatform {
        windows: Recipe::Command {
            program: "winget",
            args: &["install", "--id", "Mozilla.Firefox", "-e"],
        },
        macos: Recipe::Command {
            program: "brew",
            args: &["install", "--cask", "firefox"],
        },
        linux: Recipe::Print("sudo apt install firefox"),
    },
    pin: super::Pin::Unpinned,
    upstream: super::Upstream::Unread(wire::DoctorUnread::ManyBrands),
    pack: None,
};

pub(crate) const CHROMIUM_ROW: Requirement = Requirement {
    name: CHROMIUM,
    tier: Tier::Use,
    need: Need::Optional,
    enables: "the browser tool through chromedriver or msedgedriver, whose major \
              version must match the browser found here",
    detect: Detection::Family(Family::Chromium),
    homepage: Some("https://www.chromium.org/getting-involved/download-chromium/"),
    recipe: CHROMIUM_RECIPE,
    pin: super::Pin::Unpinned,
    upstream: super::Upstream::Unread(wire::DoctorUnread::ManyBrands),
    pack: None,
};

pub(crate) const CHROMIUM_RECIPE: PerPlatform<Recipe> = PerPlatform {
    windows: Recipe::Command {
        program: "winget",
        args: &["install", "--id", "Google.Chrome", "-e"],
    },
    macos: Recipe::Command {
        program: "brew",
        args: &["install", "--cask", "google-chrome"],
    },
    linux: Recipe::Print("sudo apt install chromium"),
};

pub(crate) const WEBKIT_ROW: Requirement = Requirement {
    name: WEBKIT,
    tier: Tier::Use,
    need: Need::OneOf(Group::BrowserEngine),
    enables: "the browser tool against Safari, experimentally: BiDi support is partial \
              and `safaridriver --enable` has to be run once",
    detect: Detection::Family(Family::WebKit),
    homepage: Some("https://www.apple.com/safari/"),
    recipe: PerPlatform {
        windows: Recipe::Manual("Safari runs on macOS only"),
        macos: Recipe::Manual("run `safaridriver --enable` once; Safari itself ships with macOS"),
        linux: Recipe::Manual("Safari runs on macOS only"),
    },
    pin: super::Pin::Unpinned,
    upstream: super::Upstream::Unread(wire::DoctorUnread::ManyBrands),
    pack: None,
};

pub(super) fn look(family: Family, platform: Option<Platform>, search_path: &OsString) -> Presence {
    if let Some(named) = std::env::var_os(BROWSER_VARIABLE).filter(|named| !named.is_empty()) {
        let path = PathBuf::from(&named);
        if claims(family, &path) {
            return match path.is_file() {
                true => found_at(path),
                false => Presence::Absent(Absence::VariableNamesNothing {
                    variable: BROWSER_VARIABLE,
                    path,
                }),
            };
        }
        return Presence::Absent(Absence::NotOnSearchPath);
    }
    let found = family.members().iter().find_map(|member| {
        on_search_path(search_path, member.program)
            .or_else(|| platform.and_then(|platform| first_file(member.places.at(platform))))
            .or_else(|| registry::installed_at(member.program, member.start_menu))
    });
    match found {
        None => Presence::Absent(Absence::NotOnSearchPath),
        Some(path) => found_at(path),
    }
}

fn found_at(at: PathBuf) -> Presence {
    let version = super::version_file::beside(&at);
    Presence::Present { at, version }
}

pub(crate) fn member_at(family: Family, at: &Path) -> Option<&'static Member> {
    let members = family.members();
    let placed = members.iter().find(|member| {
        [Platform::Windows, Platform::MacOs, Platform::Linux]
            .into_iter()
            .any(|platform| {
                member
                    .places
                    .at(platform)
                    .iter()
                    .filter_map(|place| installation_path(place))
                    .any(|place| match platform {
                        Platform::Windows => place.as_os_str().eq_ignore_ascii_case(at.as_os_str()),
                        Platform::MacOs | Platform::Linux => place == at,
                    })
            })
    });
    placed.or_else(|| {
        let stem = at.file_stem().and_then(|stem| stem.to_str())?;
        members
            .iter()
            .find(|member| member.program.eq_ignore_ascii_case(stem))
    })
}

fn claims(family: Family, path: &Path) -> bool {
    let owner = [Family::Gecko, Family::Chromium, Family::WebKit]
        .into_iter()
        .find(|candidate| member_at(*candidate, path).is_some());
    owner.unwrap_or(Family::Gecko) == family
}

fn installation_path(place: &str) -> Option<PathBuf> {
    match place.strip_prefix(LOCAL_APP_DATA_PREFIX) {
        Some(tail) => std::env::var_os(LOCAL_APP_DATA)
            .map(PathBuf::from)
            .filter(|base| base.is_absolute())
            .map(|base| base.join(tail)),
        None => Some(PathBuf::from(place)),
    }
}

fn first_file(places: &[&str]) -> Option<PathBuf> {
    places
        .iter()
        .filter_map(|place| installation_path(place))
        .find(|candidate| candidate.is_file())
}
