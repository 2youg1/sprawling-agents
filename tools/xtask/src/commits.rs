// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The subject line and the ruling trailer of every commit in a range,
//! and on a pull request the title, which a squash merge of two or more
//! commits turns into the subject main receives
//! (tools/xtask/Spec.lean §8-35). Not a gate: gates judge the tree, and this judges
//! history, so it runs only where a caller names the range.

use std::path::Path;
use std::process::Command;

use crate::report::{Violation, XtaskError};

/// The one spelling a ruling trailer may take (AGENTS.md, *Commits*).
const RULING: &str = "Verdict: user-approved";

/// What one run judges (§8-35): a range of history, or a range plus
/// the pull request title, which its squash merge would land on main
/// as the subject. The title is never judged alone — no caller has
/// one without a range.
pub(crate) enum Target<'a> {
    Range(&'a str),
    RangeAndTitle { range: &'a str, title: &'a str },
}

/// Where the target breaks the message rules of AGENTS.md.
///
/// # Errors
/// When git cannot be started or refuses the range.
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

/// Where the commits in `range` break the message rules of AGENTS.md.
///
/// # Errors
/// When git cannot be started or refuses the range.
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

/// One finding dressed for the report: the gate, the rule it broke,
/// and the recovery a reader can act on.
fn violation_for(location: String, violation: String, alternative: &str) -> Violation {
    Violation {
        gate: "commits",
        location,
        rule: format!("a subject starts `card-S<stage>.<index>: `; a ruling is `{RULING}`"),
        violation,
        alternative: alternative.to_owned(),
    }
}

/// Every way one commit message breaks the rules, one sentence each.
fn findings(message: &str) -> Vec<String> {
    let subject = message.lines().next().unwrap_or_default();
    let carded = (!carded(subject)).then(|| {
        format!(
            "the subject `{subject}` does not start with `card-S<digits>.<digits or capitals>: `"
        )
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

/// Every way a pull request title breaks the subject rule. The title
/// is one subject line: it has no body, so no ruling trailer is read.
fn title_findings(title: &str) -> Vec<String> {
    (!carded(title))
        .then(|| {
            format!(
                "the pull request title `{title}` does not start with `card-S<digits>.<digits or capitals>: `"
            )
        })
        .into_iter()
        .collect()
}

/// Whether a subject matches `^card-S\d+\.[0-9A-Z]+: `.
fn carded(subject: &str) -> bool {
    subject
        .strip_prefix("card-S")
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
