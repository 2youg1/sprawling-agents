// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a person reads, and what they are asked (sprawling-SPEC.md
//! section 8-40).
//!
//! The report is one line per item and one verdict per tier. A city
//! named on the line adds one line per building that asked for what
//! this machine lacks; `--explain <code>` answers one code instead.
//! `--install` adds one question per absent item, in table order, each
//! with the command it would run printed above it: nobody agrees to a
//! command they were not shown, and nothing this city was not asked
//! about runs.

use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use std::ffi::OsString;

use super::explain::{Explanation, explain, explanation_lines};
use super::needs::{lack_line, lacks};
use super::paint::{Ink, Part, row, summary};
use super::visit::{Visited, unreadable_line, visit};
use super::{
    Finding, Machine, PATIENCE, Platform, ThisMachine, Tier, Verdict, examine, verdict,
    verdict_line,
};

/// What the command line asked for. The default checks and changes
/// nothing.
pub(crate) struct Asked {
    pub(crate) install: bool,
    /// A city to judge building by building.
    pub(crate) city: Option<PathBuf>,
    /// A refusal code to connect to this machine, instead of a report.
    pub(crate) explain: Option<String>,
    /// Whether this report may use colour.
    pub(crate) ink: Ink,
}

/// What the command line asked of this verb, with the one environment
/// variable a terminal answers about colour. Checking is the default;
/// touching the machine takes the flag; the one word that is not a flag
/// and not a flag's value is the city.
pub(crate) fn asked(args: &[String], no_color: Option<OsString>) -> Asked {
    let mut install = false;
    let mut city = None;
    let mut explain = None;
    let mut ink = Ink::Colour;
    let mut words = args.iter().skip(1);
    while let Some(word) = words.next() {
        match word.as_str() {
            "--install" => install = true,
            "--explain" => explain = words.next().cloned(),
            "--no-color" => ink = Ink::Plain,
            flag if flag.starts_with("--") => {}
            path => city = Some(PathBuf::from(path)),
        }
    }
    Asked {
        install,
        city,
        explain,
        ink: ink.or_plain(no_color),
    }
}

/// The `doctor` verb. Exits with failure when a required item is
/// missing, or when a building of the named city lacks what it asked
/// for, so a script driving this learns the outcome from the exit code
/// rather than by reading the report.
pub fn verb(args: &[String]) -> ExitCode {
    let asked = asked(args, std::env::var_os("NO_COLOR"));
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
/// at a time. Answers whether every required item is present at the
/// end, and every named building has what it asked for, which is what
/// the caller turns into an exit code. An explanation always answers
/// true: it is an explanation, not a verdict.
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
    if let Some(code) = &asked.explain {
        let explanation = explain(code, &findings, Platform::current());
        writeln!(out)?;
        for line in explanation_lines(code, &explanation) {
            writeln!(out, "{line}")?;
        }
        writeln!(out)?;
        return Ok(!matches!(explanation, Explanation::NoSuchCode(_)));
    }
    let platform = Platform::current().map_or("this platform", Platform::as_str);
    writeln!(
        out,
        "
  this machine ({platform}), against what this city needs:
"
    )?;
    for part in Part::ALL {
        writeln!(out, "{}\n", part.heading())?;
        for finding in findings.iter().filter(|found| Part::of(found) == part) {
            writeln!(out, "{}", row(finding, asked.ink))?;
        }
        writeln!(out)?;
    }
    let mut ready = true;
    for tier in Tier::ALL {
        let verdict = verdict(&findings, tier);
        ready = ready && verdict == Verdict::Ready;
        writeln!(out, "{}", verdict_line(tier, &verdict))?;
    }
    for line in summary(&findings, asked.ink) {
        writeln!(out, "{line}")?;
    }
    if let Some(city) = &asked.city {
        ready = report_city(city, &findings, out)? && ready;
    }
    if asked.install {
        offer(&findings, machine, input, out)?;
    }
    writeln!(out)?;
    Ok(ready)
}

/// One line per building that asked for what this machine lacks, and
/// one per building whose rules will not read. Answers whether every
/// building has what it asked for.
fn report_city<W: Write>(
    city: &std::path::Path,
    findings: &[Finding],
    out: &mut W,
) -> std::io::Result<bool> {
    writeln!(
        out,
        "
  the city at {}, building by building:
",
        city.display()
    )?;
    let visited = match visit(city) {
        Ok(visited) => visited,
        Err(err) => {
            writeln!(out, "  {err}")?;
            writeln!(out, "  recovery: {}", err.recovery())?;
            return Ok(false);
        }
    };
    let mut whole = true;
    for seen in &visited {
        match seen {
            Visited::Bits { building, bits } => {
                for lack in lacks(building, bits, findings) {
                    whole = false;
                    writeln!(out, "{}", lack_line(&lack))?;
                }
            }
            Visited::Unreadable { building, err } => {
                whole = false;
                writeln!(out, "{}", unreadable_line(building, err))?;
            }
        }
    }
    if whole {
        writeln!(out, "  every building has what it asked for")?;
    }
    Ok(whole)
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
        if finding.presence.usable() {
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
