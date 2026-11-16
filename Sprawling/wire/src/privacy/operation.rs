// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxError, IdemKey};
use serde::{Deserialize, Serialize};

use super::answer::PrivacyValue;
use super::{PrivacyControl, PrivacyFaultCode, PrivacySettlement};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PrivacyRequest {
    pub action: PrivacyAction,
    pub idem: IdemKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PrivacyAction {

    Apply {
        control: PrivacyControl,
        expected: PrivacyValue,
    },

    Restore {
        control: PrivacyControl,
        expected: PrivacyValue,
    },

    Reconcile { expected: PrivacyValue },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PrivacyOutcome {
    pub idem: IdemKey,
    pub action: PrivacyAction,
    pub result: PrivacyResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PrivacyResult {

    Running,

    Applied {
        operation: u64,
    },
    Restored {
        operation: u64,
    },

    AlreadyWritten,
    Reconciled {
        operation: u64,
        settlement: PrivacySettlement,
    },

    Refused {
        code: PrivacyFaultCode,
        error: AxError,
    },
}
