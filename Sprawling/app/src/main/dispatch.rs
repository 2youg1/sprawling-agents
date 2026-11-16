// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `sprawling dispatch <addr> <task>`: one piece of work sent to a served
//! city, watched until its run freezes (`crates/sprawling/spec/WireClient.lean` §8-10).
//!
//! The frame is built here rather than typed by the caller, so nobody
//! transcribes the wire's key format, mode spelling or session rule; and
//! the wait ends on the run's own milestone rather than on a silence,
//! because how long a run takes is the model's business.

use super::calling::{exit_of, tell_unheard};
use super::city::report;
use super::exit::Exit;
use super::grammar::Arguments;
use super::refusal::Form;
use super::verbs::{self, Verb};
use super::wire_client::{self, Listen, Milestone, Spoken, Until};
use kernel::consts_policy::DEFAULT_AT;
use std::process::ExitCode;
use std::time::Duration;

/// How long a silence ends the wait when no `--quiet-ms` is given. One
/// model call streams nothing until it returns, and a dispatch to a
/// building makes one to name the room before its run starts, so the
/// window must outlast a slow call; it bounds a city that stopped
/// speaking, and a wait it ends before the milestone exits 3.
const QUIET_MS: u64 = 120_000;

/// Sends the dispatch and prints what comes back; the exit code is the
/// table `call` uses: 0 the run reached its milestone, 1 refused, 2 this
/// command line, 3 nothing came back or the run did not reach its
/// milestone, 4 no city at `--at`.
pub(super) fn verb(read: &Arguments) -> ExitCode {
    let (Some(addr), Some(task)) = (read.positional(1), read.positional(2)) else {
        if let Some(row) = verbs::row(Verb::Dispatch) {
            eprintln!("usage: {}", verbs::usage(row));
        }
        return Exit::Line.into();
    };
    let addr = match kernel::Address::parse(addr) {
        Ok(addr) => addr,
        Err(err) => return report(err),
    };
    let quiet = match read.value("--quiet-ms").map(str::parse::<u64>) {
        None => QUIET_MS,
        Some(Ok(ms)) => ms,
        Some(Err(_)) => {
            eprintln!("--quiet-ms takes a number of milliseconds");
            return Exit::Line.into();
        }
    };
    let idem = match minted() {
        Ok(idem) => idem,
        Err(err) => return report(err),
    };
    let milestone = if read.has("--detach") {
        Milestone::Started
    } else {
        Milestone::Frozen
    };
    let frame = wire::ClientFrame::Command(Box::new(wire::WireCommand::Dispatch {
        addr: addr.clone(),
        task: task.clone(),
        goal: String::new(),
        policy: wire::RunPolicy::of(wire::Mode::Work),
        idem,
        session: None,
        effort: None,
        model: read.value("--model").map(str::to_owned),
    }));
    let listen = Listen {
        quiet: Duration::from_millis(quiet),
        until: Until::Run {
            under: addr,
            milestone,
        },
    };
    let at = read.value("--at").unwrap_or(DEFAULT_AT);
    match wire_client::send(at, &frame, read.value("--token"), listen) {
        Ok(heard) => {
            // Detached, stdout carries the run id alone; attached, it is
            // the JSONL of every frame, which a run id line would break.
            if let (Milestone::Started, Some(run)) = (milestone, heard.run) {
                println!("{run}");
            }
            let spoken = heard.spoken();
            match spoken {
                Spoken::Refused | Spoken::Answered => {}
                Spoken::Quiet => eprintln!(
                    "nothing came back inside {quiet}ms: the city may still be working on it"
                ),
                Spoken::Unfinished => eprintln!(
                    "the run did not reach its milestone inside {quiet}ms of silence: it may still be working; read the city's own log"
                ),
            }
            exit_of(&spoken).into()
        }
        Err(unheard) => tell_unheard(&unheard, Form::Human).into(),
    }
}

/// The idempotency key for this one dispatch, from OS entropy: a key
/// derived from the words would make the same task typed twice one run.
fn minted() -> Result<kernel::IdemKey, kernel::AxError> {
    let mut origin = [0u8; 16];
    getrandom::fill(&mut origin).map_err(|err| {
        kernel::AxError::failure(
            kernel::AxCode::ConfigInvalid,
            "draw an idempotency key for this dispatch",
            err.to_string(),
        )
        .with_recovery("this machine's entropy source refused; dispatch from the WebUI instead")
    })?;
    Ok(kernel::IdemKey::derive(
        &kernel::RunId::CITY,
        kernel::Seq::FIRST,
        &origin,
    ))
}
