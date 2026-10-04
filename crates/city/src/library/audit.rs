// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The audit state of one skill: a reading of its audits against the
//! digest it carries now, never a stored flag, so a change to the content
//! makes an earlier audit stale by itself.
//!
//! Specified by `crates/city/spec/Library/Audit.lean` §8-28b (city D19).

mod execution;
pub use execution::{
    AuditFetchError, AuditReport, AuditRequest, HttpAudit, SKILLS_SH_TIMEOUT, SKILLSPECTOR_TIMEOUT,
    ScannerAudit, audit_skill,
};

use kernel::event::record::AuditVerdict;
use kernel::{B3Hash, Seq};

/// What the shelf page shows about one skill's audits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditState {
    /// No audit was ever had for any content of this skill.
    Unaudited,
    /// The latest audit of the content the skill carries now.
    Audited { verdict: AuditVerdict, at: Seq },
    /// Audits exist, none of them for the content the skill carries now;
    /// `audited` is the digest the latest of them judged.
    Stale { audited: B3Hash },
}

/// The audit state of a skill whose content digests to `content`, given
/// every `skill_audited` line of it as `(digest, verdict, seq)`.
///
/// An `Unreachable` line records that no audit was had, so it is not an
/// audit of anything: it never makes a skill show as audited, and it does
/// not make one stale. The latest line is the one with the highest
/// `seq`, whatever order the slice holds them in.
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
