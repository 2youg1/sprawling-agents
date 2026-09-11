// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One refusal code connected to the items on this machine that can
//! raise it (sprawling-SPEC.md section 8-48).
//!
//! A refusal's `recovery` is written where the refusal is, and it
//! cannot know this machine. `sprawling doctor --explain <code>` is the
//! join: for each item that can raise the code, what this machine
//! answered and the next move on this platform, in one line.

use kernel::AxCode;

use super::family::{CHROMIUM, GECKO, WEBKIT};
use super::table::{
    CHROMEDRIVER, FFMPEG, MSEDGEDRIVER, PYTHON_WASI, SANDBOX_ENGINE, SHELL, SPRAWLING_DESKTOP,
};
use super::{Finding, Platform};

/// What `--explain` answers.
#[derive(Debug)]
pub(crate) enum Explanation {
    /// Not a code this city has.
    NoSuchCode(String),
    /// A code decided inside the city, which no item on this machine
    /// can change.
    NotAboutThisMachine(AxCode),
    /// One line per item that can raise the code.
    Lines(Vec<String>),
}

/// The items that can raise a code, when the code is about this
/// machine at all. The table this function is: editing it is editing
/// what `--explain` knows.
fn items_behind(code: AxCode) -> Option<&'static [&'static str]> {
    match code {
        AxCode::ToolUnavailable => Some(&[
            SANDBOX_ENGINE,
            PYTHON_WASI,
            SHELL,
            SPRAWLING_DESKTOP,
            FFMPEG,
        ]),
        AxCode::BrowserUnavailable => Some(&[GECKO, CHROMIUM, CHROMEDRIVER, MSEDGEDRIVER, WEBKIT]),
        _ => None,
    }
}

/// Connects one code to the findings.
pub(crate) fn explain(code: &str, findings: &[Finding], platform: Option<Platform>) -> Explanation {
    let Some(parsed) = AxCode::parse(code) else {
        return Explanation::NoSuchCode(code.to_owned());
    };
    let Some(items) = items_behind(parsed) else {
        return Explanation::NotAboutThisMachine(parsed);
    };
    let lines = items
        .iter()
        .filter_map(|item| {
            findings
                .iter()
                .find(|finding| finding.requirement.name == *item)
        })
        .map(|finding| {
            let next = match platform {
                _ if finding.presence.usable() => "nothing to do".to_owned(),
                Some(platform) => finding.requirement.recipe.at(platform).spelled(),
                None => "no recipe for this platform".to_owned(),
            };
            format!(
                "  {:<18} {} -> {next}",
                finding.requirement.name,
                finding.presence.describe()
            )
        })
        .collect();
    Explanation::Lines(lines)
}

/// The heading above the lines, and the two one-line answers.
pub(crate) fn explanation_lines(code: &str, explanation: &Explanation) -> Vec<String> {
    match explanation {
        Explanation::NoSuchCode(raw) => vec![format!("  {raw}: not a code this city has")],
        Explanation::NotAboutThisMachine(parsed) => vec![format!(
            "  {}: decided inside the city; nothing on this machine changes it",
            parsed.as_str()
        )],
        Explanation::Lines(lines) => {
            let mut out = vec![format!(
                "  {code}: what this machine has behind it, and the next move here\n"
            )];
            out.extend(lines.iter().cloned());
            out
        }
    }
}
