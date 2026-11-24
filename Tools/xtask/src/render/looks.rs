// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::report::{Violation, XtaskError};
use crate::sheet::{self, Reading};
use crate::walk;

const LOOK: &str = ".look.solid.tsx";

const GALLERY: &str = concat!(crate::walk::client_src!(), "/views/gallery.solid.tsx");

const HAIRLINE: &str = "1";

pub(super) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut source = BTreeMap::new();
    for path in walk::client_files(root, &["ts", "tsx", "css"])? {
        source.insert(walk::rel(root, &path), walk::read_text(&path)?);
    }
    let drawn = reachable_from(GALLERY, &source);
    let mut out = Vec::new();
    for (rel, text) in source.iter().filter(|(rel, _)| rel.ends_with(LOOK)) {
        if rel.starts_with(walk::CLIENT_SRC) && !drawn.contains(rel) {
            out.push(Violation {
                gate: "render",
                location: rel.clone(),
                rule: "every look is drawn by a fixture on the gallery this gate measures"
                    .to_owned(),
                violation: "the gallery entry cannot reach this look through its fixture imports"
                    .to_owned(),
                alternative: "import the seat in a registered gallery case named for its part (`<part> · <state>`), so the roles and boxes it draws are read"
                    .to_owned(),
            });
        }
        sized_from_tokens(rel, text, &mut out);
        for style in imports(rel, text)
            .into_iter()
            .filter(|import| import.ends_with(".css"))
        {
            if let Some(sheet) = source.get(&style) {
                sized_from_tokens(&style, sheet, &mut out);
            }
        }
    }
    Ok(out)
}

fn reachable_from(entry: &str, source: &BTreeMap<String, String>) -> BTreeSet<String> {
    let mut reached = BTreeSet::new();
    let mut pending = vec![entry.to_owned()];
    while let Some(rel) = pending.pop() {
        if !reached.insert(rel.clone()) {
            continue;
        }
        if let Some(text) = source.get(&rel) {
            pending.extend(
                imports(&rel, text)
                    .iter()
                    .filter_map(|spec| resolved_source(source, spec)),
            );
        }
    }
    reached
}

fn resolved_source(source: &BTreeMap<String, String>, spec: &str) -> Option<String> {
    ["", ".tsx", ".ts", "/index.ts", "/index.tsx"]
        .into_iter()
        .map(|suffix| format!("{spec}{suffix}"))
        .find(|path| source.contains_key(path))
}

fn imports(rel: &str, text: &str) -> Vec<String> {
    let dir = rel.rsplit_once('/').map_or("", |(dir, _)| dir);
    text.lines()
        .filter(|line| !line.trim_start().starts_with("import type "))
        .filter(|line| line.contains("from ") || line.contains("import"))
        .flat_map(|line| line.split('"').skip(1).step_by(2))
        .filter(|spec| spec.starts_with("./") || spec.starts_with("../"))
        .filter_map(|spec| resolved(dir, spec))
        .collect()
}

fn resolved(dir: &str, spec: &str) -> Option<String> {
    let mut parts: Vec<&str> = dir.split('/').filter(|part| !part.is_empty()).collect();
    for part in spec.split('/') {
        match part {
            "." | "" => {}
            ".." => {
                parts.pop()?;
            }
            name => parts.push(name),
        }
    }
    Some(parts.join("/"))
}

fn sized_from_tokens(rel: &str, text: &str, out: &mut Vec<Violation>) {
    for line in sheet::lines(rel, text) {
        let spelled = match line.reading {
            Reading::Code => arbitrary_sizes(line.text),
            Reading::Sheet => pixel_lengths(line.text),
        };
        if let Some(length) = spelled
            .into_iter()
            .find(|length| length != HAIRLINE && length != "0")
        {
            out.push(Violation {
                gate: "render",
                location: format!("{rel}:{}", line.number),
                rule: "a look sizes itself from tokens; the one length it spells is the 1px line"
                    .to_owned(),
                violation: format!("this line spells {length}px"),
                alternative: "read a size token (`size-control`, `h-control-sm`, \
                              `var(--spacing-*)`), and leave the space between parts to the \
                              parent's `gap`"
                    .to_owned(),
            });
        }
    }
}

fn arbitrary_sizes(line: &str) -> Vec<String> {
    line.match_indices("px]")
        .filter_map(|(at, _)| {
            let head = line.get(..at)?;
            let open = head.rfind('[')?;
            let number = head.get(open.checked_add(1)?..)?.trim_start_matches('-');
            is_number(number).then(|| number.to_owned())
        })
        .collect()
}

fn pixel_lengths(line: &str) -> Vec<String> {
    line.match_indices("px")
        .filter_map(|(at, _)| {
            let head = line.get(..at)?;
            let start = head
                .rfind(|c: char| !(c.is_ascii_digit() || c == '.'))
                .map_or(0, |before| before.saturating_add(1));
            let number = head.get(start..)?;
            let before = head.get(..start)?;
            let named = before
                .trim_end_matches('-')
                .ends_with(|c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-');
            (is_number(number) && !named).then(|| number.to_owned())
        })
        .collect()
}

fn is_number(text: &str) -> bool {
    text.chars().any(|c| c.is_ascii_digit()) && text.chars().all(|c| c.is_ascii_digit() || c == '.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_look_is_drawn_when_a_registered_fixture_reaches_it() {
        let entries = [
            (
                "RefRain/src/views/gallery.solid.tsx",
                r#"import Case from "./gallery/runs";"#,
            ),
            (
                "RefRain/src/views/gallery/runs.tsx",
                r#"import Board from "../runs/board.solid";"#,
            ),
            (
                "RefRain/src/views/runs/board.solid.tsx",
                r#"import Look from "./board.look.solid";"#,
            ),
            ("RefRain/src/views/runs/board.look.solid.tsx", ""),
            (
                "RefRain/src/views/gallery/orphan.tsx",
                r#"import Look from "../parts/tabs.look.solid";"#,
            ),
            ("RefRain/src/views/parts/tabs.look.solid.tsx", ""),
        ];
        let source = entries
            .into_iter()
            .map(|(path, content)| (path.to_owned(), content.to_owned()))
            .collect();
        let reached = reachable_from(GALLERY, &source);
        assert!(reached.contains("RefRain/src/views/runs/board.look.solid.tsx"));
        assert!(!reached.contains("RefRain/src/views/parts/tabs.look.solid.tsx"));
    }

    #[test]
    fn a_look_spells_no_length_but_the_hairline() {
        let text = "<div class=\"w-[13px] border-[1px] h-control-sm -mt-[3px]\"></div>\n";
        let style = ".a { height: 13px; border: 1px solid; margin: 0; }\n\
                      .b { top: -2px; }\n .c { gap: var(--gap-2px); inset: 0px; }\n";
        let mut out = Vec::new();
        sized_from_tokens("RefRain/swap/views/parts/x.look.solid.tsx", text, &mut out);
        sized_from_tokens("RefRain/swap/views/parts/x.css", style, &mut out);
        let found: Vec<String> = out
            .iter()
            .map(|v| format!("{} {}", v.location, v.violation))
            .collect();
        assert_eq!(
            found,
            [
                "RefRain/swap/views/parts/x.look.solid.tsx:1 this line spells 13px",
                "RefRain/swap/views/parts/x.css:1 this line spells 13px",
                "RefRain/swap/views/parts/x.css:2 this line spells 2px",
            ]
        );
    }
}
