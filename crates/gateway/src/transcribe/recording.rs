// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The bytes a person spoke, and the format they are in.
//!
//! The extension a provider routes on and the media type it reads are
//! two faces of one fact, so they live in one enum: an audio format
//! that could name itself `.wav` while declaring `audio/webm` is a
//! shape this module cannot spell.

use kernel::{AxCode, AxError};

/// The audio containers the OpenAI audio wire accepts and a browser
/// records into. Fail closed: a container not spelled here has no
/// media type to send, so it cannot reach the wire at all.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioType {
    Webm,
    Ogg,
    Mpeg,
    Mp4,
    Wav,
}

impl AudioType {
    /// What the `Content-Type` of the file part says.
    #[must_use]
    pub fn media_type(self) -> &'static str {
        match self {
            AudioType::Webm => "audio/webm",
            AudioType::Ogg => "audio/ogg",
            AudioType::Mpeg => "audio/mpeg",
            AudioType::Mp4 => "audio/mp4",
            AudioType::Wav => "audio/wav",
        }
    }

    /// The `filename` of the file part. Providers route on the
    /// extension as well as on the media type, and one that disagreed
    /// with the other is answered 400.
    #[must_use]
    pub fn file_name(self) -> &'static str {
        match self {
            AudioType::Webm => "recording.webm",
            AudioType::Ogg => "recording.ogg",
            AudioType::Mpeg => "recording.mp3",
            AudioType::Mp4 => "recording.mp4",
            AudioType::Wav => "recording.wav",
        }
    }
}

/// The ceiling the OpenAI audio face prints for one upload. A
/// provider-side engineering parameter, not a city policy, so it stays
/// here rather than in `kernel::consts_policy`.
pub(crate) const RECORDING_MAX_BYTES: usize = 25 * 1024 * 1024;

/// One recording, ready to send.
///
/// Both invariants — non-empty, and inside the provider's ceiling —
/// are held at the single construction point, so a `Recording` that
/// exists is a `Recording` the wire will take.
#[derive(Debug, Clone)]
pub struct Recording {
    bytes: Vec<u8>,
    kind: AudioType,
}

impl Recording {
    /// # Errors
    /// `E_INVALID_ARGS` when the recording is empty, or larger than the
    /// provider's own ceiling; both refusals name the number.
    pub fn new(bytes: Vec<u8>, kind: AudioType) -> Result<Recording, AxError> {
        if bytes.is_empty() {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "take a recording",
                "the recording carries no bytes",
            )
            .with_recovery("record something audible and send it again"));
        }
        if bytes.len() > RECORDING_MAX_BYTES {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "take a recording",
                format!("the recording is {} bytes", bytes.len()),
            )
            .with_recovery(format!(
                "one recording is at most {RECORDING_MAX_BYTES} bytes; \
                 record a shorter one"
            )));
        }
        Ok(Recording { bytes, kind })
    }

    #[must_use]
    pub fn kind(&self) -> AudioType {
        self.kind
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Always false: the empty recording is refused at construction.
    /// Present because `len` without it reads as an oversight.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_recording_is_refused_where_it_is_made() {
        let err = Recording::new(Vec::new(), AudioType::Webm).unwrap_err();
        assert_eq!(err.code(), &AxCode::InvalidArgs);
        assert!(!err.recovery().is_empty());
    }

    #[test]
    fn a_recording_over_the_ceiling_is_refused_with_the_ceiling_named() {
        let err = Recording::new(vec![0u8; RECORDING_MAX_BYTES + 1], AudioType::Wav).unwrap_err();
        assert_eq!(err.code(), &AxCode::InvalidArgs);
        assert!(
            err.recovery().contains(&RECORDING_MAX_BYTES.to_string()),
            "a refusal that will not say the limit cannot be acted on: {}",
            err.recovery()
        );
    }

    #[test]
    fn every_container_names_itself_the_same_way_twice() {
        for kind in [
            AudioType::Webm,
            AudioType::Ogg,
            AudioType::Mpeg,
            AudioType::Mp4,
            AudioType::Wav,
        ] {
            let extension = kind.file_name().rsplit('.').next().unwrap();
            let subtype = kind.media_type().rsplit('/').next().unwrap();
            assert!(
                extension == subtype || (extension == "mp3" && subtype == "mpeg"),
                "{kind:?} sends {} beside {}",
                kind.file_name(),
                kind.media_type()
            );
        }
    }
}
