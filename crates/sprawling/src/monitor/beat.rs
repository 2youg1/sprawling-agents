// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The beat the monitor samples at: one atomic the sampling thread reads
//! on every wake, and the file under the city's reserved subtree that
//! keeps it from one serve to the next (`crates/sprawling/spec/Monitor.lean` §8-96, D44).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use kernel::{AxCode, AxError};
use serde::{Deserialize, Serialize};
use wire::BeatMs;

/// The file's one row.
#[derive(Debug, Serialize, Deserialize)]
struct Kept {
    beat_ms: u32,
}

/// The monitor's beat and where the city keeps it.
#[derive(Debug)]
pub(crate) struct Beat {
    ms: AtomicU32,
    file: PathBuf,
}

impl Beat {
    /// The beat this city kept, or the default when it kept none.
    ///
    /// A file that does not read, or holds a beat out of range, gives
    /// the default and says so on stderr: the beat shapes a curve a
    /// person reads and decides nothing, so a broken file must not stop
    /// a city from serving.
    pub(crate) fn open(city_root: &Path) -> Beat {
        let file = kernel::layout::CityLayout::new(city_root).monitor();
        let kept = match read(&file) {
            Ok(kept) => kept.unwrap_or(BeatMs::DEFAULT),
            Err(error) => {
                eprintln!("{error}");
                BeatMs::DEFAULT
            }
        };
        Beat {
            ms: AtomicU32::new(kept.ms()),
            file,
        }
    }

    /// The beat in force now.
    pub(crate) fn now(&self) -> BeatMs {
        BeatMs::new(self.ms.load(Ordering::Relaxed)).unwrap_or(BeatMs::DEFAULT)
    }

    /// One beat as a sleep.
    pub(crate) fn duration(&self) -> Duration {
        Duration::from_millis(u64::from(self.now().ms()))
    }

    /// Samples at `beat` from the next wake on, and keeps it for the
    /// city. A file that cannot be written is said on stderr; the new
    /// beat holds until the city stops either way.
    pub(crate) fn set(&self, beat: BeatMs) {
        self.ms.store(beat.ms(), Ordering::Relaxed);
        if self.file.as_os_str().is_empty() {
            drop(write(&self.file, beat));
        }
    }
}

/// The kept beat; `None` when the city never set one.
fn read(file: &Path) -> Result<Option<BeatMs>, AxError> {
    let text = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(failure(file, "read", &error)),
    };
    let kept: Kept = toml::from_str(&text).map_err(|error| failure(file, "read", &error))?;
    BeatMs::new(kept.beat_ms).map(Some)
}

/// The whole file, replaced through the city's one document writer.
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

    /// A beat a page sets holds at once and is the beat the next serve of
    /// the same city opens with; a city that kept none, or kept one out of
    /// range, samples at the default.
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
