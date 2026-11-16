// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use super::{Deviation, Drawn, Finding, Sampled};

const TOO_SHORT: usize = 3;

const A_COLUMN: usize = 4;

const PUNCTUATION: [&str; 6] = ["—", "–", "…", "·", "n/a", "N/A"];

const LANDMARKS: [&str; 7] = [
    "navigation",
    "main",
    "contentinfo",
    "banner",
    "complementary",
    "dialog",
    "form",
];

pub(super) fn one_fact_has_one_home<'a>(drawn: &'a [Drawn], out: &mut Vec<Deviation<'a>>) {
    let mut painted: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (seat, box_) in drawn.iter().enumerate() {
        if !carries_a_fact(box_) {
            continue;
        }
        painted.entry(box_.name.trim()).or_default().push(seat);
    }
    for (text, seats) in painted {
        if seats.len() < 2 || seats.len() > A_COLUMN {
            continue;
        }
        let homes: Vec<(usize, &str)> = seats
            .iter()
            .filter_map(|seat| drawn.get(*seat).map(|_| (*seat, landmark(drawn, *seat))))
            .collect();

        let Some((first, where_first)) = homes.first().copied() else {
            continue;
        };
        if homes.iter().all(|(_, held)| *held == where_first) {
            continue;
        }
        for (seat, _) in homes.iter().skip(1) {
            let (Some(at), Some(other)) = (drawn.get(*seat), drawn.get(first)) else {
                continue;
            };
            out.push(Deviation {
                at,
                finding: Finding::Echoed {
                    other,
                    text,
                    homes: homes.len(),
                },
            });
        }
    }
}

fn carries_a_fact(box_: &Drawn) -> bool {
    if !matches!(box_.sampled, Sampled::Words) || box_.text.is_none() {
        return false;
    }
    let said = box_.name.trim();
    said.chars().count() > TOO_SHORT
        && !PUNCTUATION.contains(&said)
        && said.chars().any(char::is_alphanumeric)
}

fn landmark(drawn: &[Drawn], seat: usize) -> &str {
    let mut here = seat;

    for _ in 0..drawn.len() {
        let Some(box_) = drawn.get(here) else {
            return "the page";
        };
        if LANDMARKS.contains(&box_.role.as_str()) {
            return box_.role.as_str();
        }
        let Ok(up) = usize::try_from(box_.parent) else {
            return "the page";
        };
        here = up;
    }
    "the page"
}
