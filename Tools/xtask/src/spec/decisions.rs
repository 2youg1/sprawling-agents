// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::path::Path;

use crate::lean;
use crate::report::{Violation, XtaskError};
use crate::walk;

const HEADING: &str = "/-!";

pub(super) fn check(root: &Path, out: &mut Vec<Violation>) -> Result<(), XtaskError> {
    let mut first: BTreeMap<(String, String), String> = BTreeMap::new();
    for file in walk::files_with_ext(root, &["lean"])? {
        let rel = walk::rel(root, &file);
        if walk::in_isolation_zone(&rel) {
            continue;
        }
        let Some(specification) = specification_of(&rel) else {
            continue;
        };
        let text = walk::read_text(&file)?;
        for (line, number) in headings(&text) {
            let here = format!("{rel}:{line}");
            match first.get(&(specification.to_owned(), number.clone())) {
                Some(earlier) => out.push(Violation {
                    gate: "spec",
                    location: here,
                    rule: "a decision number heads one decision in its specification \
                           (Tools/xtask/Spec.lean §8-42)"
                        .to_owned(),
                    violation: format!("{number} also heads a decision at {earlier}"),
                    alternative: "give one of the two the specification's next free number, \
                                  and change every place that cites it by the old one"
                        .to_owned(),
                }),
                None => {
                    first.insert((specification.to_owned(), number), here);
                }
            }
        }
    }
    Ok(())
}

fn specification_of(rel: &str) -> Option<&str> {
    if let Some(dir) = rel.strip_suffix(lean::ENTRY) {
        return Some(dir.strip_suffix('/').unwrap_or(dir));
    }
    let parts = format!("/{}/", lean::PARTS);
    rel.find(&parts).and_then(|at| rel.get(..at))
}

fn headings(text: &str) -> Vec<(usize, String)> {
    text.lines()
        .zip(1_usize..)
        .filter_map(|(line, number)| {
            let rest = line.strip_prefix(HEADING)?.trim_start();
            let token: String = rest
                .strip_prefix('D')?
                .chars()
                .take_while(char::is_ascii_alphanumeric)
                .collect();
            let figures = token.chars().take_while(char::is_ascii_digit).count();
            let lettered = token.chars().skip(figures).all(|c| c.is_ascii_lowercase());
            (figures > 0 && lettered).then(|| (number, format!("D{token}")))
        })
        .collect()
}
