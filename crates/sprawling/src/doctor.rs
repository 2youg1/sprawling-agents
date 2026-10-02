// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What this machine has, what this city needs, and what installing the
//! difference would cost (`crates/sprawling/spec/Accounting/Worker.lean` §8-40).
//!
//! Two tiers rather than one list: a person who only wants the city to
//! run needs a browser, and a person who wants to change this code needs
//! a test runner as well. Reporting both as one number would tell the
//! first person they are missing something they will never use.
//!
//! The table is data (`table`), this machine is an adapter (`probe`),
//! the terminal is `screen`, and everything here is the judgement
//! between them: what one line about one item says, and what the
//! verdict for a tier is. Nothing in this file starts a process, reads
//! the clock, or writes to a terminal.
//!
//! **This module is the one authority on what this machine has.** The
//! exec tool, the browser engine and the report all ask it through
//! `host` (section 8-47); nothing else in this binary reads the search
//! path, an environment variable naming a program, or a feature flag
//! naming an engine.

mod asking;
mod explain;
mod family;
pub(crate) mod github;
pub(crate) mod host;
mod needs;
mod pack;
mod paint;
mod pin;
mod presence;
mod probe;
mod registry;
mod report;
mod running;
mod screen;
mod table;
mod upstream;
mod version_file;
mod visit;

pub(crate) use family::Family;
pub(crate) use pack::Pack;
pub(crate) use pin::Pin;
pub(crate) use presence::{Absence, Fault, Presence, Version};
pub(crate) use probe::{Machine, ThisMachine, on_search_path};
pub(crate) use report::answer;
pub use screen::verb;
pub(crate) use table::{REQUIREMENTS, recipe_for};
pub(crate) use upstream::{Upstream, newest};

use accounting::Recipe;

/// How long one `--version` call may take before this city stops
/// waiting for it. Generous enough for a cold start on a slow disk,
/// short enough that a dozen of them never feel like a hang.
pub(crate) const PATIENCE: std::time::Duration = std::time::Duration::from_secs(5);

/// Who needs this item. A tier is answered on its own, so an item one
/// tier needs never appears in the other tier's verdict.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Tier {
    /// Enough to run a city on this machine.
    Use,
    /// Enough to change this code and close `just check`.
    Develop,
}

impl Tier {
    pub(crate) const ALL: [Tier; 2] = [Tier::Use, Tier::Develop];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Tier::Use => "use",
            Tier::Develop => "develop",
        }
    }
}

/// Whether the tier can be reached without this item.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Need {
    Required,
    /// Required as one of a group, any member of which is enough. A
    /// machine with Zen on it needs no Chrome, and a machine with Edge
    /// and its driver needs no Gecko browser: the verdict counts the
    /// group once, by its name, rather than counting each member.
    OneOf(Group),
    /// Absent is a fact rather than a fault; `enables` says what having
    /// it would add.
    Optional,
}

/// A set of items any one of which satisfies a tier.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Group {
    /// A browser the tool can hold a BiDi session with: a Gecko
    /// browser, a Chromium driver, or `safaridriver`. Opening the WebUI
    /// is not what this is for - any browser at all opens that.
    BrowserEngine,
}

impl Group {
    /// The name a verdict reports the whole group by.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Group::BrowserEngine => "a browser engine",
        }
    }
}

/// The three platforms this project is built for. `None` from `current`
/// is an honest answer: on a fourth platform nothing here can name a
/// package manager, so `--install` prints and stops.
///
/// `current` is the one place in this binary that asks which platform it
/// is running on. Everything else matches on the answer, exhaustively
/// and with no wildcard arm, so a fourth platform becomes a compile
/// error at every site that would have to decide something about it.
///
/// `pub` for the reason lib.rs states: `bin::install` enters through it,
/// and the binary is part of the "this binary" the paragraph above is
/// about. Keeping it in reach of the library alone left the installer
/// asking `cfg!(target_os = ...)` on its own, which is the second
/// sampling point this type exists to abolish.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Platform {
    Windows,
    MacOs,
    Linux,
}

impl Platform {
    /// Every platform the table must answer for. Only the tests walk it:
    /// a running machine is one platform, and the completeness of the
    /// table is a question about all three.
    #[cfg(test)]
    pub(crate) const ALL: [Platform; 3] = [Platform::Windows, Platform::MacOs, Platform::Linux];

    pub fn current() -> Option<Platform> {
        if cfg!(target_os = "windows") {
            Some(Platform::Windows)
        } else if cfg!(target_os = "macos") {
            Some(Platform::MacOs)
        } else if cfg!(target_os = "linux") {
            Some(Platform::Linux)
        } else {
            None
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Platform::Windows => "windows",
            Platform::MacOs => "macos",
            Platform::Linux => "linux",
        }
    }

    /// The character that separates one directory from the next inside
    /// this platform's search path variable. `probe` walks that
    /// variable, and reading it with the wrong separator reports an
    /// installed tool as one this machine does not have.
    pub(crate) const fn search_path_separator(self) -> char {
        match self {
            Platform::Windows => ';',
            Platform::MacOs | Platform::Linux => ':',
        }
    }
}

/// One value per platform, so the table states all three and the running
/// machine picks one. Three fields rather than a map: a platform with no
/// entry would be a hole a reader has to notice.
pub(crate) struct PerPlatform<T> {
    pub(crate) windows: T,
    pub(crate) macos: T,
    pub(crate) linux: T,
}

impl<T> PerPlatform<T> {
    pub(crate) fn at(&self, platform: Platform) -> &T {
        match platform {
            Platform::Windows => &self.windows,
            Platform::MacOs => &self.macos,
            Platform::Linux => &self.linux,
        }
    }
}

/// How this machine is asked whether the item is here.
pub(crate) enum Detection {
    /// A program resolved on the search path, or found at one of the
    /// places this platform installs it, and then asked its version.
    Program {
        program: &'static str,
        version_arg: &'static str,
        places: PerPlatform<&'static [&'static str]>,
    },
    /// A program on the search path that lists what it manages; the
    /// item is here when one line it prints starts with `line`. An
    /// empty `line` asks only that it printed a line at all.
    Listed {
        program: &'static str,
        args: &'static [&'static str],
        line: &'static str,
    },
    /// A file this city keeps at machine level. The variable wins when
    /// it is set, and a set variable that names nothing is reported
    /// rather than fallen through; otherwise the file is looked for
    /// under `~/.sprawling/components/<item>/`.
    Component {
        variable: &'static str,
        file: &'static str,
    },
    /// The interpreter a platform's variable names, or the one it has
    /// when the variable is unset.
    Interpreter {
        variable: PerPlatform<&'static str>,
        fallback: PerPlatform<&'static str>,
    },
    /// A part of this binary rather than of this machine: present when
    /// the build carries it and it starts.
    Built { carried: bool },
    /// Any member of one browser family, the first that answers.
    /// `SPRAWLING_BROWSER` overrides it (`doctor::family`).
    Family(Family),
}

/// One thing this city needs, and everything that can be said about it
/// without touching the machine.
pub(crate) struct Requirement {
    pub(crate) name: &'static str,
    pub(crate) tier: Tier,
    pub(crate) need: Need,
    /// What having it lets a person do, in one clause of the terminal
    /// report's English. The page words it itself, under
    /// `machine_enables_<name>` in `client/src/lang.json`, a hyphen in
    /// the name spelled `_`.
    pub(crate) enables: &'static str,
    pub(crate) detect: Detection,
    /// The item's own site, for a reader who wants to know what it is
    /// before installing it. `None` where there is no one site: a
    /// platform's shell, and this repository's own connector.
    pub(crate) homepage: Option<&'static str>,
    pub(crate) recipe: PerPlatform<Recipe>,
    /// The file in this repository that pins the item's version, if any.
    pub(crate) pin: Pin,
    /// Where the item's newest release is read, or why it cannot be.
    pub(crate) upstream: Upstream,
    /// The row a page draws the item inside (`crates/sprawling/spec/Doctor.lean` §8-58).
    pub(crate) pack: Option<Pack>,
}

/// One item, and this machine's answer about it.
pub(crate) struct Finding {
    pub(crate) requirement: &'static Requirement,
    pub(crate) presence: Presence,
}

/// Asks this machine about every item in the table at once, and answers
/// in table order.
pub(crate) fn examine(machine: &dyn Machine) -> Vec<Finding> {
    match examine_each(machine, |_answered, _finding| {
        Ok::<(), std::convert::Infallible>(())
    }) {
        Ok(findings) => findings,
        Err(never) => match never {},
    }
}

/// Asks this machine about every item in the table at once, hands each
/// finding to `answered` with its table index the moment it arrives,
/// and returns every finding in table order.
///
/// One scoped thread per item, because each probe spends its time
/// waiting on a child process rather than on a core: the report then
/// costs the slowest item instead of the sum of all of them.
///
/// # Errors
/// The first failure `answered` returns; the probes still out finish
/// and their answers are dropped, because nobody is left to show them.
pub(crate) fn examine_each<E>(
    machine: &dyn Machine,
    mut answered: impl FnMut(usize, &Finding) -> Result<(), E>,
) -> Result<Vec<Finding>, E> {
    let (arrived, arrivals) = std::sync::mpsc::channel();
    std::thread::scope(|scope| {
        for (index, requirement) in REQUIREMENTS.iter().enumerate() {
            let arrived = arrived.clone();
            scope.spawn(move || {
                // A probe that stopped without answering is reported as
                // such, rather than taking the other rows down with it.
                let presence = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    machine.look(requirement)
                }))
                .unwrap_or_else(|_stopped| stopped_probe());
                arrived.send((index, requirement, presence))
            });
        }
        drop(arrived);
        let mut presences: Vec<Option<Presence>> = REQUIREMENTS.iter().map(|_| None).collect();
        for (index, requirement, presence) in arrivals {
            let finding = Finding {
                requirement,
                presence,
            };
            answered(index, &finding)?;
            if let Some(slot) = presences.get_mut(index) {
                *slot = Some(finding.presence);
            }
        }
        Ok(REQUIREMENTS
            .iter()
            .zip(presences)
            .map(|(requirement, presence)| Finding {
                requirement,
                presence: presence.unwrap_or_else(stopped_probe),
            })
            .collect())
    })
}

fn stopped_probe() -> Presence {
    Presence::Broken {
        at: std::path::PathBuf::new(),
        fault: Fault::Unreadable("its probe stopped before it answered".to_owned()),
    }
}

/// Whether one tier is reachable on this machine.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Verdict {
    Ready,
    /// The required items of this tier that are absent, in table order.
    Missing(Vec<&'static str>),
}

/// The verdict for one tier. An optional item that is absent never
/// stands between a person and a tier: it is reported and not counted.
/// A broken item counts as missing: a browser that will not start is
/// not a browser a city can use. A group is counted once, under its own
/// name and after the items named one by one, because a person missing
/// every Gecko browser and every driver is missing one thing rather
/// than four.
pub(crate) fn verdict(findings: &[Finding], tier: Tier) -> Verdict {
    let mut missing: Vec<&'static str> = Vec::new();
    let mut groups: Vec<(Group, bool)> = Vec::new();
    for finding in findings
        .iter()
        .filter(|finding| finding.requirement.tier == tier)
    {
        let usable = finding.presence.usable();
        match finding.requirement.need {
            Need::Optional => {}
            Need::Required => {
                if !usable {
                    missing.push(finding.requirement.name);
                }
            }
            Need::OneOf(group) => match groups.iter_mut().find(|(seen, _)| *seen == group) {
                Some(seen) => seen.1 = seen.1 || usable,
                None => groups.push((group, usable)),
            },
        }
    }
    missing.extend(
        groups
            .into_iter()
            .filter(|(_, satisfied)| !satisfied)
            .map(|(group, _)| group.as_str()),
    );
    if missing.is_empty() {
        Verdict::Ready
    } else {
        Verdict::Missing(missing)
    }
}

pub(crate) fn verdict_line(tier: Tier, verdict: &Verdict) -> String {
    match verdict {
        Verdict::Ready => format!("  ready to {}", tier.as_str()),
        Verdict::Missing(missing) => format!(
            "  not ready to {}: missing {}",
            tier.as_str(),
            missing.join(", ")
        ),
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
    reason = "test code"
)]
mod tests;
