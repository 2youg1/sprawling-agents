// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::borrow::Cow;

use crate::header::{self, Leader};

pub(super) fn without_license(text: &str) -> Cow<'_, str> {
    if let Some(rest) = strip_prefix_lines(text, &header::notice(Leader::Rust)) {
        return Cow::Borrowed(rest);
    }
    if let Some(rest) = strip_html_wrapped_slash(text) {
        return Cow::Borrowed(rest);
    }
    if let Some(rest) = strip_html_inline_notice(text) {
        return Cow::Borrowed(rest);
    }
    if let Some(owned) = strip_script_embedded_notice(text) {
        return Cow::Owned(owned);
    }
    Cow::Borrowed(text)
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

fn strip_html_wrapped_slash(text: &str) -> Option<&str> {
    let mut rows = text.lines();
    if rows.next() != Some("<!--") {
        return None;
    }
    let expected = header::notice(Leader::Rust);
    if !expected
        .iter()
        .all(|want| rows.next() == Some(want.as_str()))
    {
        return None;
    }
    if rows.next() != Some("-->") {
        return None;
    }
    let offset = text
        .split_inclusive('\n')
        .take(expected.len().saturating_add(2))
        .fold(0_usize, |total, row| total.saturating_add(row.len()));
    Some(text.get(offset..).unwrap_or(text))
}

fn strip_html_inline_notice(text: &str) -> Option<&str> {
    let start = text.strip_prefix('\u{feff}').unwrap_or(text);
    let bom = text.len().saturating_sub(start.len());
    if !start.starts_with("<!--") {
        return None;
    }
    let after_open = start.get(4..)?;
    let close = after_open.find("-->")?;
    let inner = after_open.get(..close)?;
    if !notice_body_matches(inner) {
        return None;
    }
    let end = bom
        .saturating_add(4)
        .saturating_add(close)
        .saturating_add(3);
    let rest = text.get(end..)?;
    Some(
        rest.strip_prefix("\r\n")
            .or_else(|| rest.strip_prefix('\n'))
            .unwrap_or(rest),
    )
}

fn notice_body_matches(inner: &str) -> bool {
    let parts: Vec<&str> = inner.split_whitespace().collect();
    let expected: Vec<&str> = header::NOTICE
        .iter()
        .flat_map(|row| row.split_whitespace())
        .collect();
    parts == expected
}

fn strip_script_embedded_notice(text: &str) -> Option<String> {
    let mut lines = text.split_inclusive('\n');
    let first = lines.next()?;
    if !first.trim_start().starts_with("<script") {
        return None;
    }
    let expected = header::notice(Leader::Rust);
    for want in &expected {
        let row = lines.next()?;
        let content = row
            .trim_end_matches('\n')
            .trim_end_matches('\r')
            .trim_start();
        if content != want.as_str() {
            return None;
        }
    }
    let mut owned = String::with_capacity(text.len());
    owned.push_str(first);
    for row in lines {
        owned.push_str(row);
    }
    Some(owned)
}
