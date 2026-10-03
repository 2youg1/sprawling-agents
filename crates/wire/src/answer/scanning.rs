// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Whether real-time scanning stands in front of the city directory's
//! writes, as the doctor page reads it (`crates/wire/spec/Answer/Doctor.lean`
//! D25).
//!
//! Arm for arm the shape of `bin::doctor::scanning::Scanning`, because
//! the page says exactly what the terminal says; paths and commands
//! travel as rendered strings, since `PathBuf` and `&'static str` are
//! no wire types.

use serde::{Deserialize, Serialize};

/// What this machine says about scanning in front of the city's
/// directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorScanning {
    /// macOS and Linux: Dev Drive and Defender's exclusions are Windows
    /// features, and neither platform scans every write synchronously.
    DoesNotApply,
    /// The thread that asked stopped before it answered.
    Stopped,
    Read {
        /// The directory judged, spelled the way mount points are.
        city: String,
        drive: DoctorDrive,
        exclusion: DoctorExclusion,
    },
}

/// Whether the city's volume is a Dev Drive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorDrive {
    /// A trusted Dev Drive: scanned after a write rather than before.
    Trusted,
    /// A Dev Drive that is not trusted, scanned like any other volume.
    Untrusted {
        volume: String,
    },
    /// No Dev Drive at all.
    Not {
        volume: String,
        file_system: String,
    },
    Untold {
        why: DoctorUntold,
    },
}

/// Whether one of Defender's exclusions holds the city.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorExclusion {
    /// `under` is the excluded path that holds it.
    Inside {
        under: String,
    },
    Outside,
    Untold {
        why: DoctorUntold,
    },
}

/// Why a question about scanning has no answer here. `command` is the
/// tool as a person reads its name, such as `fsutil devdrv query`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorUntold {
    /// No disk this machine lists holds the path.
    NoDisk,
    /// Defender shows its exclusions only to an administrator.
    AdminOnly,
    /// The tool answered in words that are not read here, such as the
    /// system's own language.
    Unread {
        command: String,
    },
    Failed {
        command: String,
        code: Option<i32>,
    },
    Unstarted {
        command: String,
    },
    Unanswered {
        command: String,
        stopping: Option<String>,
    },
}
