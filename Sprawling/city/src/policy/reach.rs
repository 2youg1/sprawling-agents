// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainReach {
    Everything,
    Documents,
}

impl DomainReach {
    pub(super) fn parse(value: &str) -> Result<DomainReach, AxError> {
        match value {
            "everything" => Ok(DomainReach::Everything),
            "documents" => Ok(DomainReach::Documents),
            other => Err(AxError::failure(
                AxCode::ConfigInvalid,
                "evaluate a building's rules",
                format!("`write: {other}` is neither everything nor documents"),
            )
            .with_recovery("write `write: everything` or `write: documents`")),
        }
    }
}
