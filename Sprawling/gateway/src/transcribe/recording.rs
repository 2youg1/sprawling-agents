// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::Read as _;

use kernel::{AxCode, AxError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioType {
    Webm,
    Ogg,
    Mpeg,
    Mp4,
    Wav,
}

impl AudioType {
    pub const ALL: [AudioType; 5] = [
        AudioType::Webm,
        AudioType::Ogg,
        AudioType::Mpeg,
        AudioType::Mp4,
        AudioType::Wav,
    ];

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

    fn leads(self, head: &[u8]) -> bool {
        match self {
            AudioType::Wav => {
                head.starts_with(b"RIFF") && head.get(8..12) == Some(b"WAVE".as_slice())
            }
            AudioType::Ogg => head.starts_with(b"OggS"),

            AudioType::Webm => head.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]),
            AudioType::Mp4 => head.get(4..8) == Some(b"ftyp".as_slice()),

            AudioType::Mpeg => {
                head.starts_with(b"ID3")
                    || matches!(head, [0xFF, second, ..] if second & 0xE0 == 0xE0)
            }
        }
    }

    fn extension(self) -> &'static str {
        self.file_name()
            .rsplit_once('.')
            .map_or("", |(_, extension)| extension)
    }

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

fn listed(spell: impl Fn(AudioType) -> String) -> String {
    AudioType::ALL
        .into_iter()
        .map(spell)
        .collect::<Vec<String>>()
        .join(", ")
}

pub(crate) const RECORDING_MAX_BYTES: usize = 25 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct Recording {
    bytes: Vec<u8>,
    kind: AudioType,
}

impl Recording {
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

    pub fn read_from(reader: impl std::io::Read, kind: AudioType) -> Result<Recording, AxError> {
        Recording::new(read_to_the_ceiling(reader)?, kind)
    }

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

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

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
