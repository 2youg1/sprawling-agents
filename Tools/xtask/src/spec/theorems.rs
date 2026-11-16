// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Class {
    TextOnly,

    ClosedOnly,

    QuantifiedUnchecked,

    QuantifiedChecked,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Theorem {
    pub(crate) name: String,
    pub(crate) quantified: bool,
}

pub(crate) fn theorems(code: &str) -> Vec<Theorem> {
    let lines: Vec<&str> = code.lines().collect();
    lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            let rest = declared(line.trim())?;
            let name: String = rest
                .chars()
                .take_while(|c| !c.is_whitespace() && *c != ':' && *c != '(' && *c != '{')
                .collect();
            let after = rest.get(name.len()..).unwrap_or_default();
            let head = head_from(
                after,
                lines.get(index.saturating_add(1)..).unwrap_or_default(),
            );
            Some(Theorem {
                name: short(&name).to_owned(),
                quantified: binds(&head),
            })
        })
        .collect()
}

pub(crate) fn classify(found: &[Theorem], checked: impl Fn(&str) -> bool) -> Class {
    let quantified: Vec<&Theorem> = found.iter().filter(|t| t.quantified).collect();
    match (found.is_empty(), quantified.is_empty()) {
        (true, _) => Class::TextOnly,
        (false, true) => Class::ClosedOnly,
        (false, false) if quantified.iter().any(|t| checked(&t.name)) => Class::QuantifiedChecked,
        (false, false) => Class::QuantifiedUnchecked,
    }
}

pub(crate) fn holds_check(text: &str) -> bool {
    text.contains("#[test]") || text.contains("proptest!")
}

pub(crate) fn has_word(text: &str, name: &str) -> bool {
    !name.is_empty()
        && text.match_indices(name).any(|(at, _)| {
            let before = text.get(..at).and_then(|head| head.chars().next_back());
            let after = text
                .get(at.saturating_add(name.len())..)
                .and_then(|tail| tail.chars().next());
            !before.is_some_and(ident) && !after.is_some_and(ident)
        })
}

fn declared(line: &str) -> Option<&str> {
    let mut rest = line;
    while let Some(attribute) = rest.strip_prefix("@[") {
        rest = attribute.split_once(']')?.1.trim_start();
    }
    for modifier in ["private ", "protected "] {
        if let Some(stripped) = rest.strip_prefix(modifier) {
            rest = stripped.trim_start();
        }
    }
    ["theorem ", "lemma "]
        .iter()
        .find_map(|keyword| rest.strip_prefix(keyword))
        .map(str::trim_start)
}

fn head_from(first: &str, after: &[&str]) -> String {
    if let Some((before, _)) = first.split_once(":=") {
        return before.to_owned();
    }
    let mut head = first.to_owned();
    for line in after {
        if declared(line.trim()).is_some() {
            break;
        }
        head.push(' ');
        if let Some((before, _)) = line.split_once(":=") {
            head.push_str(before);
            break;
        }
        head.push_str(line);
    }
    head
}

fn binds(head: &str) -> bool {
    ["∀", "∃", "→", "forall"]
        .iter()
        .any(|mark| head.contains(mark))
        || ['(', '{'].iter().any(|open| {
            head.match_indices(*open).any(|(at, _)| {
                let tail = head.get(at.saturating_add(1)..).unwrap_or_default();
                let names: String = tail
                    .trim_start()
                    .chars()
                    .take_while(|c| ident(*c) || c.is_whitespace())
                    .collect();
                let first = names.trim_start().chars().next();
                first.is_some_and(|c| c.is_alphabetic() || c == '_')
                    && tail
                        .trim_start()
                        .get(names.len()..)
                        .is_some_and(|rest| rest.starts_with(':'))
            })
        })
}

fn short(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or(name)
}

fn ident(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '\'')
}
