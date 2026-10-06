// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Why a privacy operation did not make the change asked for, and the one
//! map from that to the error a person reads
//! (`crates/sprawling/spec/Privacy.lean` §12).

use std::num::NonZeroU64;

use kernel::{AxCode, AxError};
use wire::PrivacyControl;

use super::state::HistoryFault;

/// Why a read of a target failed.
#[derive(Debug)]
pub(super) enum ReadFault {
    AccessDenied,
    Failed(AxError),
}

/// Why a write of a target reported failure. It never decides the
/// outcome: the readback does.
#[derive(Debug)]
pub(super) enum WriteFault {
    AccessDenied,
    /// The person declined the UAC prompt.
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

/// The stable code of a [`PrivacyFault`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FaultCode {
    Identity,
    Clock,
    History,
    Unreadable,
    Unresolved,
    Changed,
    TargetAbsent,
    NothingOwned,
    Conflict,
    NothingUnresolved,
    HistoryFull,
    Expired,
    AccessDenied,
    ElevationDeclined,
    NotApplied,
    ReadbackMismatch,
    Unknown,
    ReceiptLost,
}

impl PrivacyFault {
    pub(super) fn code(&self) -> FaultCode {
        match self {
            Self::Identity(_) => FaultCode::Identity,
            Self::Clock(_) => FaultCode::Clock,
            Self::History(_) => FaultCode::History,
            Self::HistoryFull => FaultCode::HistoryFull,
            Self::Unreadable { .. } => FaultCode::Unreadable,
            Self::Unresolved => FaultCode::Unresolved,
            Self::Changed { .. } => FaultCode::Changed,
            Self::TargetAbsent { .. } => FaultCode::TargetAbsent,
            Self::NothingOwned { .. } => FaultCode::NothingOwned,
            Self::Conflict { .. } => FaultCode::Conflict,
            Self::NothingUnresolved => FaultCode::NothingUnresolved,
            Self::Expired { .. } => FaultCode::Expired,
            Self::NotApplied { cause, .. } => match cause {
                Some(WriteFault::AccessDenied) => FaultCode::AccessDenied,
                Some(WriteFault::Declined) => FaultCode::ElevationDeclined,
                Some(WriteFault::Failed(_)) | None => FaultCode::NotApplied,
            },
            Self::RolledBack { .. } => FaultCode::ReadbackMismatch,
            Self::Unknown { .. } => FaultCode::Unknown,
            Self::ReceiptLost { .. } => FaultCode::ReceiptLost,
        }
    }
}

impl FaultCode {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::Clock => "clock",
            Self::History => "history",
            Self::Unreadable => "unreadable",
            Self::Unresolved => "unresolved",
            Self::Changed => "changed",
            Self::TargetAbsent => "target_absent",
            Self::NothingOwned => "nothing_owned",
            Self::Conflict => "conflict",
            Self::NothingUnresolved => "nothing_unresolved",
            Self::HistoryFull => "history_full",
            Self::Expired => "expired",
            Self::AccessDenied => "access_denied",
            Self::ElevationDeclined => "elevation_declined",
            Self::NotApplied => "not_applied",
            Self::ReadbackMismatch => "readback_mismatch",
            Self::Unknown => "unknown",
            Self::ReceiptLost => "receipt_lost",
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
                format!("{code}: {control:?} could not be read ({fault:?})"),
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
            Self::Expired { control, .. } => (
                AxCode::Timeout,
                format!("{code}: the write of {control:?} did not start in time"),
                "nothing was written; read the page again and confirm once more",
            ),
            Self::NotApplied { control, cause, .. } => (
                match cause {
                    Some(WriteFault::AccessDenied | WriteFault::Declined) => AxCode::SandboxDenied,
                    Some(WriteFault::Failed(_)) | None => AxCode::ToolUnavailable,
                },
                format!("{code}: {control:?} still reads its original value ({cause:?})"),
                match cause {
                    Some(WriteFault::AccessDenied) => {
                        "this account may not write this setting; nothing changed"
                    }
                    Some(WriteFault::Declined) => {
                        "administrator approval was declined; nothing changed, approve the prompt to apply"
                    }
                    Some(WriteFault::Failed(_)) | None => {
                        "nothing changed; Windows kept the original value"
                    }
                },
            ),
            Self::RolledBack { control, .. } => (
                AxCode::EvidenceMissing,
                format!("{code}: {control:?} read back a value nobody asked for"),
                "the original value was written back and checked; read the page before trying again",
            ),
            Self::Unknown { control, why, .. } => (
                AxCode::ToolOutcomeUnknown,
                format!("{code}: {control:?} could not be confirmed ({why:?})"),
                "check the setting and reconcile the operation on the page; nothing is written until then",
            ),
            Self::ReceiptLost { control, fault, .. } => (
                AxCode::ToolOutcomeUnknown,
                format!("{code}: the outcome for {control:?} is not recorded ({fault:?})"),
                "the setting may have changed; read the page and reconcile the operation",
            ),
        };
        AxError::failure(ax, action, subject).with_recovery(recovery)
    }
}
