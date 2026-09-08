// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The verb that asks a commit which run wrote it.
//!
//! Its own file rather than one more arm of `data`: every verb there
//! moves bytes or verifies a chain, while this one reads one answer out
//! of the city's history and renders it for a person to read. `data`
//! also stands at its length budget, so a verb of this size arriving
//! there would have been paid for by shortening something else.

use super::city::report;
use sprawling::ask;
use std::process::ExitCode;

/// Which run wrote one commit, answered from the city's own history.
///
/// Git is not asked. The trailers on the commit are a projection of the
/// Ledger, and reading them back would make the projection the
/// authority; a city exported and restored with no `.git` beside it
/// answers here just the same.
///
/// Exit codes: 0 answered, 1 this city never wrote that commit, 2 this
/// command line. "Never wrote it" is a failure rather than a silent
/// success, because a script asking whether a line came from a machine
/// would otherwise read "I do not know" as "it did not".
pub(super) fn verb(city: Option<&String>, oid: Option<&String>) -> ExitCode {
    let (Some(city), Some(raw)) = (city, oid) else {
        eprintln!("usage: sprawling whose <city-dir> <commit-oid>");
        return ExitCode::from(2);
    };
    let Some(oid) = kernel::GitOid::parse(raw) else {
        eprintln!("not a commit id: {raw}");
        eprintln!("recovery: give all forty lowercase hex digits, not an abbreviation");
        return ExitCode::from(2);
    };
    match ask(std::path::Path::new(city), &channels::Query::Commit { oid }) {
        Ok(channels::Answer::Commit(said)) => {
            let session = said
                .session
                .map_or_else(|| "none".to_owned(), |n| n.to_string());
            let model = if said.model.is_empty() {
                "not recorded"
            } else {
                said.model.as_str()
            };
            println!("run     {}", said.run);
            println!("actor   {} (session {session})", said.actor.as_str());
            println!(
                "model   {model} (effort {})",
                memory::effort_word(said.effort)
            );
            println!("ledger  seq {}", said.seq.value());
            for before in said.lineage.iter().skip(1) {
                println!("replaced {before}");
            }
            ExitCode::SUCCESS
        }
        Ok(_) => {
            eprintln!("this city has no record of writing {oid}");
            eprintln!("recovery: the commit may be a person's own, or from another city");
            ExitCode::FAILURE
        }
        Err(err) => report(err),
    }
}
