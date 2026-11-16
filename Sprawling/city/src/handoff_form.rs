// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::spine_files::blank::is_guidance;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HandoffSections {

    pub overall: Option<String>,

    pub progress: Option<String>,

    pub context: Option<String>,

    pub next_step: Option<String>,
}

pub fn handoff_sections(text: &str) -> HandoffSections {
    HandoffSections {
        overall: section(text, "overall"),
        progress: section(text, "current-progress"),
        context: section(text, "context"),
        next_step: section(text, "next-step"),
    }
}

fn section(text: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let body = text
        .lines()
        .skip_while(|line| line.trim() != open)
        .skip(1)
        .take_while(|line| line.trim() != close)
        .filter(|line| !is_guidance(line.trim()))
        .collect::<Vec<&str>>()
        .join("\n");
    let body = body.trim();
    (!body.is_empty()).then(|| body.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_section_left_as_guidance_reads_as_not_written() {
        let text = "<overall>\nMeasure the drift.\n</overall>\n\n\
                    <current-progress>\n(Say it against `Roadmap.md`.)\n</current-progress>\n\n\
                    <next-step>\nTake reading four.\nThen stop.\n</next-step>\n";
        assert_eq!(
            handoff_sections(text),
            HandoffSections {
                overall: Some("Measure the drift.".to_owned()),
                progress: None,
                context: None,
                next_step: Some("Take reading four.\nThen stop.".to_owned()),
            }
        );
    }
}
