// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What this machine has, what this city needs, and what installing the
//! difference would cost (sprawling-SPEC.md section 8-40).
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

mod explain;
pub(crate) mod host;
mod needs;
mod presence;
mod probe;
mod report;
mod screen;
mod table;
mod visit;

pub(crate) use presence::{Absence, Fault, Presence, Version};
pub(crate) use probe::{Machine, ThisMachine};
pub(crate) use report::report;
pub use screen::verb;
pub(crate) use table::REQUIREMENTS;

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
    /// Absent is a fact rather than a fault; `enables` says what having
    /// it would add.
    Optional,
}

/// The three platforms this project is built for. `None` from `current`
/// is an honest answer: on a fourth platform nothing here can name a
/// package manager, so `--install` prints and stops.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Platform {
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

    pub(crate) fn current() -> Option<Platform> {
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
}

/// What installing this item costs on one platform.
pub(crate) enum Recipe {
    /// A command this machine may run, once the person has agreed to it.
    /// Every one of them is a per-user install; none asks for elevation.
    Command {
        program: &'static str,
        args: &'static [&'static str],
    },
    /// A command printed and never run. A script piped into a shell is
    /// code nobody read, so this city prints it and the person decides.
    Print(&'static str),
    /// Nothing here can install it; the line says what a person does.
    Manual(&'static str),
}

impl Recipe {
    /// The command as a person would type it, or the manual instruction.
    pub(crate) fn spelled(&self) -> String {
        match self {
            Recipe::Command { program, args } => {
                if args.is_empty() {
                    (*program).to_owned()
                } else {
                    format!("{program} {}", args.join(" "))
                }
            }
            Recipe::Print(line) => (*line).to_owned(),
            Recipe::Manual(how) => format!("manual: {how}"),
        }
    }

    /// Whether this city may run it after a person says yes.
    pub(crate) fn runnable(&self) -> bool {
        matches!(self, Recipe::Command { .. })
    }
}

/// One thing this city needs, and everything that can be said about it
/// without touching the machine.
pub(crate) struct Requirement {
    pub(crate) name: &'static str,
    pub(crate) tier: Tier,
    pub(crate) need: Need,
    /// What having it lets a person do, in one clause.
    pub(crate) enables: &'static str,
    pub(crate) detect: Detection,
    pub(crate) recipe: PerPlatform<Recipe>,
}

/// One item, and this machine's answer about it.
pub(crate) struct Finding {
    pub(crate) requirement: &'static Requirement,
    pub(crate) presence: Presence,
}

/// Asks this machine about every item in the table, in table order.
pub(crate) fn examine(machine: &dyn Machine) -> Vec<Finding> {
    REQUIREMENTS
        .iter()
        .map(|requirement| Finding {
            requirement,
            presence: machine.look(requirement),
        })
        .collect()
}

/// The one line a person reads about one item.
pub(crate) fn finding_line(finding: &Finding) -> String {
    let name = finding.requirement.name;
    let said = finding.presence.describe();
    match (&finding.presence, finding.requirement.need) {
        (Presence::Present { .. } | Presence::Broken { .. }, _) => {
            format!("  {name:<18} {said}")
        }
        (Presence::Absent(_), Need::Required) => format!("  {name:<18} {said}"),
        (Presence::Absent(_), Need::Optional) => format!(
            "  {name:<18} optional-{said} - it enables {}",
            finding.requirement.enables
        ),
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
/// not a browser a city can use.
pub(crate) fn verdict(findings: &[Finding], tier: Tier) -> Verdict {
    let missing: Vec<&'static str> = findings
        .iter()
        .filter(|finding| {
            finding.requirement.tier == tier
                && finding.requirement.need == Need::Required
                && !finding.presence.usable()
        })
        .map(|finding| finding.requirement.name)
        .collect();
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
    reason = "test code"
)]
mod tests;
