// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorScanning {

    DoesNotApply,

    Stopped,
    Read {

        city: String,
        drive: DoctorDrive,
        exclusion: DoctorExclusion,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorDrive {

    Trusted,

    Untrusted {
        volume: String,
    },

    Not {
        volume: String,
        file_system: String,
    },
    Untold {
        why: DoctorUntold,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorExclusion {

    Inside {
        under: String,
    },
    Outside,
    Untold {
        why: DoctorUntold,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorUntold {

    NoDisk,

    AdminOnly,

    Unread {
        command: String,
    },
    Failed {
        command: String,
        code: Option<i32>,
    },
    Unstarted {
        command: String,
    },
    Unanswered {
        command: String,
        stopping: Option<String>,
    },
}
