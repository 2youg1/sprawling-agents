// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

pub(crate) mod asking;
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
mod scanning;
mod screen;
mod table;
mod upstream;
mod version_file;
mod visit;

pub(crate) use family::Family;
pub use host::usable_python;
pub(crate) use pack::Pack;
pub(crate) use pin::Pin;
pub(crate) use presence::{Absence, Fault, Presence, Version};
pub(crate) use probe::{Machine, ThisMachine, on_search_path};
pub(crate) use report::answer;
pub use screen::verb;
pub(crate) use table::{REQUIREMENTS, recipe_for};
pub(crate) use upstream::{Upstream, newest};

use accounting::Recipe;

pub(crate) const PATIENCE: std::time::Duration = std::time::Duration::from_secs(5);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Tier {
    Use,

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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Need {
    Required,

    OneOf(Group),

    Optional,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Group {
    BrowserEngine,
}

impl Group {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Group::BrowserEngine => "a browser engine",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Platform {
    Windows,
    MacOs,
    Linux,
}

impl Platform {
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

    pub(crate) const fn search_path_separator(self) -> char {
        match self {
            Platform::Windows => ';',
            Platform::MacOs | Platform::Linux => ':',
        }
    }
}

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

pub(crate) enum Detection {
    Program {
        program: &'static str,
        version_arg: &'static str,
        places: PerPlatform<&'static [&'static str]>,
    },

    Listed {
        program: &'static str,
        args: &'static [&'static str],
        line: &'static str,
    },

    #[expect(
        dead_code,
        reason = "kept for probe coverage of component installs that are not currently required"
    )]
    Component {
        variable: &'static str,
        file: &'static str,
    },

    Interpreter {
        variable: PerPlatform<&'static str>,
        fallback: PerPlatform<&'static str>,
    },

    #[expect(
        dead_code,
        reason = "kept for probe coverage of build-carried tools that are not currently required"
    )]
    Built {
        carried: bool,
    },

    Family(Family),

    Font {
        files: &'static [&'static str],
    },
}

pub(crate) struct Requirement {
    pub(crate) name: &'static str,
    pub(crate) tier: Tier,
    pub(crate) need: Need,

    pub(crate) enables: &'static str,
    pub(crate) detect: Detection,

    pub(crate) homepage: Option<&'static str>,
    pub(crate) recipe: PerPlatform<Recipe>,

    pub(crate) pin: Pin,

    pub(crate) upstream: Upstream,

    pub(crate) pack: Option<Pack>,
}

pub(crate) struct Finding {
    pub(crate) requirement: &'static Requirement,
    pub(crate) presence: Presence,
}

pub(crate) fn examine(machine: &dyn Machine) -> Vec<Finding> {
    match examine_each(machine, |_answered, _finding| {
        Ok::<(), std::convert::Infallible>(())
    }) {
        Ok(findings) => findings,
        Err(never) => match never {},
    }
}

pub(crate) fn examine_each<E>(
    machine: &dyn Machine,
    mut answered: impl FnMut(usize, &Finding) -> Result<(), E>,
) -> Result<Vec<Finding>, E> {
    let (arrived, arrivals) = std::sync::mpsc::channel();
    std::thread::scope(|scope| {
        for (index, requirement) in REQUIREMENTS.iter().enumerate() {
            let arrived = arrived.clone();
            scope.spawn(move || {
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

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Verdict {
    Ready,

    Missing(Vec<&'static str>),
}

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
