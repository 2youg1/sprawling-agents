// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What `Command::PrivacyOperation` asks for, and the result the host
//! keeps for it under its `idem` (`crates/wire/spec/Privacy.lean` §8-87).

use kernel::{AxError, IdemKey};
use serde::{Deserialize, Serialize};

use super::answer::PrivacyValue;
use super::{PrivacyControl, PrivacyFaultCode, PrivacySettlement};

/// Change one host privacy control, or check the operation that has no
/// conclusion: the payload of `Command::PrivacyOperation`. The city's
/// listener carries it out on the host, and its result is read back from
/// [`crate::PrivacyAnswer::outcomes`] under `idem`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PrivacyRequest {
    pub action: PrivacyAction,
    pub idem: IdemKey,
}

/// One change the person confirmed on the page. `expected` is the value
/// the page showed when the person confirmed; the host writes nothing when
/// its fresh read differs (`crates/sprawling/spec/Privacy/Confirmation.lean`
/// D58).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PrivacyAction {
    /// Write the control's value over `expected`.
    Apply {
        control: PrivacyControl,
        expected: PrivacyValue,
    },
    /// Put back the value this app's latest change of the control found.
    Restore {
        control: PrivacyControl,
        expected: PrivacyValue,
    },
    /// The person's check of the operation that has no conclusion:
    /// `expected` is the value the page showed for its control. Writes
    /// nothing.
    Reconcile { expected: PrivacyValue },
}

/// The result kept for one operation a page sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PrivacyOutcome {
    pub idem: IdemKey,
    pub action: PrivacyAction,
    pub result: PrivacyResult,
}

/// Where one operation stands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PrivacyResult {
    /// Accepted and not finished: it waits for an earlier operation, for
    /// administrator approval, or for its readback.
    Running,
    /// `operation` is the history's number for it, never zero.
    Applied {
        operation: u64,
    },
    Restored {
        operation: u64,
    },
    /// The target already held the control's value; nothing was written
    /// or recorded.
    AlreadyWritten,
    Reconciled {
        operation: u64,
        settlement: PrivacySettlement,
    },
    /// The change asked for did not happen, or its result is unconfirmed;
    /// `error` says which and what to do.
    Refused {
        code: PrivacyFaultCode,
        error: AxError,
    },
}
