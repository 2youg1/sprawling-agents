// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `sprawling dispatch <addr> <task>`: one piece of work sent to a served
//! city, watched until its run freezes (sprawling-SPEC.md section 8-10).
//!
//! The frame is built here rather than typed by the caller, so nobody
//! transcribes the wire's key format, mode spelling or session rule; and
//! the wait ends on the run's own milestone rather than on a silence,
//! because how long a run takes is the model's business.

use super::city::report;
use super::grammar::Arguments;
use super::wire_client::{self, Ending, Milestone, Spoken};
use kernel::consts_policy::DEFAULT_AT;
use std::process::ExitCode;
use std::time::Duration;

/// How long a silence ends the wait when no `--quiet-ms` is given. A
/// running dispatch streams events, so this bounds a city that stopped
/// speaking, not a run that is slow.
const QUIET_MS: u64 = 2_000;

/// Sends the dispatch and prints what comes back; the exit code is the
/// table `call` uses: 0 answered, 1 refused, 2 this command line, 3
/// nothing came back.
pub(super) fn verb(read: &Arguments) -> ExitCode {
    let (Some(addr), Some(task)) = (read.positional(1), read.positional(2)) else {
        eprintln!("usage: sprawling dispatch <addr> <task> [--detach] [--at host:port]");
        return ExitCode::from(2);
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
            return ExitCode::from(2);
        }
    };
    let idem = match minted() {
        Ok(idem) => idem,
        Err(err) => return report(err),
    };
    let until = if read.has("--detach") {
        Milestone::Started
    } else {
        Milestone::Frozen
    };
    let frame = channels::ClientFrame::Command(Box::new(channels::WireCommand::Dispatch {
        addr: addr.clone(),
        task: task.clone(),
        goal: String::new(),
        mode: channels::Mode::PlanGoal,
        idem,
        session: None,
        effort: None,
    }));
    let ending = Ending::OnRun {
        under: addr,
        until,
        run: None,
    };
    let at = read.value("--at").unwrap_or(DEFAULT_AT);
    match wire_client::send(
        at,
        &frame,
        read.value("--token"),
        Duration::from_millis(quiet),
        ending,
    ) {
        Ok(heard) => {
            // Detached, stdout carries the run id alone; attached, it is
            // the JSONL of every frame, which a run id line would break.
            if let (Milestone::Started, Some(run)) = (until, heard.run) {
                println!("{run}");
            }
            match heard.spoken() {
                Spoken::Refused => ExitCode::FAILURE,
                Spoken::Answered => ExitCode::SUCCESS,
                Spoken::Quiet => {
                    eprintln!(
                        "nothing came back inside {quiet}ms: the city may still be working on it"
                    );
                    ExitCode::from(3)
                }
            }
        }
        Err(err) => report(err),
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
