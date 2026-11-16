// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::AxCode;

use super::family::{CHROMIUM, GECKO, WEBKIT};
use super::table::{CHROMEDRIVER, FFMPEG, MSEDGEDRIVER, PWSH, PYTHON_WASI, SANDBOX_ENGINE, SHELL};
use super::{Finding, Platform};

#[derive(Debug)]
pub(crate) enum Explanation {
    NoSuchCode(String),

    NotAboutThisMachine(AxCode),

    Lines(Vec<String>),
}

#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "two codes are about a missing program; the other sixty name no program"
)]
fn items_behind(code: AxCode) -> Option<&'static [&'static str]> {
    match code {
        AxCode::ToolUnavailable => Some(&[SANDBOX_ENGINE, PYTHON_WASI, SHELL, PWSH, FFMPEG]),
        AxCode::BrowserUnavailable => Some(&[GECKO, CHROMIUM, CHROMEDRIVER, MSEDGEDRIVER, WEBKIT]),
        _ => None,
    }
}

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
