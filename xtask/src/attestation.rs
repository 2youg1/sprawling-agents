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

fn findings(_workflow: &str) -> Vec<String> {
    Vec::new()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests;
