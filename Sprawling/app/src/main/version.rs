// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::Maturity;
use sprawling::release;
use sprawling::release::Built;
use std::process::ExitCode;
use wire::{Registry, RegistryNewest, RegistryReading, ReleaseAnswer};

pub(super) fn headline(maturity: Maturity) -> String {
    format!(
        "sprawling {} ({}){}",
        env!("CARGO_PKG_VERSION"),
        maturity.word(),
        cut()
    )
}

fn cut() -> String {
    match release::built() {
        Ok(Built::Released(mine)) => format!(", released {}", mine.released()),
        Ok(Built::Nightly(mine)) => format!(", {}", mine.tag()),
        Ok(Built::FromSource) => String::from(", built from source"),

        Err(err) => format!(", carrying a tag that does not decode: {}", err.recovery()),
    }
}

pub(super) fn check() -> ExitCode {
    println!();
    match release::answer() {
        ReleaseAnswer::Refused { refusal } => {
            eprintln!("could not check: {refusal}");
            eprintln!("recovery: {}", refusal.recovery());
            ExitCode::FAILURE
        }
        ReleaseAnswer::Unconfirmed {
            mine,
            registries,
            update,
        } => {
            println!(
                "{}: confirm the original installer before updating.",
                mine.version
            );
            for line in &registries {
                println!("{}", registry_said(line));
            }
            print_update(&update);
            ExitCode::SUCCESS
        }
        ReleaseAnswer::Unreleased { registries, update } => {
            println!("this binary was built from source, so it is none of the published releases.");
            for line in &registries {
                println!("{}", registry_said(line));
            }
            print_update(&update);
            ExitCode::SUCCESS
        }
        ReleaseAnswer::Stands {
            mine,
            newest,
            registries: _,
            verdict,
            update,
        } => {
            match verdict {
                kernel::ReleaseVerdict::Current => println!(
                    "{} is the newest release published. Nothing to do.",
                    mine.version
                ),
                kernel::ReleaseVerdict::Behind => {
                    println!(
                        "a newer release is published: {}{} (this binary is {}).",
                        newest.version,
                        date_suffix(&newest),
                        mine.version
                    );
                    println!();
                    print_update(&update);
                    println!();
                    println!(
                        "Nothing was downloaded and nothing was changed. \
                         Updating is yours to run."
                    );
                }
                kernel::ReleaseVerdict::Ahead => println!(
                    "this binary ({}) is newer than the newest available through its install channel ({}).",
                    mine.version, newest.version
                ),
            }
            ExitCode::SUCCESS
        }
    }
}

fn print_update(update: &wire::UpdateHint) {
    if let Some(command) = &update.command {
        println!("  update  {command}");
    }
    for command in &update.alternatives {
        println!("  choose  {command}");
    }
}

fn date_suffix(line: &wire::ReleaseLine) -> String {
    if line.released.is_empty() {
        String::new()
    } else {
        format!(", cut {}", line.released)
    }
}

fn registry_said(line: &RegistryNewest) -> String {
    let registry = match line.registry {
        Registry::Npm => "npm",
        Registry::CratesIo => "crates.io",
        Registry::Github => "GitHub releases",
    };
    match &line.reading {
        RegistryReading::Read { newest } => format!(
            "newest on {registry}: {}{}",
            newest.version,
            date_suffix(newest)
        ),
        RegistryReading::Refused { refusal } => format!("{registry} could not be read: {refusal}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{cut, headline};
    use kernel::Maturity;

    #[test]
    fn the_status_line_says_the_maturity_it_is_given() {
        assert_eq!(
            headline(Maturity::Alpha),
            format!("sprawling {} (alpha){}", env!("CARGO_PKG_VERSION"), cut())
        );
    }
}
