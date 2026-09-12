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

use channels::ReleaseAnswer;
use sprawling::release;
use sprawling::release::Built;
use std::process::ExitCode;

/// The day this release was cut, appended to the version line.
///
/// A pre-alpha version number says almost nothing about how old a tree
/// is, and how old it is, is what its reader most needs to know
/// (CHANGELOG.md, opening note) - so the date sits beside the number in
/// the line a person reads first, and reading it costs no network.
pub(super) fn cut() -> String {
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
        ReleaseAnswer::Unreleased { newest } => {
            println!("this binary was built from source, so it is none of the published releases.");
            println!(
                "newest published: {}, cut {}",
                newest.version, newest.released
            );
            ExitCode::SUCCESS
        }
        ReleaseAnswer::Stands {
            mine,
            newest,
            verdict,
        } => {
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
                    println!("  npm      bunx sprawling@latest up");
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
