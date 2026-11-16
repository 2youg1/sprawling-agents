// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::Payload;
use kernel::event::record::Rendering;
use serde_json::Value;

const MARK_EVERY: u64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Numbering {
    FromZero,

    FromOne,
}

impl Numbering {
    #[must_use]
    pub fn of(rendering: Rendering) -> Numbering {
        match rendering {
            Rendering::V1 | Rendering::V2 => Numbering::FromZero,
            Rendering::V3 | Rendering::V4 => Numbering::FromOne,
        }
    }

    pub(crate) fn shown(self, index: u64) -> Option<u64> {
        match self {
            Numbering::FromZero => Some(index),
            Numbering::FromOne => index.checked_add(1),
        }
    }

    fn index(self, number: u64) -> Option<u64> {
        match self {
            Numbering::FromZero => Some(number),
            Numbering::FromOne => number.checked_sub(1),
        }
    }
}

const MARK_END: char = '|';

pub(super) fn text(result: &Payload, numbering: Numbering) -> Option<String> {
    draw(result, numbering, Additions::Omitted)
}

pub(super) fn text_v4(result: &Payload) -> Option<String> {
    draw(result, Numbering::FromOne, Additions::Included)
}

enum Additions {
    Omitted,
    Included,
}

fn draw(result: &Payload, numbering: Numbering, additions: Additions) -> Option<String> {
    let map = result.as_map();
    if matches!(additions, Additions::Included)
        && map.get("unchanged").and_then(Value::as_bool) == Some(true)
    {
        let path = map.get("path")?.as_str()?;
        let offset = number(map.get("offset"))??;
        let lines = number(map.get("lines"))??;
        let total = number(map.get("total_lines"))??;
        let mut out = unchanged(Stub {
            path,
            offset,
            lines,
            total_lines: total,
            numbering,
        })?;
        append_rules(&mut out, rules(map)?);
        return Some(out);
    }
    let path = map.get("path")?.as_str()?;
    let body = map.get("text")?.as_str()?;
    let offset = number(map.get("offset"))?.unwrap_or(0);
    let total = number(map.get("total_lines"))?;
    let next = number(map.get("next_offset"))?;
    let lines: Vec<&str> = body.split_inclusive('\n').collect();
    let shown = u64::try_from(lines.len()).ok()?;
    let mut out = String::with_capacity(
        body.len()
            .saturating_add(lines.len().saturating_mul(2) / 5)
            .saturating_add(160),
    );
    out.push_str(path);
    match shown
        .checked_sub(1)
        .and_then(|span| offset.checked_add(span))
    {
        None => match total {
            Some(0) | None => out.push_str(" is empty"),
            Some(total) => out.push_str(&format!(
                " has {total} lines; offset {} is past the end",
                numbering.shown(offset)?
            )),
        },
        Some(last) => {
            let total = match total {
                Some(total) => total,
                None => last.checked_add(1)?,
            };
            out.push_str(&format!(
                " lines {}-{} of {total}",
                numbering.shown(offset)?,
                numbering.shown(last)?
            ));
            if let Some(next) = next {
                out.push_str(&format!(", next {}", numbering.shown(next)?));
            }
        }
    }
    out.push('\n');
    for (at, line) in (offset..).zip(lines) {
        push_line(&mut out, numbering, (offset, at), line)?;
    }
    if matches!(additions, Additions::Included) {
        append_rules(&mut out, rules(map)?);
    }
    Some(out)
}

pub(crate) struct Stub<'a> {
    pub(crate) path: &'a str,
    pub(crate) offset: u64,
    pub(crate) lines: u64,
    pub(crate) total_lines: u64,
    pub(crate) numbering: Numbering,
}

pub(crate) fn unchanged(stub: Stub<'_>) -> Option<String> {
    const SENTENCE: &str = "unchanged since your earlier read; not repeated";
    if stub.lines == 0 {
        return match (stub.offset, stub.total_lines) {
            (0, 0) => Some(format!("{} is empty: {SENTENCE}\n", stub.path)),
            (offset, total) if offset >= total => Some(format!(
                "{} has {total} lines; offset {} is past the end: {SENTENCE}\n",
                stub.path,
                stub.numbering.shown(offset)?
            )),
            _ => None,
        };
    }
    let last = stub.offset.checked_add(stub.lines.checked_sub(1)?)?;
    if last >= stub.total_lines {
        return None;
    }
    Some(format!(
        "{} lines {}-{} of {}: {SENTENCE}\n",
        stub.path,
        stub.numbering.shown(stub.offset)?,
        stub.numbering.shown(last)?,
        stub.total_lines,
    ))
}

fn rules(map: &serde_json::Map<String, Value>) -> Option<Vec<(&str, &str)>> {
    let Some(value) = map.get("rules") else {
        return Some(Vec::new());
    };
    value
        .as_array()?
        .iter()
        .map(|rule| Some((rule.get("room")?.as_str()?, rule.get("text")?.as_str()?)))
        .collect()
}

pub fn rule_overhead(room: &str) -> usize {
    let mut rendered = String::from("body");
    append_rules(&mut rendered, vec![(room, "")]);
    rendered
        .len()
        .saturating_sub("body".len())
        .saturating_add(1)
}

fn append_rules(out: &mut String, rules: Vec<(&str, &str)>) {
    const PRIORITY: &str = "How work is done in this part of the project, from the project itself. Where this and the building's rules above disagree, the rules decide: they are what the city enforces.";
    if rules.is_empty() {
        return;
    }
    for (index, (room, text)) in rules.into_iter().enumerate() {
        if index > 0 || !out.is_empty() {
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push('\n');
        }
        out.push_str(&format!("## {room}/AGENTS.md\n\n{PRIORITY}\n\n{text}"));
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
}

pub(super) fn push_line(
    out: &mut String,
    numbering: Numbering,
    (first, at): (u64, u64),
    line: &str,
) -> Option<()> {
    let number = numbering.shown(at)?;
    if is_marked(numbering.shown(first)?, number) {
        out.push_str(&format!("{number}{MARK_END}"));
    }
    out.push_str(line);
    Some(())
}

fn is_marked(first: u64, at: u64) -> bool {
    at == first || at.is_multiple_of(MARK_EVERY)
}

fn number(value: Option<&Value>) -> Option<Option<u64>> {
    match value {
        None => Some(None),
        Some(value) => value.as_u64().map(Some),
    }
}

pub struct Copied<'a> {
    lines: Vec<CopiedLine<'a>>,
    numbering: Numbering,
}

struct CopiedLine<'a> {
    whole: &'a str,

    mark: Option<(u64, &'a str)>,
}

impl<'a> Copied<'a> {
    #[must_use]
    pub fn of(text: &'a str, numbering: Numbering) -> Copied<'a> {
        let lines = text
            .split_inclusive('\n')
            .map(|whole| {
                let digits = whole.bytes().take_while(u8::is_ascii_digit).count();
                let mark = whole
                    .get(..digits)
                    .filter(|spelled| !spelled.is_empty())
                    .and_then(|spelled| spelled.parse::<u64>().ok())
                    .zip(
                        whole
                            .get(digits..)
                            .and_then(|rest| rest.strip_prefix(MARK_END)),
                    );
                CopiedLine { whole, mark }
            })
            .collect();
        Copied { lines, numbering }
    }

    #[must_use]
    pub fn starts(&self) -> std::collections::BTreeSet<u64> {
        (0_u64..)
            .zip(&self.lines)
            .filter_map(|(at, line)| {
                let (number, _) = line.mark?;
                let first = number
                    .checked_sub(at)
                    .filter(|&first| is_marked(first, number))?;
                self.numbering.index(first)
            })
            .collect()
    }

    #[must_use]
    pub fn drawn_from(&self, start: u64) -> Option<(String, std::collections::BTreeSet<u64>)> {
        let first = self.numbering.shown(start)?;
        let mut out = String::new();
        let mut taken = std::collections::BTreeSet::new();
        for (at, line) in (0_u64..).zip(&self.lines) {
            let drawn = line.mark.filter(|&(number, _)| {
                first.checked_add(at) == Some(number) && is_marked(first, number)
            });
            match drawn {
                Some((number, rest)) => {
                    out.push_str(rest);
                    taken.insert(number);
                }
                None => out.push_str(line.whole),
            }
        }
        (!taken.is_empty()).then_some((out, taken))
    }

    #[must_use]
    pub fn without(&self, numbers: &std::collections::BTreeSet<u64>) -> String {
        self.lines
            .iter()
            .map(|line| match line.mark {
                Some((number, rest)) if numbers.contains(&number) => rest,
                Some(_) | None => line.whole,
            })
            .collect()
    }
}
