// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The verb that asks a commit which run wrote it, and with `--trace`
//! which calls came before it (`crates/sprawling/Spec.lean` §8-167, §8-136).
//!
//! Its own file rather than one more arm of `data`: every verb there
//! moves bytes or verifies a chain, while this one reads one answer out
//! of the city's history and renders it for a person to read. `data`
//! also stands at its length budget, so a verb of this size arriving
//! there would have been paid for by shortening something else.

use super::city::report;
use super::grammar::Arguments;
use accounting::trace::{Trace, trace};
use accounting::views::ask;
use std::io::{ErrorKind, Write};
use std::path::Path;
use std::process::ExitCode;

/// What the city's history said about one commit.
enum Said {
    Commit(wire::CommitAnswer),
    Traced(Trace),
}

/// Which run wrote one commit, answered from the city's own history,
/// and with `--trace` the calls that run made since its previous commit.
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
pub(super) fn verb(read: &Arguments) -> ExitCode {
    let (Some(city), Some(raw)) = (read.positional(1), read.positional(2)) else {
        eprintln!("usage: sprawling whose <city-dir> <commit-oid> [--trace]");
        return ExitCode::from(2);
    };
    let Some(oid) = kernel::GitOid::parse(raw) else {
        eprintln!("not a commit id: {raw}");
        eprintln!("recovery: give all forty lowercase hex digits, not an abbreviation");
        return ExitCode::from(2);
    };
    let city = Path::new(city);
    let found = if read.has("--trace") {
        trace(city, oid).map(|traced| traced.map(Said::Traced))
    } else {
        ask(city, &wire::Query::Commit { oid }).map(|answer| {
            if let wire::Answer::Commit(said) = answer {
                Some(Said::Commit(*said))
            } else {
                None
            }
        })
    };
    match found {
        Ok(Some(said)) => written(&said),
        Ok(None) => {
            eprintln!("this city has no record of writing {oid}");
            eprintln!("recovery: the commit may be the User's own, or from another city");
            ExitCode::FAILURE
        }
        Err(err) => report(err),
    }
}

/// Writes `said` to stdout. A reader that stopped early (`| head`) has
/// what it asked for.
fn written(said: &Said) -> ExitCode {
    let mut out = std::io::stdout().lock();
    let wrote = match said {
        Said::Commit(commit) => write_commit(commit, &mut out),
        Said::Traced(traced) => write_trace(traced, &mut out),
    };
    match wrote.and_then(|()| out.flush()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) if err.kind() == ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("sprawling: whose: stdout: {err}");
            ExitCode::FAILURE
        }
    }
}

/// The run, the resident, the model, where the commit's line sits, the
/// runs it replaced, and the commit the same run announced before it.
fn write_commit(said: &wire::CommitAnswer, out: &mut impl Write) -> std::io::Result<()> {
    let session = said
        .session
        .as_ref()
        .map_or_else(|| "none".to_owned(), ToString::to_string);
    let model = if said.model.is_empty() {
        "not recorded"
    } else {
        said.model.as_str()
    };
    writeln!(out, "run     {}", said.run)?;
    writeln!(out, "actor   {} (session {session})", said.actor.as_str())?;
    writeln!(
        out,
        "model   {model} (effort {})",
        storage::effort_word(storage::recorded_effort(said.effort))
    )?;
    writeln!(out, "ledger  seq {}", said.seq.value())?;
    for before in said.lineage.iter().skip(1) {
        writeln!(out, "replaced {before}")?;
    }
    match said.previous {
        Some(previous) => writeln!(
            out,
            "previous {} at seq {}",
            previous.oid,
            previous.seq.value()
        ),
        None => writeln!(out, "previous none, the run's first commit"),
    }
}

/// The commit, the span it is traced over, one line for each of its
/// run's calls in the span, and who else called tools in the building.
pub(super) fn write_trace(traced: &Trace, out: &mut impl Write) -> std::io::Result<()> {
    write_commit(&traced.commit, out)?;
    let before = traced.commit.seq.value();
    match traced.commit.previous {
        Some(previous) => writeln!(
            out,
            "span    after seq {}, before seq {before}",
            previous.seq.value()
        )?,
        None => writeln!(
            out,
            "span    from the run's first line, before seq {before}"
        )?,
    }
    if traced.calls.is_empty() {
        writeln!(out, "call    none")?;
    }
    for call in &traced.calls {
        write_call(call, out)?;
    }
    for nearby in &traced.nearby {
        writeln!(
            out,
            "nearby  {} at {}: {} call(s)",
            nearby.run,
            nearby.actor.as_str(),
            nearby.calls
        )?;
    }
    if !traced.nearby.is_empty() {
        writeln!(
            out,
            "note    the calls are candidates; a nearby run may have written in the same span"
        )?;
    }
    Ok(())
}

/// One call: where it sits, when it was called, the tool, what the tool
/// was registered as crossing, how it ended and what it acted on; then
/// the first line it answered with.
fn write_call(call: &wire::Call, out: &mut impl Write) -> std::io::Result<()> {
    let effect = match &call.effect {
        Some(effect) => spelled(effect)?,
        None => "unregistered".to_owned(),
    };
    let what = call
        .subject
        .clone()
        .or_else(|| {
            call.arguments
                .as_ref()
                .and_then(|args| first_line(&args.head))
        })
        .unwrap_or_else(|| "-".to_owned());
    writeln!(
        out,
        "call    seq {}  {}  {}  {effect}  {}  {what}",
        call.at.value(),
        runtime::clock::iso(call.called),
        call.tool,
        spelled(&call.outcome)?
    )?;
    if let Some(answered) = call
        .output
        .as_ref()
        .and_then(|output| first_line(&output.head))
    {
        writeln!(out, "        > {answered}")?;
    }
    Ok(())
}

/// A value as its serde spelling writes it: the bare word for a variant
/// that carries nothing, compact JSON for one that does. The spelling
/// stays the type's own rather than a second one written here.
fn spelled(value: &impl serde::Serialize) -> std::io::Result<String> {
    let value = serde_json::to_value(value).map_err(std::io::Error::from)?;
    Ok(value
        .as_str()
        .map_or_else(|| value.to_string(), str::to_owned))
}

fn first_line(text: &str) -> Option<String> {
    text.lines()
        .next()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
}
