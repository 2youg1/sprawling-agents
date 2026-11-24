// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use std::ffi::OsString;

use kernel::AxError;

use crate::serving::standing::{Held, Standing};

use super::explain::{Explanation, explain, explanation_lines};
use super::needs::{lack_line, lacks};
use super::paint::{Ink, Part, row, summary};
use super::scanning::{self, Scanning};
use super::visit::{Visited, unreadable_line, visit};
use super::{
    Finding, Machine, PATIENCE, Platform, REQUIREMENTS, ThisMachine, Tier, Verdict, examine,
    examine_each, verdict, verdict_line,
};

pub(crate) struct Asked {
    pub(crate) install: bool,

    pub(crate) city: Option<PathBuf>,

    pub(crate) scanned: PathBuf,

    pub(crate) explain: Option<String>,

    pub(crate) ink: Ink,
}

pub(crate) fn asked(
    args: &[String],
    no_color: Option<OsString>,
    unnamed: fn() -> PathBuf,
) -> Asked {
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
        scanned: city.clone().unwrap_or_else(unnamed),
        city,
        explain,
        ink: ink.or_plain(no_color),
    }
}

pub fn verb(args: &[String], unnamed: fn() -> PathBuf) -> ExitCode {
    let mut asked = asked(args, std::env::var_os("NO_COLOR"), unnamed);
    if let Some(workspace) = &asked.city {
        match accounting::home::Home::detect().and_then(|home| home.workspace(workspace)) {
            Ok(city) => {
                asked.scanned.clone_from(&city);
                asked.city = Some(city);
            }
            Err(err) => {
                eprintln!("{err}");
                return ExitCode::FAILURE;
            }
        }
    }
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

    out.flush()?;
    let mut rows = Rows::new(out, asked.ink);

    let (findings, scanning) = std::thread::scope(|scope| {
        let scanning = scope.spawn(|| machine.scanning(&asked.scanned));
        let findings = examine_each(machine, |index, finding| rows.answered(index, finding));
        (findings, scanning.join().unwrap_or(Scanning::Stopped))
    });
    let findings = findings?;
    rows.close()?;
    for line in priority_lines(&machine.core_standing()) {
        writeln!(out, "{line}")?;
    }
    writeln!(out, "    {}\n", crate::serving::placement::report())?;
    for line in scanning::lines(&scanning) {
        writeln!(out, "{line}")?;
    }
    let mut ready = true;
    for tier in Tier::ALL {
        let verdict = verdict(&findings, tier);
        ready = match tier {
            Tier::Use => ready && verdict == Verdict::Ready,
            Tier::Develop => ready,
        };
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

struct Rows<'o, W: Write> {
    out: &'o mut W,
    ink: Ink,

    place: Vec<usize>,

    lines: Vec<(Part, Option<String>)>,
    written: usize,

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
