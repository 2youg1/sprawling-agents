// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use kernel::{AxCode, AxError};
use serde::{Deserialize, Serialize};
use wire::BeatMs;

#[derive(Debug, Serialize, Deserialize)]
struct Kept {
    beat_ms: u32,
}

#[derive(Debug)]
pub(crate) struct Beat {
    ms: AtomicU32,
    file: PathBuf,
}

impl Beat {

    pub(crate) fn open(city_root: &Path) -> Beat {
        let file = kernel::layout::CityLayout::new(city_root).monitor();
        let kept = match read(&file) {
            Ok(kept) => kept.unwrap_or(BeatMs::DEFAULT),
            Err(error) => {
                said(&error);
                BeatMs::DEFAULT
            }
        };
        Beat {
            ms: AtomicU32::new(kept.ms()),
            file,
        }
    }

    pub(crate) fn now(&self) -> BeatMs {
        BeatMs::new(self.ms.load(Ordering::Relaxed)).unwrap_or(BeatMs::DEFAULT)
    }

    pub(crate) fn duration(&self) -> Duration {
        Duration::from_millis(u64::from(self.now().ms()))
    }

    pub(crate) fn set(&self, beat: BeatMs) {
        self.ms.store(beat.ms(), Ordering::Relaxed);
        if let Err(error) = write(&self.file, beat) {
            said(&error);
        }
    }
}

fn said(error: &AxError) {
    crate::serving::journal::notice("bin::monitor", &format!("{error}; {}", error.recovery()));
}

fn read(file: &Path) -> Result<Option<BeatMs>, AxError> {
    let text = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(failure(file, "read", &error)),
    };
    let kept: Kept = toml::from_str(&text).map_err(|error| failure(file, "read", &error))?;
    BeatMs::new(kept.beat_ms).map(Some)
}

fn write(file: &Path, beat: BeatMs) -> Result<(), AxError> {
    let text = toml::to_string(&Kept { beat_ms: beat.ms() })
        .map_err(|error| failure(file, "write", &error))?;
    city::edit_document(file, |held| held.replace(text.as_bytes()))
}

fn failure(file: &Path, verb: &str, why: &impl std::fmt::Display) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        format!("{verb} the monitor's beat"),
        file.display().to_string(),
    )
    .with_recovery(format!(
        "{why}; the monitor samples at the default beat until the file is fixed or a beat is set again"
    ))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn a_beat_set_holds_and_the_next_serve_of_the_city_opens_with_it() {
        let city = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(city.path().join(kernel::RESERVED_PREFIX)).unwrap();
        let first = Beat::open(city.path());
        let opened = first.now();
        first.set(BeatMs::new(250).unwrap());
        let held = first.now();
        let next = Beat::open(city.path()).now();
        std::fs::write(
            kernel::layout::CityLayout::new(city.path()).monitor(),
            "beat_ms = 5
",
        )
        .unwrap();
        let out_of_range = Beat::open(city.path()).now();
        assert_eq!(
            (opened, held, next, out_of_range),
            (
                BeatMs::DEFAULT,
                BeatMs::new(250).unwrap(),
                BeatMs::new(250).unwrap(),
                BeatMs::DEFAULT
            )
        );
    }
}
