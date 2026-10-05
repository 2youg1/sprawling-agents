// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Explicit container limits and a local immutable image identity
//! (`crates/kernel/spec/Config.lean`, container input contract).

use std::num::{NonZeroU32, NonZeroU64};

use serde::{Deserialize, Serialize};

use crate::{AxCode, AxError};

/// A locally installed image ID. Registry references and mutable tags
/// cannot reach an execution request through this type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ContainerImage(String);

impl ContainerImage {
    /// Parses an immutable image ID without consulting a registry.
    ///
    /// # Errors
    /// `E_CONFIG_INVALID` for anything except a full lowercase sha256 ID.
    pub fn parse(image: &str) -> Result<Self, AxError> {
        if image.strip_prefix("sha256:").is_some_and(|digest| {
            digest.len() == 64
                && digest
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        }) {
            return Ok(Self(image.to_owned()));
        }
        Err(AxError::failure(
            AxCode::ConfigInvalid,
            "read the container image",
            "the image is not a full local sha256 image ID",
        )
        .with_recovery(
            "prepare an image yourself and use its sha256 ID from image inspect; exec never pulls an image",
        ))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ContainerImage {
    type Error = AxError;

    fn try_from(image: String) -> Result<Self, Self::Error> {
        Self::parse(&image)
    }
}

impl From<ContainerImage> for String {
    fn from(image: ContainerImage) -> Self {
        image.0
    }
}

/// Every limit is explicit and nonzero. CPU is in millicpu, memory is
/// in bytes, and the process count bounds descendants as well as PID 1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ContainerLimits {
    pub image: ContainerImage,
    pub user: NonZeroU32,
    pub cpu_millis: NonZeroU32,
    pub memory_bytes: NonZeroU64,
    pub pids: NonZeroU32,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn configuration_and_serde_share_the_immutable_image_admission() {
        for image in [
            "",
            "alpine:latest",
            "--privileged",
            "sha256:abc",
            "sha256:fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffF",
        ] {
            assert!(ContainerImage::parse(image).is_err());
            assert!(serde_json::from_value::<ContainerImage>(serde_json::json!(image)).is_err());
        }
        let image = format!("sha256:{}", "a".repeat(64));
        let parsed = ContainerImage::parse(&image).unwrap();
        assert_eq!(
            serde_json::from_value::<ContainerImage>(serde_json::json!(image)).unwrap(),
            parsed
        );
        assert_eq!(
            serde_json::to_value(parsed).unwrap(),
            serde_json::json!(image)
        );
    }
}
