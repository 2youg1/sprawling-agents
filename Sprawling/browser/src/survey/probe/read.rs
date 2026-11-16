// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::{Cut, Declared, Drawn, Marking, Overflow, Paint, Sampled, TextRun};
use super::BETWEEN;

pub fn elements(records: &str) -> Vec<Drawn> {
    records.split(BETWEEN).filter_map(element).collect()
}

fn element(record: &str) -> Option<Drawn> {
    let mut field = record.split_whitespace();
    let tag = field.next()?.to_owned();
    let role = field.next()?.to_owned();
    let name = decode(field.next()?);
    let mut drawn = Drawn {
        tag,
        role,
        name,
        left: field.next()?.parse().ok()?,
        top: field.next()?.parse().ok()?,
        width: field.next()?.parse().ok()?,
        height: field.next()?.parse().ok()?,
        depth: field.next()?.parse().ok()?,
        parent: field.next()?.parse().ok()?,
        across: overflow(field.next()?)?,
        down: overflow(field.next()?)?,
        sampled: sampled(field.next()?)?,
        first_mark: field.next()?.parse().ok()?,
        underlined: field.next()? == "1",
        fill: paint(field.next()?),
        class: decode(field.next()?),
        text: None,
    };
    drawn.text = text_run(field.next()?);
    Some(drawn)
}

pub fn declared(records: &str) -> Declared {
    let mut colours = Vec::new();
    let mut type_steps = Vec::new();
    let mut spacing = Vec::new();
    for record in records.split(BETWEEN) {
        let Some((name, value)) = record.split_once(' ') else {
            continue;
        };
        if let Some(colour) = paint(value) {
            if name.starts_with("--color-") {
                colours.push((name.to_owned(), colour));
            }
            continue;
        }
        let Ok(hundredths) = value.parse::<u32>() else {
            continue;
        };
        if name.starts_with("--text-") {
            type_steps.push((name.to_owned(), hundredths));
        } else if name.starts_with("--spacing-") {
            spacing.push((name.to_owned(), hundredths));
        }
    }
    Declared::of(colours, type_steps, spacing)
}

fn overflow(field: &str) -> Option<Overflow> {
    match field {
        "shows" => Some(Overflow::Shows),
        "clips" => Some(Overflow::Clips),
        "scrolls" => Some(Overflow::Scrolls),
        _ => None,
    }
}

fn sampled(field: &str) -> Option<Sampled> {
    match field {
        "frame" => Some(Sampled::Frame),
        "words" => Some(Sampled::Words),
        _ => None,
    }
}

fn paint(field: &str) -> Option<Paint> {
    if field.len() != 6 {
        return None;
    }
    let channel = |from: usize| {
        let to = from.checked_add(2)?;
        u8::from_str_radix(field.get(from..to)?, 16).ok()
    };
    Some(Paint::of(channel(0)?, channel(2)?, channel(4)?))
}

fn text_run(field: &str) -> Option<TextRun> {
    let mut part = field.split(',');
    let ink = paint(part.next()?);
    let px_x100 = part.next()?.parse().ok()?;
    let shown = part.next()?.parse().ok()?;
    let needs = part.next()?.parse().ok()?;
    let marking = match part.next()? {
        "ellipsis" => Marking::Ellipsis,
        "spills" => Marking::Spills,
        "clip" => Marking::Clip,

        _ => return None,
    };
    Some(TextRun {
        ink,
        px_x100,
        cut: Cut::of(shown, needs, marking),
    })
}

fn decode(field: &str) -> String {
    if field == "-" {
        return String::new();
    }
    let bytes = field.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index: usize = 0;
    while let Some(byte) = bytes.get(index) {
        if *byte == b'%'
            && let Some(pair) =
                field.get(index.saturating_add(1)..index.saturating_add(3).min(field.len()))
            && let Ok(value) = u8::from_str_radix(pair, 16)
        {
            out.push(value);
            index = index.saturating_add(3);
            continue;
        }
        out.push(*byte);
        index = index.saturating_add(1);
    }
    String::from_utf8_lossy(&out).into_owned()
}
