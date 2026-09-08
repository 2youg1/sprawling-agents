// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a drive left, on the ledger before it is made true.

use kernel::AxError;
use kernel::Locator;

/// What the sweep after a drive works on.
///
/// The three arrive together because they answer one question - what a
/// wave left in the tree that the history has not accounted for yet: the
/// commits to restore a discarded file from, the escalations a gate
/// raised while the driver held the ledger, and the pin of the work an
/// escalation interrupted. `raised` is borrowed mutably because the
/// sweep adds to it: a discard a person has to answer is raised here
/// rather than during the drive.
pub(super) struct Sweep<'a> {
    pub(super) fenced: &'a [String],
    pub(super) raised: &'a mut Vec<kernel::ApprovalItem>,
    pub(super) job_locator: &'a Locator,
}

/// What one drive ended with, as the conclusion reads it.
///
/// The drive's own outcome stays a `Result` here rather than being
/// propagated: a run that failed still has a tree to give back and
/// approvals to file, and both are worse left undone than the failure
/// that caused them.
pub(super) struct Ending<'a> {
    pub(super) driven: Result<runtime::Run<runtime::run::Frozen>, AxError>,
    pub(super) raised: Vec<kernel::ApprovalItem>,
    pub(super) delegates: &'a std::sync::Arc<std::sync::Mutex<collab::DelegateDesk>>,
    pub(super) succession: &'a std::sync::Arc<std::sync::Mutex<runtime::SuccessionDesk>>,
}

pub(super) mod desks;
pub(super) mod landing;
#[cfg(test)]
mod tests;
