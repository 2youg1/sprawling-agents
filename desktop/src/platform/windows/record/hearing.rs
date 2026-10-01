// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The sound of one recording.

use std::path::Path;

/// The arguments the sound's ffmpeg is started with.
pub(super) fn arguments(_device: &str, _into: &Path) -> Vec<String> {
    Vec::new()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    /// ffmpeg is told the device the scope file names, as DirectShow
    /// spells an audio input, and writes the 16 kHz mono wav the city's
    /// `transcribe` takes, into the recording's own directory.
    #[test]
    fn the_device_the_scope_file_names_is_the_device_ffmpeg_hears() {
        let line = arguments("Microphone (USB Audio)", Path::new("recorded"));
        let adjacent = |first: &str, second: &str| {
            line.windows(2)
                .any(|pair| pair[0] == first && pair[1] == second)
        };
        assert!(adjacent("-f", "dshow"), "{line:?}");
        assert!(adjacent("-i", "audio=Microphone (USB Audio)"), "{line:?}");
        assert!(adjacent("-ac", "1"), "{line:?}");
        assert!(adjacent("-ar", "16000"), "{line:?}");
        assert_eq!(
            line.last().map(String::as_str),
            Some(
                Path::new("recorded")
                    .join("sound.wav")
                    .display()
                    .to_string()
                    .as_str()
            ),
            "{line:?}"
        );
    }
}
