// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a person reads, and what they are asked (`crates/sprawling/spec/Accounting/Worker.lean`
//! §8-40).
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

use kernel::AxError;

use crate::serving::standing::{Held, Standing};

use super::explain::{Explanation, explain, explanation_lines};
use super::needs::{lack_line, lacks};
use super::paint::{Ink, Part, row, summary};
use super::visit::{Visited, unreadable_line, visit};
use super::{
    Finding, Machine, PATIENCE, Platform, REQUIREMENTS, ThisMachine, Tier, Verdict, examine,
    examine_each, verdict, verdict_line,
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
    if let Some(code) = &asked.explain {
        let findings = examine(machine);
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
        "\n  this machine ({platform}), against what this city needs:\n"
    )?;
    // The heading is on the screen before the probes start, so a person
    // waiting on a slow tool is not looking at an empty terminal.
    out.flush()?;
    let mut rows = Rows::new(out, asked.ink);
    let findings = examine_each(machine, |index, finding| rows.answered(index, finding))?;
    rows.close()?;
    for line in priority_lines(&machine.core_standing()) {
        writeln!(out, "{line}")?;
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

/// The two parts of the report, written row by row as the probes answer:
/// a row goes out once it and every row above it on the screen have
/// answered, so the layout is the same on every run however the answers
/// race (`crates/sprawling/spec/Doctor.lean` §8-59).
struct Rows<'o, W: Write> {
    out: &'o mut W,
    ink: Ink,
    /// For each table index, where its row stands on the screen.
    place: Vec<usize>,
    /// For each screen position, its part and, once answered, its row.
    lines: Vec<(Part, Option<String>)>,
    written: usize,
    /// How many parts have their heading on the screen.
    opened: usize,
}

impl<'o, W: Write> Rows<'o, W> {
    fn new(out: &'o mut W, ink: Ink) -> Self {
        let screen_order: Vec<(usize, Part)> = Part::ALL
            .into_iter()
            .flat_map(|part| {
                REQUIREMENTS
                    .iter()
                    .enumerate()
                    .filter(move |(_, requirement)| Part::of(requirement) == part)
                    .map(move |(index, _)| (index, part))
            })
            .collect();
        let mut place = vec![0; REQUIREMENTS.len()];
        for (position, (index, _)) in screen_order.iter().enumerate() {
            if let Some(slot) = place.get_mut(*index) {
                *slot = position;
            }
        }
        Rows {
            out,
            ink,
            place,
            lines: screen_order
                .into_iter()
                .map(|(_, part)| (part, None))
                .collect(),
            written: 0,
            opened: 0,
        }
    }

    fn answered(&mut self, index: usize, finding: &Finding) -> std::io::Result<()> {
        if let Some((_, line)) = self.place.get(index).and_then(|at| self.lines.get_mut(*at)) {
            *line = Some(row(finding, self.ink));
        }
        while let Some((part, Some(line))) = self.lines.get(self.written) {
            let (rank, line) = (rank(*part), line.clone());
            self.open_through(rank)?;
            writeln!(self.out, "{line}")?;
            self.written = self.written.saturating_add(1);
        }
        self.out.flush()
    }

    /// Writes the heading of every part up to the one ranked `rank`,
    /// closing the part before each with a blank line.
    fn open_through(&mut self, rank: usize) -> std::io::Result<()> {
        while self.opened <= rank {
            if self.opened > 0 {
                writeln!(self.out)?;
            }
            if let Some(part) = Part::ALL.get(self.opened) {
                writeln!(self.out, "{}\n", part.heading())?;
            }
            self.opened = self.opened.saturating_add(1);
        }
        Ok(())
    }

    /// Opens the parts no row reached, and closes the last one.
    fn close(mut self) -> std::io::Result<()> {
        self.open_through(Part::ALL.len().saturating_sub(1))?;
        writeln!(self.out)
    }
}

fn rank(part: Part) -> usize {
    match part {
        Part::Required => 0,
        Part::Recommended => 1,
    }
}

/// The part that says where this machine lets the core's threads stand.
/// A thread the valve lowered is a serving city's, never the doctor's,
/// but the arm keeps the words for every standing in this one place.
fn priority_lines(core: &Result<Standing, AxError>) -> [String; 4] {
    let level = match core {
        Ok(Standing::Raised) => "one step above normal".to_owned(),
        Ok(Standing::Normal(Held::ByTheSetting)) => {
            "normal, as config.toml [core] priority asks".to_owned()
        }
        Ok(Standing::Normal(Held::Refused(reason))) => {
            format!("normal, the platform refused: {reason}")
        }
        Ok(Standing::Normal(Held::ByTheValve)) => {
            "normal, lowered after keeping a core busy".to_owned()
        }
        Err(err) => format!("unknown: {err}"),
    };
    [
        "  priority - where the core's threads stand".to_owned(),
        String::new(),
        format!("    core threads    {level}"),
        String::new(),
    ]
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
        "\n  the city at {}, building by building:\n",
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
        // The one authority on whether this city may run a recipe also
        // holds the sentence saying what the person does instead.
        let runnable = match recipe.command(name) {
            Ok(runnable) => runnable,
            Err(refused) => {
                writeln!(out, "  {}", refused.recovery())?;
                continue;
            }
        };
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
        match accounting::Machine::install(machine, name, &runnable) {
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
