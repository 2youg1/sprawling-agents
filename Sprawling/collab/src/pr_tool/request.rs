// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::CommitAttribution;
use kernel::{AxError, Payload};
use serde::{Deserialize, Serialize};

use crate::workshop::NodeId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenRequest {
    pub node: NodeId,
    pub implementer: String,
    pub branch: String,

    pub commit: String,
}

impl OpenRequest {
    pub fn payload(&self) -> Result<Payload, AxError> {
        Payload::of(self)
    }

    pub fn from_payload(data: &Payload) -> Result<OpenRequest, AxError> {
        data.read()
    }

    pub fn rejected_payload(&self, by: String, why: String) -> Result<Payload, AxError> {
        Payload::of(&RejectedRequest {
            request: self.clone(),
            by,
            why,
        })
    }

    pub fn merged_payload(
        &self,
        merge_commit: String,
        verified_by: String,
        by: CommitAttribution,
    ) -> Result<Payload, AxError> {
        Payload::of(&MergedRequest {
            node: self.node.clone(),
            implementer: self.implementer.clone(),
            branch: self.branch.clone(),
            reviewed_commit: self.commit.clone(),
            commit: merge_commit,
            verified_by,
            by,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergedRequest {
    pub node: NodeId,
    pub implementer: String,
    pub branch: String,

    #[serde(default)]
    pub reviewed_commit: String,

    pub commit: String,

    pub verified_by: String,

    #[serde(flatten)]
    pub by: CommitAttribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejectedRequest {
    #[serde(flatten)]
    pub request: OpenRequest,
    pub by: String,
    pub why: String,
}
