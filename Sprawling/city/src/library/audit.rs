// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod execution;
pub use execution::{
    AuditFetchError, AuditReport, AuditRequest, HttpAudit, LocalScanner, SKILLS_SH_TIMEOUT,
    SKILLSPECTOR_TIMEOUT, ScannerAudit, audit_skill,
};

use kernel::event::record::AuditVerdict;
use kernel::{B3Hash, Seq};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditState {

    Unaudited,

    Audited { verdict: AuditVerdict, at: Seq },

    Stale { audited: B3Hash },
}

#[must_use]
pub fn audit_state(content: &B3Hash, audits: &[(B3Hash, AuditVerdict, Seq)]) -> AuditState {
    let had = audits
        .iter()
        .filter(|(_, verdict, _)| is_an_audit(*verdict));
    let latest_of_content = had
        .clone()
        .filter(|(digest, _, _)| digest == content)
        .max_by_key(|(_, _, at)| *at);
    match latest_of_content {
        Some((_, verdict, at)) => AuditState::Audited {
            verdict: *verdict,
            at: *at,
        },
        None => match had.max_by_key(|(_, _, at)| *at) {
            Some((audited, _, _)) => AuditState::Stale { audited: *audited },
            None => AuditState::Unaudited,
        },
    }
}

fn is_an_audit(verdict: AuditVerdict) -> bool {
    match verdict {
        AuditVerdict::Pass | AuditVerdict::Warn | AuditVerdict::Fail => true,
        AuditVerdict::Unreachable => false,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, reason = "test code")]
mod tests;
