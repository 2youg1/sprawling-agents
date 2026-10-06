// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Why a privacy operation did not make the change asked for, or its
//! history could not be read or extended, and the one map from each to the
//! error a person reads
//! (`crates/sprawling/spec/Privacy.lean` §12).

use std::num::NonZeroU64;

use kernel::{AxCode, AxError};
use wire::{PrivacyControl, PrivacyFaultCode};

/// Why a history was not read or a line not written. Reasons stay
/// distinct until a caller maps them to AxError with its own action.
#[derive(Debug)]
pub(super) enum HistoryFault {
    Io(std::io::Error),
    Decode {
        line: usize,
        column: usize,
        category: serde_json::error::Category,
    },
    Invalid(&'static str),
    Busy,
    Capacity,
}

impl HistoryFault {
    /// The error a caller that was doing `action` reports.
    pub(super) fn into_ax(self, action: &'static str) -> AxError {
        let (code, subject, recovery) = match self {
            Self::Io(source) => (
                AxCode::StorageFatal,
                format!("history IO: {source}"),
                "check local history access and retry",
            ),
            Self::Decode {
                line,
                column,
                category,
            } => (
                AxCode::StorageFatal,
                format!("history decoding at line {line} column {column} ({category:?})"),
                "preserve the history bytes and inspect the malformed line; do not truncate it",
            ),
            Self::Invalid(reason) => (
                AxCode::StorageFatal,
                reason.to_owned(),
                "preserve the history and resolve its incompatible or unresolved operation before any write",
            ),
            Self::Busy => (
                AxCode::LedgerHeld,
                "privacy history is held by another process".to_owned(),
                "wait for that operation to end, then read status again",
            ),
            Self::Capacity => (
                AxCode::StorageFatal,
                "privacy history exceeds its reader capacity".to_owned(),
                "preserve the history and use a reader that supports its size; do not remove original values",
            ),
        };
        AxError::failure(code, action, subject).with_recovery(recovery)
    }
}

/// Why a read of a target failed.
#[derive(Debug)]
pub(crate) enum ReadFault {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the Windows adapters construct it and are not routed to the production host yet (Privacy.Service D68)"
        )
    )]
    AccessDenied,
    Failed(AxError),
}

/// Why a write of a target reported failure. It never decides the
/// outcome: the readback does.
#[derive(Debug)]
pub(crate) enum WriteFault {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the Windows adapters construct it and are not routed to the production host yet (Privacy.Service D68)"
        )
    )]
    AccessDenied,
    /// The person declined the UAC prompt.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the Windows adapters construct it and are not routed to the production host yet (Privacy.Service D68)"
        )
    )]
    Declined,
    Failed(AxError),
}

/// Why a privacy operation ended without the change asked for: the closed
/// set of stable codes in `crates/sprawling/spec/Privacy.lean` §12.
#[derive(Debug)]
pub(super) enum PrivacyFault {
    Identity(AxError),
    Clock(AxError),
    History(HistoryFault),
    HistoryFull,
    Unreadable {
        control: PrivacyControl,
        fault: ReadFault,
    },
    Unresolved,
    Changed {
        control: PrivacyControl,
    },
    TargetAbsent {
        control: PrivacyControl,
    },
    NothingOwned {
        control: PrivacyControl,
    },
    Conflict {
        control: PrivacyControl,
    },
    NothingUnresolved,
    /// The deadline passed before the write; `operation` is the recorded
    /// operation when its `Prepared` was already durable.
    Expired {
        control: PrivacyControl,
        operation: Option<NonZeroU64>,
    },
    /// The target still reads its original; `cause` is what the write
    /// reported, if anything.
    NotApplied {
        control: PrivacyControl,
        operation: NonZeroU64,
        cause: Option<WriteFault>,
    },
    /// The readback was a third value and the original was written back.
    RolledBack {
        control: PrivacyControl,
        operation: NonZeroU64,
    },
    Unknown {
        control: PrivacyControl,
        operation: NonZeroU64,
        why: Unconfirmed,
    },
    /// The host may have been written but the receipt is not on disk.
    ReceiptLost {
        control: PrivacyControl,
        operation: NonZeroU64,
        fault: HistoryFault,
    },
}

/// Why an operation ended unknown.
#[derive(Debug)]
pub(super) enum Unconfirmed {
    ReadbackUnreadable(ReadFault),
    RollbackMissed(Option<WriteFault>),
    RollbackUnreadable(ReadFault),
}

impl PrivacyFault {
    pub(super) fn code(&self) -> PrivacyFaultCode {
        match self {
            Self::Identity(_) => PrivacyFaultCode::Identity,
            Self::Clock(_) => PrivacyFaultCode::Clock,
            Self::History(_) => PrivacyFaultCode::History,
            Self::HistoryFull => PrivacyFaultCode::HistoryFull,
            Self::Unreadable { .. } => PrivacyFaultCode::Unreadable,
            Self::Unresolved => PrivacyFaultCode::Unresolved,
            Self::Changed { .. } => PrivacyFaultCode::Changed,
            Self::TargetAbsent { .. } => PrivacyFaultCode::TargetAbsent,
            Self::NothingOwned { .. } => PrivacyFaultCode::NothingOwned,
            Self::Conflict { .. } => PrivacyFaultCode::Conflict,
            Self::NothingUnresolved => PrivacyFaultCode::NothingUnresolved,
            Self::Expired { .. } => PrivacyFaultCode::Expired,
            Self::NotApplied { cause, .. } => match cause {
                Some(WriteFault::AccessDenied) => PrivacyFaultCode::AccessDenied,
                Some(WriteFault::Declined) => PrivacyFaultCode::ElevationDeclined,
                Some(WriteFault::Failed(_)) | None => PrivacyFaultCode::NotApplied,
            },
            Self::RolledBack { .. } => PrivacyFaultCode::ReadbackMismatch,
            Self::Unknown { .. } => PrivacyFaultCode::Unknown,
            Self::ReceiptLost { .. } => PrivacyFaultCode::ReceiptLost,
        }
    }
}

impl PrivacyFault {
    /// The one map from a privacy fault to the error a person reads: an
    /// existing code, the stable privacy code first in the subject, and the
    /// recovery that answers it.
    pub(super) fn into_ax(self, action: &'static str) -> AxError {
        let code = self.code().as_str();
        let (ax, subject, recovery) = match self {
            Self::Identity(error) | Self::Clock(error) => return error,
            Self::History(fault) => return fault.into_ax(action),
            Self::HistoryFull => (
                AxCode::StorageFatal,
                format!("{code}: the privacy history has no operation number left"),
                "keep the history; nothing was written",
            ),
            Self::Unreadable { control, fault } => (
                match fault {
                    ReadFault::AccessDenied => AxCode::SandboxDenied,
                    ReadFault::Failed(_) => AxCode::ToolUnavailable,
                },
                format!("{code}: {control:?} could not be read: {fault}"),
                "nothing was written; check that this account may read the setting and ask again",
            ),
            Self::Unresolved => (
                AxCode::ToolOutcomeUnknown,
                format!("{code}: an earlier privacy operation has no conclusion"),
                "check the setting the page shows for that operation and reconcile it; nothing is written until then",
            ),
            Self::Changed { control } => (
                AxCode::VersionConflict,
                format!("{code}: {control:?} no longer reads the value you confirmed"),
                "nothing was written; read the page again and confirm the value it shows now",
            ),
            Self::TargetAbsent { control } => (
                AxCode::PathNotFound,
                format!("{code}: this host has no target for {control:?}"),
                "nothing was written; this host does not have what this control changes",
            ),
            Self::NothingOwned { control } => (
                AxCode::InvalidArgs,
                format!("{code}: this app owns no change of {control:?}"),
                "nothing was written; only a value this app wrote can be restored",
            ),
            Self::Conflict { control } => (
                AxCode::VersionConflict,
                format!("{code}: {control:?} was changed after this app wrote it"),
                "nothing was written; the later value is not overwritten, change it in Windows if you want the original back",
            ),
            Self::NothingUnresolved => (
                AxCode::InvalidArgs,
                format!("{code}: no privacy operation waits for a check"),
                "nothing to reconcile; read the page again",
            ),
            Self::Expired { control, operation } => (
                AxCode::Timeout,
                match operation {
                    None => format!("{code}: the write of {control:?} did not start in time"),
                    Some(operation) => format!(
                        "{code}: the write of {control:?} did not start in time; operation {operation} is recorded as not applied"
                    ),
                },
                "nothing was written; read the page again and confirm once more",
            ),
            Self::NotApplied {
                control,
                operation,
                cause,
            } => (
                match cause {
                    Some(WriteFault::AccessDenied | WriteFault::Declined) => AxCode::SandboxDenied,
                    Some(WriteFault::Failed(_)) | None => AxCode::ToolUnavailable,
                },
                match &cause {
                    Some(fault) => format!(
                        "{code}: {control:?} still reads its original value after operation {operation}; the write reported: {fault}"
                    ),
                    None => format!(
                        "{code}: {control:?} still reads its original value after operation {operation}"
                    ),
                },
                match cause {
                    Some(WriteFault::AccessDenied) => {
                        "this account may not write this policy key; nothing changed"
                    }
                    Some(WriteFault::Declined) => {
                        "administrator approval was declined; nothing changed, approve the prompt to apply"
                    }
                    Some(WriteFault::Failed(_)) | None => {
                        "nothing changed; Windows kept the original value"
                    }
                },
            ),
            Self::RolledBack { control, operation } => (
                AxCode::EvidenceMissing,
                format!(
                    "{code}: {control:?} read back a value nobody asked for in operation {operation}"
                ),
                "the original value was written back and checked; read the page before trying again",
            ),
            Self::Unknown {
                control,
                operation,
                why,
            } => (
                AxCode::ToolOutcomeUnknown,
                format!(
                    "{code}: operation {operation} on {control:?} could not be confirmed: {why}"
                ),
                "check the setting and reconcile the operation on the page; nothing is written until then",
            ),
            Self::ReceiptLost {
                control,
                operation,
                fault,
            } => (
                AxCode::ToolOutcomeUnknown,
                format!(
                    "{code}: the outcome of operation {operation} on {control:?} is not recorded: {}",
                    fault.into_ax(action).subject()
                ),
                "the setting may have changed; read the page and reconcile the operation",
            ),
        };
        AxError::failure(ax, action, subject).with_recovery(recovery)
    }
}

impl std::fmt::Display for ReadFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AccessDenied => f.write_str("access denied"),
            Self::Failed(error) => write!(f, "{error}"),
        }
    }
}

impl std::fmt::Display for WriteFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AccessDenied => f.write_str("access denied"),
            Self::Declined => f.write_str("administrator approval declined"),
            Self::Failed(error) => write!(f, "{error}"),
        }
    }
}

impl std::fmt::Display for Unconfirmed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadbackUnreadable(fault) => write!(f, "the readback failed: {fault}"),
            Self::RollbackMissed(None) => {
                f.write_str("the original was written back but does not read back")
            }
            Self::RollbackMissed(Some(fault)) => {
                write!(f, "writing the original back failed: {fault}")
            }
            Self::RollbackUnreadable(fault) => {
                write!(
                    f,
                    "the read after writing the original back failed: {fault}"
                )
            }
        }
    }
}
