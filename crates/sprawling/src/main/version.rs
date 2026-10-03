// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What `status` says about which release this is, and the one command
//! in this binary that asks the registry whether a newer one exists.
//!
//! Its own file rather than another pair of functions under `status`:
//! everything else `status` prints is read out of this binary, and this
//! is the part that leaves the machine. The reading itself belongs to
//! `bin::release`; this renders it for a terminal, as the machine page
//! renders the same value for a browser.

use kernel::Maturity;
use sprawling::release;
use sprawling::release::Built;
use std::process::ExitCode;
use wire::{Registry, RegistryNewest, RegistryReading, ReleaseAnswer};

/// The first line `status` prints, as in
/// `sprawling 0.0.8 (pre-alpha), built from source`.
///
/// The production path passes `kernel::release::MATURITY`, the one place
/// the maturity is written (kernel D18); taking it as a parameter is what
/// lets a test see this line follow it (`crates/sprawling/spec/Doctor.lean` §8-162).
pub(super) fn headline(maturity: Maturity) -> String {
    format!(
        "sprawling {} ({}){}",
        env!("CARGO_PKG_VERSION"),
        maturity.word(),
        cut()
    )
}

/// The day this release was cut, appended to the version line.
///
/// An early version number says almost nothing about how old a tree is,
/// and how old it is, is what its reader most needs to know
/// (CHANGELOG.md, opening note) - so the date sits beside the number in
/// the line a person reads first, and reading it costs no network.
fn cut() -> String {
    match release::built() {
        Ok(Built::Released(mine)) => format!(", released {}", mine.released()),
        Ok(Built::FromSource) => String::from(", built from source"),
        // A build that carries a tag it cannot decode is mislabelled.
        // Saying so here is cheaper than letting every later line
        // describe a release this binary is not.
        Err(err) => format!(", carrying a tag that does not decode: {}", err.recovery()),
    }
}

/// The one command in this binary that reaches the network, and only
/// when `--check` asked it to.
///
/// The exit code reports whether the question was answered, never what
/// the answer was: a release being out of date is news, not a failure,
/// while a registry that could not be read leaves a person believing
/// they checked when they did not.
pub(super) fn check() -> ExitCode {
    println!();
    match release::answer() {
        ReleaseAnswer::Refused { refusal } => {
            eprintln!("could not check: {refusal}");
            eprintln!("recovery: {}", refusal.recovery());
            ExitCode::FAILURE
        }
        ReleaseAnswer::Unreleased { registries, .. } => {
            println!("this binary was built from source, so it is none of the published releases.");
            for line in &registries {
                println!("{}", registry_said(line));
            }
            ExitCode::SUCCESS
        }
        ReleaseAnswer::Stands {
            mine,
            registries,
            verdict,
            update,
        } => {
            let Some(newest) = registries.iter().find_map(|line| match &line.reading {
                RegistryReading::Read { newest } if line.registry == Registry::Npm => Some(newest),
                RegistryReading::Read { .. }
                | RegistryReading::Refused { .. }
                | RegistryReading::Unasked => None,
            }) else {
                eprintln!("could not check: the answer carried no npm reading");
                return ExitCode::FAILURE;
            };
            match verdict {
                kernel::ReleaseVerdict::Current => println!(
                    "{} is the newest release published. Nothing to do.",
                    mine.version
                ),
                kernel::ReleaseVerdict::Behind => {
                    println!(
                        "a newer release is published: {}, cut {} (this binary is {}).",
                        newest.version, newest.released, mine.version
                    );
                    println!();
                    if let Some(command) = &update.command {
                        println!("  npm      {command}");
                    }
                    println!("  archive  download it, then run `sprawling install` again:");
                    println!("           https://github.com/2youg1/sprawling-agents/releases");
                    println!();
                    println!(
                        "Nothing was downloaded and nothing was changed. \
                         Updating is yours to run."
                    );
                }
                // The release page is published before the npm packages,
                // so this is what a download looks like for as long as
                // that job takes rather than a state to worry about.
                kernel::ReleaseVerdict::Ahead => println!(
                    "this binary ({}) is newer than the newest on npm ({}); \
                     the packages are published after the release page.",
                    mine.version, newest.version
                ),
            }
            ExitCode::SUCCESS
        }
    }
}

/// One registry's line, as the terminal prints it.
fn registry_said(line: &RegistryNewest) -> String {
    let registry = match line.registry {
        Registry::Npm => "npm",
        Registry::CratesIo => "crates.io",
    };
    match &line.reading {
        RegistryReading::Read { newest } => format!(
            "newest on {registry}: {}, cut {}",
            newest.version, newest.released
        ),
        RegistryReading::Refused { refusal } => format!("{registry} could not be read: {refusal}"),
        RegistryReading::Unasked => format!("{registry} was not asked"),
    }
}

#[cfg(test)]
mod tests {
    use super::{cut, headline};
    use kernel::Maturity;

    /// Entering alpha moves `kernel::release::MATURITY` and nothing else,
    /// so the line a person reads first has to say what it is given.
    #[test]
    fn the_status_line_says_the_maturity_it_is_given() {
        assert_eq!(
            headline(Maturity::Alpha),
            format!("sprawling {} (alpha){}", env!("CARGO_PKG_VERSION"), cut())
        );
    }
}
