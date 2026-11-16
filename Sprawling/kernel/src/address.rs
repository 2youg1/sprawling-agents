// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::error::{AxCode, AxError};

pub const RESERVED_PREFIX: &str = ".sprawling";

pub const GIT_METADATA: &str = ".git";

pub const PROTECTED_METADATA: [&str; 2] = [RESERVED_PREFIX, GIT_METADATA];

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct Address(String);

impl Address {
    pub fn parse(raw: &str) -> Result<Self, AxError> {
        let reject = |violation: &str| {
            Err(
                AxError::failure(AxCode::InvalidArgs, "parse address", raw).with_recovery(format!(
                    "{violation}; use a canonical relative path: `/`-separated \
                     segments without `.`/`..`, backslash, `:`, or control characters"
                )),
            )
        };
        if raw.is_empty() {
            return reject("address is empty");
        }
        if raw.starts_with('/') {
            return reject("address is absolute");
        }
        if raw.contains('\\') {
            return reject("address contains a backslash");
        }
        if raw.contains(':') {
            return reject("address contains `:`");
        }
        if raw.chars().any(char::is_control) {
            return reject("address contains a control character");
        }
        for segment in raw.split('/') {
            if segment.is_empty() {
                return reject("address contains an empty segment");
            }
            if segment == "." || segment == ".." {
                return reject("address contains a `.` or `..` segment");
            }

            if segment.ends_with('.') || segment.ends_with(char::is_whitespace) {
                return reject(
                    "a segment ends with a dot or whitespace, which some file systems drop, \
                     making this address a second spelling of a different one",
                );
            }
        }
        Ok(Address(raw.to_owned()))
    }

    pub fn is_within(&self, prefix: &Address) -> bool {
        match self.0.strip_prefix(&prefix.0) {
            Some(rest) => rest.is_empty() || rest.starts_with('/'),
            None => false,
        }
    }

    pub fn is_reserved(&self) -> bool {
        self.0.split('/').any(|segment| {
            PROTECTED_METADATA
                .iter()
                .any(|name| segment.eq_ignore_ascii_case(name))
        })
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn name(&self) -> &str {
        self.0.rsplit('/').next().unwrap_or(&self.0)
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadVerdict {
    Open,

    Confidential,

    RulesUnreadable(AxError),
}

pub fn may_read(
    reader_building: &Address,
    target: &Address,
    rules: impl FnOnce() -> Result<bool, AxError>,
) -> ReadVerdict {
    if target.is_within(reader_building) {
        return ReadVerdict::Open;
    }
    match rules() {
        Ok(false) => ReadVerdict::Open,
        Ok(true) => ReadVerdict::Confidential,
        Err(unread) => ReadVerdict::RulesUnreadable(unread),
    }
}

impl<'de> Deserialize<'de> for Address {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Address::parse(&raw).map_err(serde::de::Error::custom)
    }
}

pub(crate) const SESSION_NAME_MAX: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct SessionName(String);

impl SessionName {
    pub fn parse(raw: &str) -> Result<Self, AxError> {
        let trimmed = raw.trim();
        let reject = |violation: &str| {
            Err(
                AxError::failure(AxCode::InvalidArgs, "name a session", raw).with_recovery(
                    format!(
                        "{violation}; give one word or phrase this session can be found by - it \
                         becomes a folder beside the others in that building"
                    ),
                ),
            )
        };
        if trimmed.is_empty() {
            return reject("a session with no name has no folder to work in");
        }
        if trimmed.chars().count() > SESSION_NAME_MAX {
            return reject("that is longer than a name and shorter than a task");
        }
        if trimmed == "." || trimmed == ".." {
            return reject("that names a directory rather than a session");
        }

        if matches!(Address::parse(trimmed), Ok(addr) if addr.is_reserved()) {
            return reject("that name belongs to the city itself");
        }
        if trimmed.contains('/') || Address::parse(trimmed).is_err() {
            return reject(
                "a session name is one segment that stands on its own: no `/`, `\\`, `:`, \
                 control characters, and no trailing dot",
            );
        }
        Ok(SessionName(trimmed.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for SessionName {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        SessionName::parse(&raw).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
pub(crate) mod tests;
