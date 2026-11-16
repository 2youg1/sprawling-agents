// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::SignalKind;
use kernel::{Address, AxError, GitOid, RunId, Seq, TimeMs};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Note {
    Refused {
        error: AxError,
        at: Seq,
    },

    Checkpointed {
        oid: GitOid,
        at: Seq,
    },

    Waiting {
        at: Seq,
        t: TimeMs,
        answered: Option<TimeMs>,
    },

    Arrived {
        from: Option<String>,
        said: Option<String>,
        by: Speaker,
        t: TimeMs,

        handback: Option<HandbackNote>,

        #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
        kind: Option<SignalKind>,

        session: Option<RunId>,
        at: Seq,
    },

    AwaitingReply {
        on: Address,
        until: TimeMs,
        t: TimeMs,
        ended: Option<ReplyEnded>,
        at: Seq,
    },

    Discarded {
        count: usize,
        at: Seq,
    },

    Unreadable {
        cause: String,
        at: Seq,
    },
}

impl Note {
    #[must_use]
    pub fn at(&self) -> Seq {
        match *self {
            Self::Refused { at, .. }
            | Self::Checkpointed { at, .. }
            | Self::Waiting { at, .. }
            | Self::Arrived { at, .. }
            | Self::AwaitingReply { at, .. }
            | Self::Discarded { at, .. }
            | Self::Unreadable { at, .. } => at,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Speaker {
    User,
    City,
    Resident,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum HandbackNote {
    Finished { verified_by: String },
    Stopped { because: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ReplyEnded {
    pub by: ReplyEnd,
    pub t: TimeMs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ReplyEnd {
    Reply,
    Timeout,
    Left,
}
