// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::{SPOKEN, Said, words_the_view_wrote};

pub(super) fn handed_to_a_reader(src: &str) -> Vec<Said> {
    let mut out = Vec::new();
    let mut block: Option<&'static str> = None;
    let mut comment = false;
    for (index, line) in src.lines().enumerate() {
        let number = index.saturating_add(1);
        let drawn = visible(line, &mut block, &mut comment);
        for said in text_nodes(&drawn) {
            if let Some(left) = words_the_view_wrote(&said) {
                out.push(Said {
                    line: number,
                    seat: "a text node",
                    left,
                });
            }
        }
        for said in spoken_attributes(&drawn) {
            if let Some(left) = words_the_view_wrote(&said) {
                out.push(Said {
                    line: number,
                    seat: "a spoken attribute",
                    left,
                });
            }
        }
    }
    out
}

fn visible(line: &str, block: &mut Option<&'static str>, comment: &mut bool) -> String {
    let mut out = String::new();
    let mut rest = line;
    loop {
        if *comment {
            match rest.find("-->") {
                Some(at) => {
                    *comment = false;
                    rest = rest.get(at.saturating_add(3)..).unwrap_or("");
                }
                None => return out,
            }
            continue;
        }
        if let Some(kind) = *block {
            let close = format!("</{kind}>");
            match rest.find(&close) {
                Some(at) => {
                    *block = None;
                    rest = rest.get(at.saturating_add(close.len())..).unwrap_or("");
                }
                None => return out,
            }
            continue;
        }

        const OPENERS: [&str; 3] = ["<!--", "<script", "<style"];
        let next = OPENERS
            .iter()
            .filter_map(|opener| rest.find(opener).map(|at| (at, *opener)))
            .min_by_key(|(at, _)| *at);
        let Some((at, opener)) = next else {
            out.push_str(rest);
            return out;
        };
        if let Some(head) = rest.get(..at) {
            out.push_str(head);
        }
        let after = rest.get(at.saturating_add(opener.len())..).unwrap_or("");
        if opener == "<!--" {
            *comment = true;
        } else if after.starts_with('>')
            || after
                .chars()
                .next()
                .is_some_and(|mark| mark.is_whitespace() || mark == '/')
        {
            *block = Some(if opener == "<script" {
                "script"
            } else {
                "style"
            });
        } else {
            out.push_str(opener);
        }
        rest = after;
    }
}

fn text_nodes(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = code.as_bytes();
    let mut index: usize = 0;

    let mut opened: Vec<Opening> = Vec::new();
    while let Some(byte) = bytes.get(index) {
        let after = bytes.get(index.saturating_add(1));
        if *byte == b'<' {

            if after != Some(&b'=') {
                let before = index.checked_sub(1).and_then(|at| bytes.get(at));
                opened.push(match before {
                    Some(mark) if mark.is_ascii_alphanumeric() || *mark == b'_' => {
                        Opening::TypeArgs
                    }
                    _ => Opening::Element,
                });
            }
            index = index.saturating_add(1);
            continue;
        }
        if *byte != b'>' {
            index = index.saturating_add(1);
            continue;
        }
        let before = index.checked_sub(1).and_then(|at| bytes.get(at));

        if before == Some(&b'=') || after == Some(&b'=') {
            index = index.saturating_add(1);
            continue;
        }
        if opened.pop() == Some(Opening::TypeArgs) {
            index = index.saturating_add(1);
            continue;
        }
        let start = index.saturating_add(1);
        let rest = code.get(start..).unwrap_or("");
        let end = rest.find(['<', '{', '}']).unwrap_or(rest.len());

        if rest.get(end..).is_some_and(|tail| tail.starts_with('<'))
            && let Some(run) = rest.get(..end)
        {
            out.push(run.to_owned());
        }
        index = start.saturating_add(end);
    }
    out
}

#[derive(PartialEq, Eq)]
enum Opening {
    Element,
    TypeArgs,
}

fn spoken_attributes(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    for name in SPOKEN {
        let mark = format!("{name}=\"");
        let mut from: usize = 0;
        while let Some(found) = code.get(from..).and_then(|rest| rest.find(&mark)) {
            let opens = from.saturating_add(found).saturating_add(mark.len());
            let Some(rest) = code.get(opens..) else { break };
            let Some(close) = rest.find('"') else { break };
            if let Some(value) = rest.get(..close) {
                out.push(value.to_owned());
            }
            from = opens.saturating_add(close);
        }
    }
    out
}
