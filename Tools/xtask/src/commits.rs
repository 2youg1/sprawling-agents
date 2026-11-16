// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;
use std::process::Command;

use crate::report::{Violation, XtaskError};

const RULING: &str = "Verdict: user-approved";

pub(crate) enum Target<'a> {
    Range(&'a str),
    RangeAndTitle { range: &'a str, title: &'a str },
}

pub(crate) fn check(root: &Path, target: Target<'_>) -> Result<Vec<Violation>, XtaskError> {
    let (range, title) = match target {
        Target::Range(range) => (range, None),
        Target::RangeAndTitle { range, title } => (range, Some(title)),
    };
    let mut violations = range_violations(root, range)?;
    if let Some(title) = title {
        violations.extend(title_findings(title).into_iter().map(|violation| {
            violation_for(
                "pull request title".to_owned(),
                violation,
                "reword the pull request title before it merges",
            )
        }));
    }
    Ok(violations)
}

#[expect(clippy::disallowed_methods, reason = "developer tool (child D4)")]
fn range_violations(root: &Path, range: &str) -> Result<Vec<Violation>, XtaskError> {
    let cmd = format!("git log --no-merges {range}");
    let output = Command::new("git")
        .args(["log", "--no-merges", "--format=%H%x1f%B%x1e", range])
        .current_dir(root)
        .output()
        .map_err(|err| XtaskError::Cmd {
            cmd: cmd.clone(),
            msg: err.to_string(),
        })?;
    if !output.status.success() {
        return Err(XtaskError::Cmd {
            cmd,
            msg: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .split('\u{1e}')
        .filter_map(|record| record.trim_start().split_once('\u{1f}'))
        .flat_map(|(hash, message)| {
            let short = hash.get(..10).unwrap_or(hash).to_owned();
            findings(message).into_iter().map(move |violation| {
                violation_for(
                    short.clone(),
                    violation,
                    "reword the commit (`git rebase -i`, `reword`) before it merges",
                )
            })
        })
        .collect())
}

fn violation_for(location: String, violation: String, alternative: &str) -> Violation {
    Violation {
        gate: "commits",
        location,
        rule: format!("a subject starts `CS<stage>.<index>: `; a ruling is `{RULING}`"),
        violation,
        alternative: alternative.to_owned(),
    }
}

fn findings(message: &str) -> Vec<String> {
    let subject = message.lines().next().unwrap_or_default();
    let carded = (!carded(subject)).then(|| {
        format!("the subject `{subject}` does not start with `CS<digits>.<digits or capitals>: `")
    });
    carded
        .into_iter()
        .chain(
            message
                .lines()
                .skip(1)
                .filter(|line| line.starts_with("Verdict:") && line.trim_end() != RULING)
                .map(|line| format!("the trailer `{line}` is not spelt `{RULING}`")),
        )
        .collect()
}

fn title_findings(title: &str) -> Vec<String> {
    (!carded(title))
        .then(|| {
            format!(
                "the pull request title `{title}` does not start with `CS<digits>.<digits or capitals>: `"
            )
        })
        .into_iter()
        .collect()
}

fn carded(subject: &str) -> bool {
    subject
        .strip_prefix("CS")
        .and_then(|rest| rest.split_once(": "))
        .and_then(|(card, _)| card.split_once('.'))
        .is_some_and(|(stage, index)| {
            !stage.is_empty()
                && stage.bytes().all(|byte| byte.is_ascii_digit())
                && !index.is_empty()
                && index
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || byte.is_ascii_uppercase())
        })
}

#[cfg(test)]
mod tests;
