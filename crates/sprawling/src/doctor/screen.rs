// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a person reads, and what they are asked (sprawling-SPEC.md
//! section 8-40).
//!
//! The report is one line per item and one verdict per tier. `--install`
//! adds one question per absent item, in table order, each with the
//! command it would run printed above it: nobody agrees to a command
//! they were not shown, and nothing this city was not asked about runs.

use std::io::{BufRead, Write};
use std::process::ExitCode;
use std::time::Duration;

use super::{
    Finding, Machine, Platform, Presence, ThisMachine, Tier, Verdict, examine, finding_line,
    verdict, verdict_line,
};

/// What the command line asked for. The default checks and changes
/// nothing.
pub(crate) struct Asked {
    pub(crate) install: bool,
}

/// How long one `--version` call may take before this city stops
/// waiting for it. Generous enough for a cold start on a slow disk,
/// short enough that nine of them never feel like a hang.
const PATIENCE: Duration = Duration::from_secs(5);

/// What the command line asked of this verb. Checking is the default;
/// touching the machine takes the flag.
pub(crate) fn asked(args: &[String]) -> Asked {
    Asked {
        install: args.iter().any(|arg| arg == "--install"),
    }
}

/// The `doctor` verb. Exits with failure when a required item is
/// missing, so a script driving this learns the outcome from the exit
/// code rather than by reading the report.
pub(crate) fn verb(args: &[String]) -> ExitCode {
    let asked = asked(args);
    let machine = ThisMachine::new(Platform::current(), PATIENCE);
    let answered = run(
        &asked,
        &machine,
        &mut std::io::stdin().lock(),
        &mut std::io::stdout().lock(),
    );
    match answered {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(err) => {
            eprintln!("could not report on this machine: {err}");
            ExitCode::FAILURE
        }
    }
}

/// Reports this machine, and - when asked - offers each absent item one
/// at a time. Answers whether every required item is present at the end,
/// which is what the caller turns into an exit code.
///
/// # Errors
/// Propagates what the terminal reports: a report nobody can be shown,
/// or an answer that cannot be read, is not an answer to guess at.
pub(crate) fn run<R: BufRead, W: Write>(
    asked: &Asked,
    machine: &dyn Machine,
    input: &mut R,
    out: &mut W,
) -> std::io::Result<bool> {
    let findings = examine(machine);
    let platform = Platform::current().map_or("this platform", Platform::as_str);
    writeln!(
        out,
        "\n  this machine ({platform}), against what this city needs:\n"
    )?;
    for finding in &findings {
        writeln!(out, "{}", finding_line(finding))?;
    }
    writeln!(out)?;
    let mut ready = true;
    for tier in Tier::ALL {
        let verdict = verdict(&findings, tier);
        ready = ready && verdict == Verdict::Ready;
        writeln!(out, "{}", verdict_line(tier, &verdict))?;
    }
    if asked.install {
        offer(&findings, machine, input, out)?;
    }
    writeln!(out)?;
    Ok(ready)
}

/// Offers every absent item, one question at a time, and reports what
/// was installed. Nothing runs that was not asked about and answered
/// `y`, and nothing runs with elevation.
fn offer<R: BufRead, W: Write>(
    findings: &[Finding],
    machine: &dyn Machine,
    input: &mut R,
    out: &mut W,
) -> std::io::Result<()> {
    let Some(platform) = Platform::current() else {
        writeln!(
            out,
            "\n  this platform has no package manager here; install by hand:"
        )?;
        return Ok(());
    };
    let mut installed: Vec<&'static str> = Vec::new();
    for finding in findings {
        if finding.presence != Presence::Absent {
            continue;
        }
        let name = finding.requirement.name;
        let recipe = finding.requirement.recipe.at(platform);
        writeln!(out, "\n  {name}: {}", recipe.spelled())?;
        if !recipe.runnable() {
            writeln!(out, "  this one is yours to run; nothing was asked")?;
            continue;
        }
        write!(out, "  run it? [y/N] ")?;
        out.flush()?;
        let mut answer = String::new();
        // End of input is not consent: an unattended stdin has nobody to
        // ask, so the honest reading of silence is no.
        if input.read_line(&mut answer)? == 0 {
            writeln!(out, "\n  nobody answered; nothing was installed")?;
            break;
        }
        if !answer.trim().eq_ignore_ascii_case("y") {
            writeln!(out, "  skipped")?;
            continue;
        }
        match machine.install(name, recipe) {
            Ok(()) => installed.push(name),
            Err(err) => {
                writeln!(out, "  {err}")?;
                writeln!(out, "  recovery: {}", err.recovery())?;
            }
        }
    }
    if installed.is_empty() {
        writeln!(out, "\n  nothing was installed")?;
    } else {
        writeln!(out, "\n  installed: {}", installed.join(", "))?;
        writeln!(out, "  open a NEW shell window before checking again")?;
    }
    Ok(())
}
