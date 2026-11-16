// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::PathBuf;

use kernel::{Address, AxError};

use super::holding::Views;

pub struct GitStatusAsk {
    city_root: PathBuf,
    building: Address,
    checkpoint: Option<wire::CommitAnswer>,
}

impl Views {

    pub(super) fn git_status_ask(&self, building: &Address) -> Result<GitStatusAsk, AxError> {
        Ok(GitStatusAsk {
            city_root: self.city_root.clone(),
            building: building.clone(),

            checkpoint: self
                .commits_answer(Some(building), None, 1)?
                .commits
                .into_iter()
                .next(),
        })
    }
}

impl GitStatusAsk {

    pub(super) fn read(self) -> wire::Answer {
        let Ok(status) = storage::working_status(
            &self.city_root,
            Some(self.building.as_str()),
            self.checkpoint.as_ref().map(|commit| commit.oid),
        ) else {
            return super::prepared::unavailable(format!("GitStatus({})", self.building.as_str()));
        };
        wire::Answer::GitStatus(Box::new(wire::GitStatusAnswer {
            building: self.building,
            branch: status.branch,
            drift: status.drift.map(|drift| wire::Drift {
                ahead: drift.ahead,
                behind: drift.behind,
            }),
            files: status.files,
            checkpoint: self.checkpoint,
        }))
    }
}
