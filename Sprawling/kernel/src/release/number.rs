// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::error::{AxCode, AxError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    maturity: u32,
    repository: u32,
    package: u32,
}

impl Version {
    pub fn from_crates_version(text: &str) -> Result<Version, AxError> {
        Version::parse(text, &|msg| {
            AxError::failure(
                AxCode::ConfigInvalid,
                "read a crates.io version",
                text.to_owned(),
            )
            .with_recovery(msg)
        })
    }

    pub(super) fn parse(
        version: &str,
        refused: &dyn Fn(String) -> AxError,
    ) -> Result<Version, AxError> {
        let mut parts = version.split('.');
        let mut next_number = || parts.next().and_then(number);
        let (Some(maturity), Some(repository), Some(package)) =
            (next_number(), next_number(), next_number())
        else {
            return Err(refused(format!(
                "`{version}` is not three dot-separated numbers, as in `0.17.0`"
            )));
        };
        if parts.next().is_some() {
            return Err(refused(format!(
                "`{version}` carries more than the three numbers a version of \
                 this project has"
            )));
        }
        Ok(Version {
            maturity,
            repository,
            package,
        })
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Version {
            maturity,
            repository,
            package,
        } = *self;
        write!(f, "{maturity}.{repository}.{package}")
    }
}

fn number(text: &str) -> Option<u32> {
    let mut chars = text.chars();
    let first = chars.next()?;
    if !first.is_ascii_digit() {
        return None;
    }
    if first == '0' && chars.next().is_some() {
        return None;
    }
    text.parse().ok()
}
