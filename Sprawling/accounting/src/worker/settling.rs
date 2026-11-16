// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::AxError;
use kernel::Locator;

pub(super) struct Sweep<'a> {

    pub(super) swept: Option<super::dispatching::heavy_landing::Swept>,
    pub(super) raised: &'a mut Vec<kernel::ApprovalItem>,
    pub(super) job_locator: &'a Locator,
}

pub(super) struct Settling<'a> {
    pub(super) sweep: Sweep<'a>,
    pub(super) chain: super::KnockChain,

    pub(super) open_claims: &'a mut crate::worker::booking::OpenClaims,
}

pub(super) struct Ending<'a> {
    pub(super) driven: Result<runtime::Run<runtime::run::Frozen>, AxError>,
    pub(super) raised: Vec<kernel::ApprovalItem>,
    pub(super) succession: &'a std::sync::Arc<std::sync::Mutex<runtime::SuccessionDesk>>,

    pub(super) owing: super::Owing,
}

pub(super) mod desks;
pub(super) mod landing;
#[cfg(test)]
mod tests;
