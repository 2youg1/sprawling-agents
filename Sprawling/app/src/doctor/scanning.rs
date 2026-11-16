// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use super::Platform;
use super::asking::{self, Ended};

const PATIENCE: u32 = 300;

const DEV_DRIVE_FILE_SYSTEM: &str = "ReFS";

const DEV_DRIVE_QUERY: &str = "fsutil devdrv query";

const EXCLUSION_QUERY: &str = "Get-MpPreference";

const LIST_EXCLUSIONS: &str = "[Console]::OutputEncoding = [Text.Encoding]::UTF8; \
     $ErrorActionPreference = 'Stop'; (Get-MpPreference).ExclusionPath";

const LABEL: usize = 16;

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Scanning {
    DoesNotApply,

    Stopped,
    Read {
        city: PathBuf,
        drive: Drive,
        exclusion: Exclusion,
    },
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Drive {
    Trusted,

    Untrusted { volume: String },

    Not { volume: String, file_system: String },
    Untold(Untold),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Exclusion {
    Inside { under: String },
    Outside,
    Untold(Untold),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Untold {
    NoDisk,

    AdminOnly,

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

fn volume_named(mount: &Path) -> String {
    let spelled = mount.display().to_string();
    match spelled.strip_suffix('\\') {
        Some(letter) if letter.ends_with(':') => letter.to_owned(),
        Some(_) | None => spelled,
    }
}

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
