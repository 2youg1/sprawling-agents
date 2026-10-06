// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Error codes: the closed AxCode set and its carrier events.
//! The part `crates/kernel/spec/Error.lean` specifies this module with the
//! rest of `kernel::error`.

use serde::{Deserialize, Serialize};

use crate::event::EventKind;

/// Where an AxCode surfaces in history: its carrier event, or the loadtime
/// class (the closed C9 exception: the Ledger is not writable yet).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Carrier {
    Event(EventKind),
    Loadtime,
}

/// Closed set of error codes.
/// Extension is additive only; the wire spelling lives in [`AxCode::as_str`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AxCode {
    // Base table (15).
    PathNotFound,
    ToolUnknown,
    ToolUnavailable,
    InvalidArgs,
    OutsideWriteDomain,
    VersionConflict,
    GateDenied,
    BudgetExhausted,
    Timeout,
    Provider,
    EvidenceMissing,
    LoopSuspected,
    LocatorInvalid,
    SandboxDenied,
    /// The address or resource this names is taken right now.
    ///
    /// Distinct from `WorktreeBusy` and `RepairBusy`, which are
    /// about those two mechanisms in particular: this one says a
    /// *run* is working where the caller wants to act, and names the
    /// run so the caller can stop it and act.
    Busy,
    // Collaboration (5). A sixth, `SignalUnknown`, was defined away:
    // signal payloads are written and read by one module, the kind is
    // an exhaustive enum, and a kind this version does not know
    // can only come from a newer binary's ledger — which the version
    // door already refuses.
    DraftStale,
    GoalConflict,
    TaintedAction,
    RepairBusy,
    DelegationDepth,
    // Governance and facilities (15).
    ApprovalPending,
    ApprovalDenied,
    CrossBuildingDenied,
    DigestSuspect,
    CredentialMissing,
    /// No model is chosen for the tag a caller asked for. Apart from
    /// `ConfigInvalid` because its recovery differs: a new city starts
    /// here and is sent to the settings page, while `ConfigInvalid`
    /// also answers a room whose frozen model changed, which a new
    /// session recovers.
    ModelUnchosen,
    ConfigInvalid,
    CasCorrupt,
    StorageFatal,
    WorktreeBusy,
    BrowserUnavailable,
    EndpointDialectUnsupported,
    WireMismatch,
    LogVersionUnsupported,
    /// Another process holds this city's Ledger for writing. Loadtime,
    /// because the process it refuses is the one that may not write.
    LedgerHeld,
    /// A served city is still proving the history it opened from, and
    /// its writer takes no line until the proof is whole. Loadtime for
    /// the same reason as `LedgerHeld`: the refused process may not write
    /// yet (`crates/storage/Spec.lean` §8-30).
    HistoryUnproven,
    // Privacy and Discard (2).
    SecretEgress,
    DiscardIrreversible,
    // Backpressure (1). A shed signal is a refused delivery, not a lost
    // one: the line is on the ledger, the queue never took it, and the
    // caller learns it here rather than by reading the queue back.
    BackpressureShed,
    // Unknown outcome (1).
    ToolOutcomeUnknown,
    // Plan (1). A standing goal set on a building with no plan; the
    // subject is always `<building address>: <goal>`, which is the shape
    // the client's form recovery reads.
    PlanMissing,
}

impl AxCode {
    /// Every code, in the order the SPEC table lists them. Data face for tests and
    /// `xtask specalign`.
    pub const ALL: [AxCode; 41] = [
        AxCode::PathNotFound,
        AxCode::ToolUnknown,
        AxCode::ToolUnavailable,
        AxCode::InvalidArgs,
        AxCode::OutsideWriteDomain,
        AxCode::VersionConflict,
        AxCode::GateDenied,
        AxCode::BudgetExhausted,
        AxCode::Timeout,
        AxCode::Provider,
        AxCode::EvidenceMissing,
        AxCode::LoopSuspected,
        AxCode::LocatorInvalid,
        AxCode::SandboxDenied,
        AxCode::Busy,
        AxCode::DraftStale,
        AxCode::GoalConflict,
        AxCode::TaintedAction,
        AxCode::RepairBusy,
        AxCode::DelegationDepth,
        AxCode::ApprovalPending,
        AxCode::ApprovalDenied,
        AxCode::CrossBuildingDenied,
        AxCode::DigestSuspect,
        AxCode::CredentialMissing,
        AxCode::ModelUnchosen,
        AxCode::ConfigInvalid,
        AxCode::CasCorrupt,
        AxCode::StorageFatal,
        AxCode::WorktreeBusy,
        AxCode::BrowserUnavailable,
        AxCode::EndpointDialectUnsupported,
        AxCode::WireMismatch,
        AxCode::LogVersionUnsupported,
        AxCode::LedgerHeld,
        AxCode::HistoryUnproven,
        AxCode::SecretEgress,
        AxCode::DiscardIrreversible,
        AxCode::BackpressureShed,
        AxCode::ToolOutcomeUnknown,
        AxCode::PlanMissing,
    ];

    /// The wire spelling. Sole spelling authority; serde and Display reuse it.
    pub fn as_str(&self) -> &'static str {
        match self {
            AxCode::PathNotFound => "E_PATH_NOT_FOUND",
            AxCode::ToolUnknown => "E_TOOL_UNKNOWN",
            AxCode::ToolUnavailable => "E_TOOL_UNAVAILABLE",
            AxCode::InvalidArgs => "E_INVALID_ARGS",
            AxCode::OutsideWriteDomain => "E_OUTSIDE_WRITE_DOMAIN",
            AxCode::VersionConflict => "E_VERSION_CONFLICT",
            AxCode::GateDenied => "E_GATE_DENIED",
            AxCode::BudgetExhausted => "E_BUDGET_EXHAUSTED",
            AxCode::Timeout => "E_TIMEOUT",
            AxCode::Provider => "E_PROVIDER",
            AxCode::EvidenceMissing => "E_EVIDENCE_MISSING",
            AxCode::LoopSuspected => "E_LOOP_SUSPECTED",
            AxCode::LocatorInvalid => "E_LOCATOR_INVALID",
            AxCode::SandboxDenied => "E_SANDBOX_DENIED",
            AxCode::Busy => "E_BUSY",
            AxCode::DraftStale => "E_DRAFT_STALE",
            AxCode::GoalConflict => "E_GOAL_CONFLICT",
            AxCode::TaintedAction => "E_TAINTED_ACTION",
            AxCode::RepairBusy => "E_REPAIR_BUSY",
            AxCode::DelegationDepth => "E_DELEGATION_DEPTH",
            AxCode::ApprovalPending => "E_APPROVAL_PENDING",
            AxCode::ApprovalDenied => "E_APPROVAL_DENIED",
            AxCode::CrossBuildingDenied => "E_CROSS_BUILDING_DENIED",
            AxCode::DigestSuspect => "E_DIGEST_SUSPECT",
            AxCode::CredentialMissing => "E_CREDENTIAL_MISSING",
            AxCode::ModelUnchosen => "E_MODEL_UNCHOSEN",
            AxCode::ConfigInvalid => "E_CONFIG_INVALID",
            AxCode::CasCorrupt => "E_CAS_CORRUPT",
            AxCode::StorageFatal => "E_STORAGE_FATAL",
            AxCode::WorktreeBusy => "E_WORKTREE_BUSY",
            AxCode::BrowserUnavailable => "E_BROWSER_UNAVAILABLE",
            AxCode::EndpointDialectUnsupported => "E_ENDPOINT_DIALECT_UNSUPPORTED",
            AxCode::WireMismatch => "E_WIRE_MISMATCH",
            AxCode::LogVersionUnsupported => "E_LOG_VERSION_UNSUPPORTED",
            AxCode::LedgerHeld => "E_LEDGER_HELD",
            AxCode::HistoryUnproven => "E_HISTORY_UNPROVEN",
            AxCode::SecretEgress => "E_SECRET_EGRESS",
            AxCode::BackpressureShed => "E_BACKPRESSURE_SHED",
            AxCode::DiscardIrreversible => "E_DISCARD_IRREVERSIBLE",
            AxCode::ToolOutcomeUnknown => "E_TOOL_OUTCOME_UNKNOWN",
            AxCode::PlanMissing => "E_PLAN_MISSING",
        }
    }

    /// Wire spelling back to code; `None` is the caller's fail-closed branch.
    pub fn parse(raw: &str) -> Option<AxCode> {
        AxCode::ALL.into_iter().find(|code| code.as_str() == raw)
    }

    /// The carrier declaration (C9): which event carries this code into
    /// history. Sole declaration site, exhaustive on purpose — a new code
    /// without a carrier decision is a compile error. The loadtime arm is
    /// the closed whitelist of codes that arise only while this process
    /// cannot write the Ledger; `crates/kernel/Spec.lean` §12 gives each one's
    /// reason.
    pub fn carrier(&self) -> Carrier {
        match self {
            AxCode::GateDenied
            | AxCode::OutsideWriteDomain
            | AxCode::TaintedAction
            | AxCode::CrossBuildingDenied
            | AxCode::DiscardIrreversible
            | AxCode::SecretEgress
            | AxCode::DelegationDepth => Carrier::Event(EventKind::GateDenied),
            AxCode::ApprovalPending => Carrier::Event(EventKind::ApprovalRequested),
            AxCode::ApprovalDenied => Carrier::Event(EventKind::ApprovalResolved),
            AxCode::BudgetExhausted => Carrier::Event(EventKind::BudgetLimit),
            AxCode::LoopSuspected => Carrier::Event(EventKind::WatchdogFired),
            AxCode::Provider => Carrier::Event(EventKind::ProviderDegraded),
            AxCode::EndpointDialectUnsupported => Carrier::Event(EventKind::EndpointLost),
            AxCode::ConfigInvalid
            | AxCode::CasCorrupt
            | AxCode::StorageFatal
            | AxCode::WireMismatch
            | AxCode::LogVersionUnsupported
            | AxCode::LedgerHeld
            | AxCode::HistoryUnproven => Carrier::Loadtime,
            AxCode::PathNotFound
            | AxCode::ToolUnknown
            | AxCode::ToolUnavailable
            | AxCode::InvalidArgs
            | AxCode::VersionConflict
            | AxCode::Timeout
            | AxCode::EvidenceMissing
            | AxCode::LocatorInvalid
            | AxCode::SandboxDenied
            | AxCode::Busy
            | AxCode::DraftStale
            | AxCode::GoalConflict
            | AxCode::RepairBusy
            | AxCode::DigestSuspect
            | AxCode::CredentialMissing
            | AxCode::ModelUnchosen
            | AxCode::WorktreeBusy
            | AxCode::BrowserUnavailable
            | AxCode::ToolOutcomeUnknown
            | AxCode::BackpressureShed
            | AxCode::PlanMissing => Carrier::Event(EventKind::ToolResult),
        }
    }
}

impl std::fmt::Display for AxCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for AxCode {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for AxCode {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        AxCode::parse(&raw)
            .ok_or_else(|| serde::de::Error::custom(format_args!("unknown AxCode `{raw}`")))
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn axcode_is_the_declared_length_and_spelling_is_bijective() {
        // The length is the close of the table, so it is stated once:
        // a code added without a spelling, or two codes sharing one,
        // fails here rather than at a caller.
        assert_eq!(AxCode::ALL.len(), 42);
        let spellings: BTreeSet<&str> = AxCode::ALL.iter().map(AxCode::as_str).collect();
        assert_eq!(spellings.len(), 42);
        for s in &spellings {
            assert!(s.starts_with("E_"));
        }
    }

    #[test]
    fn axcode_parse_roundtrips_every_variant() {
        for code in AxCode::ALL {
            assert_eq!(AxCode::parse(code.as_str()), Some(code));
        }
        assert_eq!(AxCode::parse("E_NO_SUCH_CODE"), None);
    }

    #[test]
    fn axcode_serde_uses_the_wire_spelling() {
        let json = serde_json::to_string(&AxCode::PathNotFound).unwrap();
        assert_eq!(json, "\"E_PATH_NOT_FOUND\"");
        let back: AxCode = serde_json::from_str("\"E_LOCATOR_INVALID\"").unwrap();
        assert_eq!(back, AxCode::LocatorInvalid);
        assert!(serde_json::from_str::<AxCode>("\"E_BOGUS\"").is_err());
    }
}
