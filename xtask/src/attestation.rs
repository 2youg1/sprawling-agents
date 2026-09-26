// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Every archive the release attaches to a tag carries a build-provenance
//! attestation before it is attached (xtask-SPEC §8-34).

use std::path::Path;

use crate::platform::WORKFLOW;
use crate::report::{Violation, XtaskError};
use crate::walk;

/// Where the release workflow attests less than it attaches.
///
/// # Errors
/// When the release workflow cannot be read.
pub(crate) fn unattested(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    Ok(findings(&walk::read_text(&root.join(WORKFLOW))?)
        .into_iter()
        .map(|violation| Violation {
            gate: "artifact",
            location: WORKFLOW.to_owned(),
            rule: "every archive a release attaches is attested before it is attached".to_owned(),
            violation,
            alternative: "in the job that runs `gh release create`, grant `id-token: write` and \
                          `attestations: write`, and run `actions/attest-build-provenance` \
                          on the same glob before the release is created"
                .to_owned(),
        })
        .collect())
}

/// The action whose step writes the attestation.
const ATTEST: &str = "actions/attest-build-provenance@";

/// The command that attaches archives to the tag.
const ATTACH: &str = "gh release create";

/// The permissions the attesting job cannot mint an attestation without.
const PERMISSIONS: [&str; 2] = ["id-token: write", "attestations: write"];

/// What the job that attaches archives leaves unattested, one sentence
/// per fact of xtask-SPEC §8-34.
///
/// Read by shape: a job opens with a two-space-indented `name:` line and
/// runs to the next one, and only the lines of the attaching job count,
/// because a permission another job holds grants this one nothing.
fn findings(workflow: &str) -> Vec<String> {
    let lines: Vec<&str> = workflow.lines().collect();
    let Some(job) = attaching_job(&lines) else {
        return vec![format!(
            "no job runs `{ATTACH}`, so nothing says what the release attaches"
        )];
    };
    let mut out: Vec<String> = PERMISSIONS
        .iter()
        .filter(|grant| !job.iter().any(|line| line.trim() == **grant))
        .map(|grant| format!("the job that runs `{ATTACH}` does not grant `{grant}`"))
        .collect();
    let attach = job.iter().position(|line| runs_attach(line));
    let Some(attest) = job.iter().position(|line| line.contains(ATTEST)) else {
        out.push(format!(
            "no step of the job that runs `{ATTACH}` uses `{ATTEST}`"
        ));
        return out;
    };
    let subject = job
        .iter()
        .skip(attest)
        .find_map(|line| line.trim().strip_prefix("subject-path:"))
        .map(str::trim);
    let attached = attach.map(|index| attached_words(&job, index));
    match (subject, attached) {
        (Some(glob), Some(words)) if words.contains(&glob) => {}
        (Some(glob), _) => {
            out.push(format!(
                "the attestation covers `{glob}`, which `{ATTACH}` does not attach"
            ));
        }
        (None, _) => out.push(format!("the `{ATTEST}` step names no `subject-path`")),
    }
    if attach.is_some_and(|index| index < attest) {
        out.push(format!(
            "the attestation is written after `{ATTACH}` attaches the archives"
        ));
    }
    out
}

/// Whether this line runs the command, rather than a comment naming it.
fn runs_attach(line: &str) -> bool {
    line.trim_start().starts_with(ATTACH)
}

/// The words of the command that starts on line `from`, read through every
/// line it continues onto with a trailing `\`.
fn attached_words<'a>(job: &[&'a str], from: usize) -> Vec<&'a str> {
    let mut words = Vec::new();
    for line in job.iter().skip(from) {
        let line = line.trim_end();
        let text = line.strip_suffix('\\');
        words.extend(text.unwrap_or(line).split_whitespace());
        if text.is_none() {
            break;
        }
    }
    words
}

/// The lines of the job whose steps run `gh release create`.
fn attaching_job<'a>(lines: &[&'a str]) -> Option<Vec<&'a str>> {
    let opens = |line: &&str| {
        line.strip_prefix("  ")
            .is_some_and(|rest| !rest.starts_with([' ', '#']) && rest.trim_end().ends_with(':'))
    };
    let mut starts: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| opens(line))
        .map(|(index, _)| index)
        .collect();
    starts.push(lines.len());
    starts.windows(2).find_map(|pair| {
        let job = lines.get(*pair.first()?..*pair.get(1)?)?;
        job.iter()
            .any(|line| runs_attach(line))
            .then(|| job.to_vec())
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests;
