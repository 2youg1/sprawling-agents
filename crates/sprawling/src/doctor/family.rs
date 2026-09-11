// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A browser is an engine, and an engine has many brands
//! (sprawling-SPEC.md section 8-57).
//!
//! One row per engine family, not one row per vendor: a machine with
//! Zen on it has a Gecko browser, and a doctor that asks for `firefox`
//! by name tells that person they are missing something they already
//! have. Each family lists its members with the places this platform
//! installs them; the first member this machine answers for is the
//! family's answer.
//!
//! `SPRAWLING_BROWSER` is the override, and it is read here alone. The
//! browser engine takes the path out of the `Presence` this returns
//! (`bin::browser_bidi::engine`), so the variable a person sets and the
//! program a run starts cannot be two different browsers.

mod chromium;
mod gecko;
mod webkit;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::probe::{ask_version, on_search_path};
use super::{Absence, Detection, Need, PerPlatform, Platform, Presence, Recipe, Requirement, Tier};
use super::{Group, registry};

/// The environment variable that names the browser to use, whatever
/// this table found. Spelled here and nowhere else in this crate.
pub(crate) const BROWSER_VARIABLE: &str = "SPRAWLING_BROWSER";

/// The names the rest of this binary asks the families by.
pub(crate) const GECKO: &str = "gecko";
pub(crate) const CHROMIUM: &str = "chromium";
pub(crate) const WEBKIT: &str = "webkit";

/// The three engines that can hold a BiDi session, by what they run
/// rather than by who sells them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Family {
    /// Speaks BiDi itself: nothing stands between the city and it.
    Gecko,
    /// Reachable only through a driver process, which is its own row.
    Chromium,
    /// Safari through `safaridriver`, on macOS alone.
    WebKit,
}

impl Family {
    /// Every family, for the tests that hold each one to a row.
    #[cfg(test)]
    pub(crate) const ALL: [Family; 3] = [Family::Gecko, Family::Chromium, Family::WebKit];

    /// The members, in the order this city would choose between them.
    /// A member a person must confirm by hand comes last, so a machine
    /// with an ordinary browser on it never answers with the awkward
    /// one.
    pub(crate) fn members(self) -> &'static [Member] {
        match self {
            Family::Gecko => gecko::MEMBERS,
            Family::Chromium => chromium::MEMBERS,
            Family::WebKit => webkit::MEMBERS,
        }
    }
}

/// How far this project trusts one member to hold a session.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Confidence {
    /// Driven in this project and expected to work.
    Tried,
    /// It starts, and something about the way it is packaged can stop
    /// the session: a launcher that replaces the process, a proxy it
    /// insists on. A person confirms it once by hand.
    NeedsConfirmation,
    /// The protocol support itself is partial.
    Experimental,
}

impl Confidence {
    /// What the report adds after the member's name, for a member that
    /// is not simply expected to work. An empty clause for the ordinary
    /// case, so a row about Firefox carries no warning nobody needs.
    pub(crate) fn caveat(self) -> &'static str {
        match self {
            Confidence::Tried => "",
            Confidence::NeedsConfirmation => ", confirm it by hand once",
            Confidence::Experimental => ", experimental",
        }
    }
}

/// One brand inside a family.
pub(crate) struct Member {
    /// What a person calls it, and what the report names.
    pub(crate) name: &'static str,
    /// The program name a shell resolves, when this one is on the
    /// search path at all.
    pub(crate) program: &'static str,
    pub(crate) homepage: &'static str,
    pub(crate) confidence: Confidence,
    /// Where this platform puts it when no shell resolves it.
    pub(crate) places: PerPlatform<&'static [&'static str]>,
    /// The `StartMenuInternet` key Windows registers a browser under.
    /// Empty when this brand registers none.
    pub(crate) start_menu: &'static str,
}

/// The Gecko row: the engine that needs no driver, so it leads.
pub(crate) const GECKO_ROW: Requirement = Requirement {
    name: GECKO,
    tier: Tier::Use,
    need: Need::OneOf(Group::BrowserEngine),
    enables: "the browser tool, driven without a driver; Firefox, Zen, LibreWolf, \
              Waterfox, Floorp and Firefox Developer all answer here, and Tor Browser \
              answers too but wants confirming by hand",
    detect: Detection::Family(Family::Gecko),
    homepage: Some("https://www.mozilla.org/firefox/"),
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
};

/// The Chromium row. Optional, and that is the whole point: a Chromium
/// browser on its own opens no session, because the way in is the
/// driver beside it, which is its own row.
pub(crate) const CHROMIUM_ROW: Requirement = Requirement {
    name: CHROMIUM,
    tier: Tier::Use,
    need: Need::Optional,
    enables: "the browser tool through chromedriver or msedgedriver, whose major \
              version must match the browser found here",
    detect: Detection::Family(Family::Chromium),
    homepage: Some("https://www.chromium.org/"),
    recipe: PerPlatform {
        windows: Recipe::Command {
            program: "winget",
            args: &["install", "--id", "Google.Chrome", "-e"],
        },
        macos: Recipe::Command {
            program: "brew",
            args: &["install", "--cask", "google-chrome"],
        },
        linux: Recipe::Print("sudo apt install chromium"),
    },
};

/// The WebKit row: Safari through its driver, on macOS and nowhere
/// else. Marked experimental because the protocol support is partial.
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
};

/// Which member of this family this machine has, and where.
///
/// The override wins over everything, including a family that has a
/// member installed: a person who names a browser has answered the
/// question this function otherwise asks the machine.
pub(super) fn look(
    family: Family,
    platform: Option<Platform>,
    patience: Duration,
    search_path: &OsString,
) -> Presence {
    if let Some(named) = std::env::var_os(BROWSER_VARIABLE).filter(|named| !named.is_empty()) {
        let path = PathBuf::from(&named);
        if claims(family, &path) {
            return match path.is_file() {
                true => ask_version(&path, "--version", patience),
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
        Some(path) => ask_version(&path, "--version", patience),
    }
}

/// Which member a path is, for a report that wants to say `zen` rather
/// than `gecko` and to link to the site of the browser it found.
///
/// The place a platform installs a brand at is compared first, because
/// two brands can install a program under one name: Firefox Developer
/// Edition's program file is called `firefox`, and only its directory
/// tells the two apart.
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
                    .any(|place| Path::new(place) == at)
            })
    });
    placed.or_else(|| {
        let stem = at.file_stem().and_then(|stem| stem.to_str())?;
        members
            .iter()
            .find(|member| member.program.eq_ignore_ascii_case(stem))
    })
}

/// Whether an overridden browser belongs to this family.
///
/// A path is matched by file stem against every member of every family,
/// so exactly one family claims a browser this table knows. A browser
/// no family knows is claimed by Gecko, because the driverless launch
/// is the one a fork of anything answers to and the alternative - an
/// override nobody claims - reports "not on the search path" about a
/// file the person is looking at.
fn claims(family: Family, path: &Path) -> bool {
    let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
        return family == Family::Gecko;
    };
    let owner = [Family::Gecko, Family::Chromium, Family::WebKit]
        .into_iter()
        .find(|candidate| {
            candidate
                .members()
                .iter()
                .any(|member| member.program.eq_ignore_ascii_case(stem))
        });
    owner.unwrap_or(Family::Gecko) == family
}

/// The first of these paths that is a file on this machine.
fn first_file(places: &[&str]) -> Option<PathBuf> {
    places
        .iter()
        .map(PathBuf::from)
        .find(|candidate| candidate.is_file())
}
