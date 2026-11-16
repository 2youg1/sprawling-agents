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

use std::io::Read as _;

use kernel::{AxCode, AxError};

/// The audio containers the OpenAI audio wire accepts and a browser
/// records into. Fail closed: a container not spelled here has no
/// media type to send, so it cannot reach the wire at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioType {
    Webm,
    Ogg,
    Mpeg,
    Mp4,
    Wav,
}

impl AudioType {
    /// Every container this city can send, in the order a refusal lists
    /// them.
    pub const ALL: [AudioType; 5] = [
        AudioType::Webm,
        AudioType::Ogg,
        AudioType::Mpeg,
        AudioType::Mp4,
        AudioType::Wav,
    ];

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

    /// The container a media type names, or a refusal saying which
    /// containers exist.
    ///
    /// Fail closed, and the refusal lists them: a browser that recorded
    /// into something else has to be told what to record into instead,
    /// and a person cannot act on "unsupported".
    ///
    /// # Errors
    /// `E_INVALID_ARGS` for a media type this build cannot send.
    pub fn of_media_type(media: &str) -> Result<AudioType, AxError> {
        match media.trim().to_ascii_lowercase().as_str() {
            "audio/webm" | "video/webm" => Ok(AudioType::Webm),
            "audio/ogg" => Ok(AudioType::Ogg),
            "audio/mpeg" | "audio/mp3" => Ok(AudioType::Mpeg),
            "audio/mp4" => Ok(AudioType::Mp4),
            "audio/wav" | "audio/x-wav" | "audio/wave" => Ok(AudioType::Wav),
            other => Err(AxError::failure(
                AxCode::InvalidArgs,
                "read a recording's container",
                other.to_owned(),
            )
            .with_recovery(format!(
                "record into one of {}",
                listed(|kind| kind.media_type().to_owned())
            ))),
        }
    }

    /// The container a recording's file name declares by its extension
    /// (`crates/gateway/spec/Transcribe/Recording.lean` §8-33).
    ///
    /// # Errors
    /// `E_INVALID_ARGS` for a name whose extension is none of the
    /// containers this city can send.
    pub fn of_file_name(name: &str) -> Result<AudioType, AxError> {
        let declared = std::path::Path::new(name)
            .extension()
            .and_then(std::ffi::OsStr::to_str);
        AudioType::ALL
            .into_iter()
            .find(|kind| declared.is_some_and(|asked| kind.extension().eq_ignore_ascii_case(asked)))
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "read a recording's container",
                    name.to_owned(),
                )
                .with_recovery(format!(
                    "name a recording whose file ends in one of {}",
                    listed(|kind| format!(".{}", kind.extension()))
                ))
            })
    }

    /// The container a recording's leading bytes show (`crates/gateway/spec/Ocr.lean`
    /// §8-34): a recording with no name, such as a block a
    /// connector stored, says what it is only by how it starts.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` for bytes that start like none of the containers
    /// this city can send; the refusal lists them.
    pub fn of_signature(head: &[u8]) -> Result<AudioType, AxError> {
        AudioType::ALL
            .into_iter()
            .find(|kind| kind.leads(head))
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "read a recording's container",
                    "its leading bytes name none of the containers this city can send".to_owned(),
                )
                .with_recovery(format!(
                    "pass a recording in one of {}",
                    listed(|kind| kind.media_type().to_owned())
                ))
            })
    }

    /// Whether `head` starts the way this container's files start. Each
    /// container's signature is written in this one exhaustive match, so
    /// a sixth container cannot be added without one.
    fn leads(self, head: &[u8]) -> bool {
        match self {
            AudioType::Wav => {
                head.starts_with(b"RIFF") && head.get(8..12) == Some(b"WAVE".as_slice())
            }
            AudioType::Ogg => head.starts_with(b"OggS"),
            // The EBML header a browser's WebM recording opens with.
            AudioType::Webm => head.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]),
            AudioType::Mp4 => head.get(4..8) == Some(b"ftyp".as_slice()),
            // An ID3 tag, or an MPEG audio frame's sync: eleven set bits.
            AudioType::Mpeg => {
                head.starts_with(b"ID3")
                    || matches!(head, [0xFF, second, ..] if second & 0xE0 == 0xE0)
            }
        }
    }

    /// The extension [`AudioType::file_name`] ends in, so the name a
    /// request body sends and the name a file is read by are one fact.
    fn extension(self) -> &'static str {
        self.file_name()
            .rsplit_once('.')
            .map_or("", |(_, extension)| extension)
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

/// Every container, each spelled the way `spell` spells it, for a
/// refusal to list: the list is [`AudioType::ALL`], so a sixth container
/// is named in every refusal the moment it exists.
fn listed(spell: impl Fn(AudioType) -> String) -> String {
    AudioType::ALL
        .into_iter()
        .map(spell)
        .collect::<Vec<String>>()
        .join(", ")
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

    /// A recording read from `reader` (`crates/gateway/spec/Transcribe/Recording.lean` §8-33).
    ///
    /// # Errors
    /// `E_STORAGE_FATAL` when the reader fails, and whatever
    /// [`Recording::new`] refuses.
    ///
    /// The reader is read no further than one byte past
    /// `RECORDING_MAX_BYTES`, which is enough for [`Recording::new`] to
    /// refuse a recording over the ceiling: a file of several gigabytes is
    /// refused without being held in memory, and the ceiling keeps one
    /// owner rather than a copy in every caller.
    pub fn read_from(reader: impl std::io::Read, kind: AudioType) -> Result<Recording, AxError> {
        Recording::new(read_to_the_ceiling(reader)?, kind)
    }

    /// A recording with no name to read its container from, read from
    /// `reader` as [`Recording::read_from`] reads one, its container
    /// taken from the bytes it starts with (`crates/gateway/spec/Ocr.lean`
    /// §8-34).
    ///
    /// # Errors
    /// `E_STORAGE_FATAL` when the reader fails, what
    /// [`AudioType::of_signature`] refuses, and whatever
    /// [`Recording::new`] refuses.
    pub fn read_unlabelled(reader: impl std::io::Read) -> Result<Recording, AxError> {
        let bytes = read_to_the_ceiling(reader)?;
        let kind = AudioType::of_signature(&bytes)?;
        Recording::new(bytes, kind)
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

/// What `reader` holds, read no further than one byte past
/// `RECORDING_MAX_BYTES`: enough for [`Recording::new`] to refuse a
/// recording over the ceiling without holding all of it.
///
/// # Errors
/// `E_STORAGE_FATAL` when the reader fails.
fn read_to_the_ceiling(reader: impl std::io::Read) -> Result<Vec<u8>, AxError> {
    let unreadable = |why: String| {
        AxError::failure(AxCode::StorageFatal, "take a recording", why)
            .with_recovery("name a recording this machine can read, or record it again")
    };
    let past_the_ceiling = u64::try_from(RECORDING_MAX_BYTES)
        .map_err(|err| unreadable(err.to_string()))?
        .saturating_add(1);
    let mut bytes = Vec::new();
    reader
        .take(past_the_ceiling)
        .read_to_end(&mut bytes)
        .map_err(|err| unreadable(err.to_string()))?;
    Ok(bytes)
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

    /// What a browser declares is what this city reads back, and a
    /// container it cannot send is refused with the list of the ones it
    /// can. "Unsupported" is not something a person can act on.
    #[test]
    fn a_container_this_city_cannot_send_is_refused_with_the_ones_it_can() {
        for kind in [
            AudioType::Webm,
            AudioType::Ogg,
            AudioType::Mpeg,
            AudioType::Mp4,
            AudioType::Wav,
        ] {
            assert_eq!(
                AudioType::of_media_type(kind.media_type()).unwrap(),
                kind,
                "{kind:?} is read back from the type it declares"
            );
        }
        // What a browser actually sends: the container, then the codec
        // inside it. The parameter belongs to the caller to strip, so
        // this door sees the container alone.
        assert_eq!(
            AudioType::of_media_type("AUDIO/WEBM").unwrap(),
            AudioType::Webm,
            "a media type is not case-sensitive"
        );
        let refused = AudioType::of_media_type("audio/flac").unwrap_err();
        assert_eq!(*refused.code(), AxCode::InvalidArgs);
        assert!(
            refused.recovery().contains("audio/webm"),
            "the refusal names what to record into instead: {}",
            refused.recovery()
        );
    }

    /// A file carries no content-type, so its name is the only thing
    /// that says what container it is, read against the same enum the
    /// request body is written from.
    #[test]
    fn a_recording_file_is_read_as_the_container_its_name_declares() {
        for kind in AudioType::ALL {
            let name = format!("hall/dropped/0/{}", kind.file_name());
            assert_eq!(AudioType::of_file_name(&name).ok(), Some(kind), "{name}");
        }
        assert_eq!(
            AudioType::of_file_name("VOICE.WAV").ok(),
            Some(AudioType::Wav),
            "an extension is not case-sensitive"
        );
        for unknown in ["voice.flac", "voice", "voice."] {
            let refused = AudioType::of_file_name(unknown).err();
            assert!(
                refused
                    .as_ref()
                    .is_some_and(|err| *err.code() == AxCode::InvalidArgs
                        && err.recovery().contains(".wav")
                        && err.recovery().contains(".mp3")),
                "{unknown} is refused with the extensions this city can send: {refused:?}"
            );
        }
    }

    /// A recording with no name is read as the container its first bytes
    /// show, and bytes that start like none of them are refused with the
    /// list of the ones this city can send.
    #[test]
    fn a_recording_with_no_name_is_read_as_the_container_it_starts_with() {
        let starts: [(&[u8], AudioType); 6] = [
            (b"RIFF\x24\x00\x00\x00WAVEfmt ", AudioType::Wav),
            (b"OggS\x00\x02", AudioType::Ogg),
            (&[0x1A, 0x45, 0xDF, 0xA3, 0x9F], AudioType::Webm),
            (b"\x00\x00\x00\x20ftypM4A ", AudioType::Mp4),
            (b"ID3\x04\x00", AudioType::Mpeg),
            (&[0xFF, 0xFB, 0x90, 0x00], AudioType::Mpeg),
        ];
        let read: Vec<Option<AudioType>> = starts
            .iter()
            .map(|(head, _)| {
                Recording::read_unlabelled(*head)
                    .ok()
                    .map(|taken| taken.kind())
            })
            .collect();
        let owed: Vec<Option<AudioType>> = starts.iter().map(|(_, kind)| Some(*kind)).collect();
        assert_eq!(read, owed);
        let refused = AudioType::of_signature(b"fLaC\x00").err();
        assert!(
            refused
                .as_ref()
                .is_some_and(|err| *err.code() == AxCode::InvalidArgs
                    && err.recovery().contains("audio/webm")
                    && err.recovery().contains("audio/wav")),
            "{refused:?}"
        );
    }

    /// Zeros for as long as anyone reads, counting how many were given.
    struct Endless {
        given: usize,
    }

    impl std::io::Read for Endless {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            buf.fill(0);
            self.given += buf.len();
            Ok(buf.len())
        }
    }

    /// A file far larger than a provider takes is refused by the one
    /// ceiling, without being read into memory whole.
    #[test]
    fn a_recording_is_read_no_further_than_one_byte_past_the_ceiling() {
        let small = Recording::read_from(&b"RIFFfake"[..], AudioType::Wav);
        assert!(
            small
                .as_ref()
                .is_ok_and(|taken| taken.len() == 8 && taken.kind() == AudioType::Wav),
            "{small:?}"
        );
        let mut endless = Endless { given: 0 };
        let refused = Recording::read_from(&mut endless, AudioType::Wav).err();
        assert!(
            refused
                .as_ref()
                .is_some_and(|err| *err.code() == AxCode::InvalidArgs
                    && err.recovery().contains(&RECORDING_MAX_BYTES.to_string())),
            "{refused:?}"
        );
        assert!(
            endless.given <= RECORDING_MAX_BYTES + 1,
            "{} bytes were read to refuse a recording over {RECORDING_MAX_BYTES}",
            endless.given
        );
    }
}
