// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Whether real-time scanning stands in front of the city directory's
//! writes (`crates/sprawling/Spec.lean` §8-166).
//!
//! A city writes many small files: the Ledger, the buildings' worktrees,
//! their builds. On Windows, Defender checks each write before it
//! lands, and two things move that check out of the way: a trusted Dev
//! Drive, which Defender scans asynchronously, and an exclusion that
//! holds the city. This module says which of them holds for the city's
//! directory, or that it cannot tell and why, and what the person can
//! do. What it says is advice; no tier's verdict reads it.
//!
//! **Each fact is read the way that needs no administrator first.** The
//! file system name comes from `sysinfo`, and every Dev Drive is ReFS,
//! so any other file system settles the question without starting a
//! process. Only a ReFS volume is put to `fsutil devdrv query`, which an
//! ordinary user is often refused, and which speaks the system's
//! language; both read as "cannot tell" rather than as a guess.
//! Defender's exclusions are read through PowerShell, and a Defender
//! that shows them only to an administrator says so in a line this
//! module reads.

use std::path::{Path, PathBuf};

use super::Platform;
use super::asking::{self, Ended};

/// How many knocks each question gets: `PATIENCE * asking::TICK` is
/// fifteen seconds, because a cold Windows PowerShell takes seconds to
/// start before it asks Defender anything.
const PATIENCE: u32 = 300;

/// The file system every Dev Drive is formatted with.
const DEV_DRIVE_FILE_SYSTEM: &str = "ReFS";

/// The tool that says whether a volume is a Dev Drive, as a person
/// reads its name in a sentence.
const DEV_DRIVE_QUERY: &str = "fsutil devdrv query";

/// The cmdlet that lists Defender's settings, as a person reads it.
const EXCLUSION_QUERY: &str = "Get-MpPreference";

/// Defender's excluded paths one per line, in UTF-8 whatever the
/// console's code page, and a non-zero exit when Defender cannot be
/// asked at all.
const LIST_EXCLUSIONS: &str = "[Console]::OutputEncoding = [Text.Encoding]::UTF8; \
     $ErrorActionPreference = 'Stop'; (Get-MpPreference).ExclusionPath";

/// The column the value of each line of the part starts at, as in the
/// `priority` part above it.
const LABEL: usize = 16;

/// What this machine says about scanning in front of one directory.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Scanning {
    /// Not Windows: Dev Drive and Defender's exclusions are Windows
    /// features.
    DoesNotApply,
    /// The thread that asked stopped before it answered.
    Stopped,
    Read {
        /// The directory judged, spelled the way mount points are.
        city: PathBuf,
        drive: Drive,
        exclusion: Exclusion,
    },
}

/// Whether the city's volume is a Dev Drive.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Drive {
    /// A trusted Dev Drive: Defender scans it in performance mode,
    /// after a write rather than in front of it.
    Trusted,
    /// A Dev Drive that is not trusted, which Defender scans like any
    /// other volume.
    Untrusted {
        volume: String,
    },
    /// Not a Dev Drive: not ReFS, or ReFS that fsutil says is no
    /// developer volume.
    Not {
        volume: String,
        file_system: String,
    },
    Untold(Untold),
}

/// Whether one of Defender's exclusions holds the city.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Exclusion {
    /// `under` is the excluded path that holds it.
    Inside {
        under: String,
    },
    Outside,
    Untold(Untold),
}

/// Why a question has no answer here.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Untold {
    /// No disk this machine lists has a mount point the path starts
    /// with.
    NoDisk,
    /// Defender shows its exclusions only to an administrator.
    AdminOnly,
    /// The tool exited cleanly in words this module does not read,
    /// such as the system's own language.
    Unread {
        command: &'static str,
    },
    Failed {
        command: &'static str,
        code: Option<i32>,
    },
    Unstarted {
        command: &'static str,
    },
    Unanswered {
        command: &'static str,
        stopping: Option<String>,
    },
}

/// Asks this machine about the directory `city`, which need not exist
/// yet. `ThisMachine::scanning` is this.
pub(crate) fn read(platform: Option<Platform>, city: &Path) -> Scanning {
    match platform {
        Some(Platform::Windows) => {}
        Some(Platform::MacOs | Platform::Linux) | None => return Scanning::DoesNotApply,
    }
    let city = crate::monitor::volume::resolved(city).unwrap_or_else(|| city.to_path_buf());
    let exclusions = asking::ask(
        child::command("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command"])
            .arg(LIST_EXCLUSIONS),
        PATIENCE,
        {
            let city = city.display().to_string();
            move |entry: &str| {
                let entry = entry.trim();
                entry.starts_with("N/A") || covers(entry, &city)
            }
        },
    );
    Scanning::Read {
        drive: drive(&city),
        exclusion: exclusion_said(&city, &exclusions),
        city,
    }
}

/// The Dev Drive answer for the volume a write under `city` lands on.
fn drive(city: &Path) -> Drive {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let Some(disk) = crate::monitor::volume::holding(&disks, city) else {
        return Drive::Untold(Untold::NoDisk);
    };
    let volume = volume_named(disk.mount_point());
    let file_system = disk.file_system().to_string_lossy().into_owned();
    if !file_system.eq_ignore_ascii_case(DEV_DRIVE_FILE_SYSTEM) {
        return Drive::Not {
            volume,
            file_system,
        };
    }
    let asked = asking::ask(
        child::command("fsutil").args(["devdrv", "query", volume.as_str()]),
        PATIENCE,
        |line: &str| line.trim().to_ascii_lowercase().starts_with("this is"),
    );
    drive_said(&volume, &file_system, &asked)
}

/// A mount point spelled the way fsutil takes a volume: `D:\` as `D:`,
/// a folder mount as its path.
fn volume_named(mount: &Path) -> String {
    let spelled = mount.display().to_string();
    match spelled.strip_suffix('\\') {
        Some(letter) if letter.ends_with(':') => letter.to_owned(),
        Some(_) | None => spelled,
    }
}

/// What `fsutil devdrv query <volume>` said about a ReFS `volume`.
///
/// The answer is the first line that opens with `This is`; the lines
/// after it list filters and say nothing about this volume's standing.
pub(super) fn drive_said(volume: &str, file_system: &str, ended: &Ended) -> Drive {
    let stdout = match printed(DEV_DRIVE_QUERY, ended) {
        Ok(stdout) => stdout,
        Err(why) => return Drive::Untold(why),
    };
    let statement = stdout
        .lines()
        .map(|line| line.trim().to_ascii_lowercase())
        .find(|line| line.starts_with("this is"));
    let Some(said) = statement.filter(|said| said.contains("developer volume")) else {
        return Drive::Untold(Untold::Unread {
            command: DEV_DRIVE_QUERY,
        });
    };
    let distrusted = ["untrusted", "not trusted", "not a trusted"]
        .iter()
        .any(|denial| said.contains(denial));
    if said.contains("not a developer volume") {
        Drive::Not {
            volume: volume.to_owned(),
            file_system: file_system.to_owned(),
        }
    } else if said.contains("trusted") && !distrusted {
        Drive::Trusted
    } else {
        Drive::Untrusted {
            volume: volume.to_owned(),
        }
    }
}

/// What `Get-MpPreference` said about the exclusions that hold `city`.
pub(super) fn exclusion_said(city: &Path, ended: &Ended) -> Exclusion {
    let stdout = match printed(EXCLUSION_QUERY, ended) {
        Ok(stdout) => stdout,
        Err(why) => return Exclusion::Untold(why),
    };
    let entries: Vec<&str> = stdout
        .lines()
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .collect();
    if entries.iter().any(|entry| entry.starts_with("N/A")) {
        return Exclusion::Untold(Untold::AdminOnly);
    }
    let city = city.display().to_string();
    entries
        .into_iter()
        .find(|entry| covers(entry, &city))
        .map_or(Exclusion::Outside, |under| Exclusion::Inside {
            under: under.to_owned(),
        })
}

/// Whether the excluded path `entry` holds `city`: the same directory,
/// or one of its ancestors at a separator, compared without case as
/// Windows compares paths.
///
/// An entry with an environment variable or a wildcard holds nothing
/// here: expanding it is Defender's rule, and a second copy of that
/// rule would drift from the first without anyone noticing.
fn covers(entry: &str, city: &str) -> bool {
    if entry.contains(['%', '*', '?']) {
        return false;
    }
    let entry = entry.trim_end_matches(['\\', '/']).to_lowercase();
    let city = city.trim_end_matches(['\\', '/']).to_lowercase();
    city == entry
        || city
            .strip_prefix(entry.as_str())
            .is_some_and(|rest| rest.starts_with(['\\', '/']))
}

/// What a tool that exited cleanly printed, or why there is nothing
/// of it to read.
fn printed<'e>(command: &'static str, ended: &'e Ended) -> Result<&'e str, Untold> {
    match ended {
        Ended::Exited {
            code: Some(0),
            kept,
        } => Ok(kept),
        Ended::Exited { code, .. } => Err(Untold::Failed {
            command,
            code: *code,
        }),
        Ended::Unstarted => Err(Untold::Unstarted { command }),
        Ended::Unanswered { stopping } => Err(Untold::Unanswered {
            command,
            stopping: stopping.clone(),
        }),
    }
}

/// The part of the terminal report that says it, closed by a blank
/// line.
pub(crate) fn lines(scanning: &Scanning) -> Vec<String> {
    let mut lines = vec![
        "  scanning - whether antivirus scanning holds up the city's writes".to_owned(),
        String::new(),
    ];
    match scanning {
        Scanning::DoesNotApply => lines.push(line(
            "scanning",
            "does not apply: Dev Drive and Defender's exclusions are Windows features",
        )),
        Scanning::Stopped => lines.push(line(
            "scanning",
            "cannot tell: its probe stopped before it answered",
        )),
        Scanning::Read {
            city,
            drive,
            exclusion,
        } => {
            lines.push(line("city", &city.display().to_string()));
            lines.push(line("dev drive", &drive_words(drive)));
            lines.push(line("exclusion", &exclusion_words(exclusion)));
            lines.extend(advice(city, drive, exclusion));
        }
    }
    lines.push(String::new());
    lines
}

fn line(label: &str, value: &str) -> String {
    format!("    {label:<LABEL$}{value}")
}

fn drive_words(drive: &Drive) -> String {
    match drive {
        Drive::Trusted => {
            "yes, trusted: Defender scans it after a write, not in front of it".to_owned()
        }
        Drive::Untrusted { volume } => {
            format!("yes, but {volume} is not trusted, so Defender scans in front of every write")
        }
        Drive::Not {
            volume,
            file_system,
        } => format!("no: {volume} is {file_system}, not a Dev Drive"),
        Drive::Untold(why) => format!("cannot tell: {}", untold_words(why)),
    }
}

fn exclusion_words(exclusion: &Exclusion) -> String {
    match exclusion {
        Exclusion::Inside { under } => format!("yes, under {under}"),
        Exclusion::Outside => "no".to_owned(),
        Exclusion::Untold(why) => format!("cannot tell: {}", untold_words(why)),
    }
}

fn untold_words(why: &Untold) -> String {
    let waited = asking::TICK.saturating_mul(PATIENCE).as_secs();
    match why {
        Untold::NoDisk => "no disk this machine lists holds that path".to_owned(),
        Untold::AdminOnly => {
            "Windows shows Defender's exclusions only to an administrator".to_owned()
        }
        Untold::Unread { command } => {
            format!("{command} answered in words this doctor does not read")
        }
        Untold::Failed { command, code } => match code {
            Some(code) => format!("{command} ended with exit code {code}"),
            None => format!("{command} was ended before it exited"),
        },
        Untold::Unstarted { command } => format!("{command} would not start"),
        Untold::Unanswered { command, stopping } => format!(
            "{command} did not answer within {waited} s, and {}",
            stopping.as_deref().unwrap_or("it was stopped")
        ),
    }
}

/// What the person can do, when neither a trusted Dev Drive nor an
/// exclusion moves scanning out of the way: nothing when one does.
fn advice(city: &Path, drive: &Drive, exclusion: &Exclusion) -> Vec<String> {
    match (drive, exclusion) {
        (Drive::Trusted, _) | (_, Exclusion::Inside { .. }) => Vec::new(),
        (Drive::Untrusted { volume }, Exclusion::Outside | Exclusion::Untold(_)) => vec![line(
            "to speed it up",
            &format!(
                "trust the Dev Drive in an administrator terminal: fsutil devdrv trust {volume}"
            ),
        )],
        (Drive::Not { .. } | Drive::Untold(_), Exclusion::Outside | Exclusion::Untold(_)) => {
            let quoted = city.display().to_string().replace('\'', "''");
            vec![
                line(
                    "to speed it up",
                    "move the city onto a Dev Drive (Settings > System > Storage > Advanced \
                     storage settings > Disks & volumes > Create dev drive),",
                ),
                line(
                    "",
                    &format!(
                        "or exclude it in an administrator PowerShell: \
                         Add-MpPreference -ExclusionPath '{quoted}'"
                    ),
                ),
            ]
        }
    }
}

#[cfg(test)]
#[path = "scanning/tests.rs"]
mod tests;
