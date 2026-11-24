// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::Language;
use crate::header;

pub(super) fn without_license(text: &str, language: Language) -> &str {
    let plain = match language {
        Language::Rust | Language::TypeScript | Language::JavaScript | Language::Zig => {
            strip_prefix_lines(text, &notice("//"))
        }
        Language::Python => strip_prefix_lines(text, &notice("#")),
        Language::Css => strip_prefix_lines(text, &css_notice()),
    };
    plain.unwrap_or(text)
}

fn notice(mark: &str) -> [String; 4] {
    header::NOTICE.map(|row| format!("{mark} {row}"))
}

fn css_notice() -> [String; 4] {
    let [first, second, third, fourth] = header::NOTICE;
    [
        format!("/* {first}"),
        format!(" * {second}"),
        format!(" * {third}"),
        format!(" * {fourth} */"),
    ]
}

fn strip_prefix_lines<'a>(text: &'a str, expected: &[String]) -> Option<&'a str> {
    let mut rows = text.lines();
    let licensed = expected
        .iter()
        .all(|want| rows.next() == Some(want.as_str()));
    if !licensed {
        return None;
    }
    let offset = text
        .split_inclusive('\n')
        .take(expected.len())
        .fold(0_usize, |total, row| total.saturating_add(row.len()));
    Some(text.get(offset..).unwrap_or(text))
}
