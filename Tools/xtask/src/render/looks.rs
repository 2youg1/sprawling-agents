// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;
use std::path::Path;

use crate::report::{Violation, XtaskError};
use crate::sheet::{self, Reading};
use crate::walk;

const LOOK: &str = ".look.svelte";

const GALLERY: &str = concat!(crate::walk::client_src!(), "/views/gallery/");

const HAIRLINE: &str = "1";

pub(super) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut looks = Vec::new();
    let mut imported = BTreeSet::new();
    for path in walk::client_files(root, &["svelte", "ts"])? {
        let rel = walk::rel(root, &path);
        if rel.ends_with(LOOK) {
            looks.push((rel.clone(), walk::read_text(&path)?));
        }
        if rel.starts_with(GALLERY) {
            imported.extend(imports(&rel, &walk::read_text(&path)?));
        }
    }
    let mut out = Vec::new();
    for (rel, text) in &looks {
        if rel.starts_with(walk::CLIENT_SRC) && !drawn_by_gallery(rel, &imported) {
            out.push(Violation {
                gate: "render",
                location: rel.clone(),
                rule: "every look is drawn by a fixture on the gallery this gate measures"
                    .to_owned(),
                violation: "no file under client/src/views/gallery/ imports this look or its seat"
                    .to_owned(),
                alternative: "import the seat in a gallery case named for its part \
                              (`<part> · <state>`), so the roles and boxes it draws are read"
                    .to_owned(),
            });
        }
        sized_from_tokens(rel, text, &mut out);
    }
    Ok(out)
}

fn drawn_by_gallery(rel: &str, imported: &BTreeSet<String>) -> bool {
    let seat = rel.strip_suffix(LOOK).map(|stem| format!("{stem}.svelte"));
    imported.contains(rel) || seat.is_some_and(|seat| imported.contains(&seat))
}

fn imports(rel: &str, text: &str) -> Vec<String> {
    let dir = rel.rsplit_once('/').map_or("", |(dir, _)| dir);
    text.lines()
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
    fn a_look_is_drawn_when_a_fixture_imports_it_or_its_seat() {
        let imported: BTreeSet<String> = imports(
            "client/src/views/gallery/parts/segmented.svelte",
            "  import Segmented from \"../../parts/segmented.svelte\";\n\
             const lazy = import(\"./other.svelte\");",
        )
        .into_iter()
        .collect();
        assert!(drawn_by_gallery(
            "client/src/views/parts/segmented.look.svelte",
            &imported
        ));
        assert!(imported.contains("client/src/views/gallery/parts/other.svelte"));
        assert!(!drawn_by_gallery(
            "client/src/views/parts/tabs.look.svelte",
            &imported
        ));
    }

    #[test]
    fn a_look_spells_no_length_but_the_hairline() {
        let text = "<div class=\"w-[13px] border-[1px] h-control-sm -mt-[3px]\"></div>\n\
                    <style>\n  .a { height: 13px; border: 1px solid; margin: 0; }\n\
                    .b { top: -2px; }\n  .c { gap: var(--gap-2px); inset: 0px; }\n</style>\n";
        let mut out = Vec::new();
        sized_from_tokens("client/swap/views/parts/x.look.svelte", text, &mut out);
        let found: Vec<String> = out
            .iter()
            .map(|v| format!("{} {}", v.location, v.violation))
            .collect();
        assert_eq!(
            found,
            [
                "client/swap/views/parts/x.look.svelte:1 this line spells 13px",
                "client/swap/views/parts/x.look.svelte:3 this line spells 13px",
                "client/swap/views/parts/x.look.svelte:4 this line spells 2px",
            ]
        );
    }
}
