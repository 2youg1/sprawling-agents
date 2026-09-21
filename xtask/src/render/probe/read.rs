// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The reading half of the probe's sentence.
//!
//! **A child of the file that writes it, not a sibling.** The record is
//! a line of positional fields with no schema behind it, so a field
//! inserted on one side and not the other silently shifts every field
//! after it and the gate goes on reporting numbers that are no longer
//! what they are named. Keeping the reader directly under the writer is
//! the whole of the defence available here: the two are one sentence,
//! and a reader who finds either finds both.

use super::BETWEEN;
use crate::survey::{Cut, Declared, Drawn, Marking, Overflow, Paint, Sampled, TextRun};

/// Every element in one sink, as the probe wrote them.
pub(crate) fn elements(records: &str) -> Vec<Drawn> {
    records.split(BETWEEN).filter_map(element).collect()
}

/// `tag role name left top width height depth parent across down
/// sampled first-mark underlined fill class text`, as the probe writes
/// it. The name and the class are percent-encoded, because they are the
/// two fields that hold a person's words and those contain spaces.
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

/// The page's own vocabulary, as the probe resolved it: one `name value`
/// per record, colours in hex and lengths in hundredths of a pixel.
pub(crate) fn declared(records: &str) -> Declared {
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

/// `shows`, `clips` or `scrolls`, in the three words the probe writes.
fn overflow(field: &str) -> Option<Overflow> {
    match field {
        "shows" => Some(Overflow::Shows),
        "clips" => Some(Overflow::Clips),
        "scrolls" => Some(Overflow::Scrolls),
        _ => None,
    }
}

/// `frame` or `words`: what put this element into the measurement.
fn sampled(field: &str) -> Option<Sampled> {
    match field {
        "frame" => Some(Sampled::Frame),
        "words" => Some(Sampled::Words),
        _ => None,
    }
}

/// `rrggbb`, or nothing when the probe could not decide what was
/// painted there.
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

/// `ink,px-hundredths,shown,needs,mark`, or `-` when the element draws
/// no text of its own.
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
        // A word this file did not write is a record it cannot read,
        // and a dropped record is honest where a default is not.
        _ => return None,
    };
    Some(TextRun {
        ink,
        px_x100,
        cut: Cut::of(shown, needs, marking),
    })
}

/// Percent-decoding, which is all the probe needs on this side.
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
