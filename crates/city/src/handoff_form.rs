// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A room's handoff form read into its sections.
//!
//! The template spells each section as a tag alone on its line, the
//! body, and the closing tag alone on its line, so a section is the
//! lines between the two. This module is the only reader of those tag
//! names outside the template itself.
//!
//! Specified by `crates/city/spec/SpineFiles.lean` §8-5.

use crate::spine_files::blank::is_guidance;

/// The four prose sections of a filled `Handoff.md`.
///
/// Each is `None` when the section is absent or still holds only the
/// template's own guidance: a section nobody wrote says nothing, and a
/// reader given the guidance would take the template's instructions for
/// the last session's findings. The form's `<must-read>` section is not
/// here, because it is prose for the next agent rather than a list of
/// locators; the caller pins the whole file instead.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HandoffSections {
    /// `<overall>`: what the work is, and why.
    pub overall: Option<String>,
    /// `<current-progress>`: where the work stands.
    pub progress: Option<String>,
    /// `<context>`: what a newcomer would get wrong without being told.
    pub context: Option<String>,
    /// `<next-step>`: what to do first.
    pub next_step: Option<String>,
}

/// Reads a handoff's text into its four prose sections.
pub fn handoff_sections(text: &str) -> HandoffSections {
    HandoffSections {
        overall: section(text, "overall"),
        progress: section(text, "current-progress"),
        context: section(text, "context"),
        next_step: section(text, "next-step"),
    }
}

/// The body of one section, without the template's guidance lines, or
/// `None` when nothing else is there.
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
