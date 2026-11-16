// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::super::{Absence, Fault, Platform, Presence, Version};

pub(super) fn font_at(
    platform: Platform,
    home: Option<PathBuf>,
    local_app_data: Option<OsString>,
    files: &[&str],
) -> Presence {
    let system = match platform {
        Platform::Windows => windows_font_root(),
        Platform::MacOs => Ok(vec![PathBuf::from("/Library/Fonts")]),
        Platform::Linux => Ok(vec![
            PathBuf::from("/usr/local/share/fonts"),
            PathBuf::from("/usr/share/fonts"),
        ]),
    };
    font_in(
        platform,
        FontLocations {
            home,
            local_app_data,
            system,
        },
        files,
    )
}

struct FontLocations {
    home: Option<PathBuf>,
    local_app_data: Option<OsString>,
    system: Result<Vec<PathBuf>, UnreadableRoot>,
}

struct UnreadableRoot {
    at: PathBuf,
    said: String,
}

#[cfg(windows)]
fn windows_font_root() -> Result<Vec<PathBuf>, UnreadableRoot> {
    crate::privacy::windows::system_root()
        .map(|root| vec![root.join("Fonts")])
        .map_err(|err| UnreadableRoot {
            at: crate::privacy::windows::system_root_location(),
            said: err.to_string(),
        })
}

#[cfg(not(windows))]
fn windows_font_root() -> Result<Vec<PathBuf>, UnreadableRoot> {
    Ok(Vec::new())
}

fn font_in(platform: Platform, locations: FontLocations, files: &[&str]) -> Presence {
    let mut roots = Vec::new();
    if let Platform::Windows = platform
        && let Some(local) = locations.local_app_data
    {
        roots.push(
            PathBuf::from(local)
                .join("Microsoft")
                .join("Windows")
                .join("Fonts"),
        );
    }
    if let Some(home) = locations.home.as_ref() {
        match platform {
            Platform::Windows => roots.push(
                home.join("scoop")
                    .join("apps")
                    .join("maple-mono-nf-cn")
                    .join("current"),
            ),
            Platform::MacOs => roots.push(home.join("Library").join("Fonts")),
            Platform::Linux => {
                roots.push(home.join(".local").join("share").join("fonts"));
                roots.push(home.join(".fonts"));
            }
        }
    }
    let depth = if matches!(platform, Platform::Linux) {
        4
    } else {
        0
    };

    let mut trouble: Option<Presence> = None;
    for root in roots {
        for file in files {
            match font_under(&root, file, depth) {
                Some(answer @ Presence::Present { .. }) => return answer,
                Some(answer @ Presence::Broken { .. }) => {
                    trouble.get_or_insert(answer);
                }
                _ => {}
            }
        }
    }
    match locations.system {
        Ok(roots) => {
            for root in roots {
                for file in files {
                    match font_under(&root, file, depth) {
                        Some(answer @ Presence::Present { .. }) => return answer,
                        Some(answer @ Presence::Broken { .. }) => {
                            trouble.get_or_insert(answer);
                        }
                        _ => {}
                    }
                }
            }
        }
        Err(UnreadableRoot { at, said }) => {
            trouble.get_or_insert(Presence::Broken {
                at,
                fault: Fault::Unreadable(said),
            });
        }
    }
    if let Some(answer) = trouble {
        return answer;
    }
    if locations.home.is_none() {
        Presence::Absent(Absence::NoHome)
    } else {
        Presence::Absent(Absence::NotInstalled)
    }
}

fn font_under(root: &Path, file: &str, depth: usize) -> Option<Presence> {
    let candidate = root.join(file);
    match std::fs::metadata(&candidate) {
        Ok(meta) if meta.is_file() => {
            return Some(Presence::Present {
                at: candidate,
                version: Version::Silent,
            });
        }
        Ok(_) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => {
            return Some(Presence::Broken {
                at: candidate,
                fault: Fault::Unreadable(err.to_string()),
            });
        }
    }
    if depth == 0 {
        return None;
    }
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return None,
        Err(err) => {
            return Some(Presence::Broken {
                at: root.to_path_buf(),
                fault: Fault::Unreadable(err.to_string()),
            });
        }
    };
    let mut directories = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                return Some(Presence::Broken {
                    at: root.to_path_buf(),
                    fault: Fault::Unreadable(err.to_string()),
                });
            }
        };
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => directories.push(entry.path()),
            Ok(_) => {}
            Err(err) => {
                return Some(Presence::Broken {
                    at: entry.path(),
                    fault: Fault::Unreadable(err.to_string()),
                });
            }
        }
    }
    directories.sort();
    for directory in directories {
        if let Some(answer) = font_under(&directory, file, depth.saturating_sub(1)) {
            return Some(answer);
        }
    }
    None
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn font_files_are_found_in_user_font_directories_and_absence_is_explicit() {
        const FILE: &str = "MapleMono-NF-CN-Regular.ttf";
        let linux_home = tempfile::tempdir().unwrap();
        let linux = linux_home
            .path()
            .join(".local")
            .join("share")
            .join("fonts")
            .join(FILE);
        std::fs::create_dir_all(linux.parent().unwrap()).unwrap();
        std::fs::write(&linux, b"font").unwrap();
        assert_eq!(
            font_at(
                Platform::Linux,
                Some(linux_home.path().to_path_buf()),
                None,
                &[FILE]
            ),
            Presence::Present {
                at: linux,
                version: Version::Silent,
            }
        );

        let windows_home = tempfile::tempdir().unwrap();
        let windows = windows_home
            .path()
            .join("scoop")
            .join("apps")
            .join("maple-mono-nf-cn")
            .join("current")
            .join(FILE);
        std::fs::create_dir_all(windows.parent().unwrap()).unwrap();
        std::fs::write(&windows, b"font").unwrap();
        assert_eq!(
            font_at(
                Platform::Windows,
                Some(windows_home.path().to_path_buf()),
                None,
                &[FILE]
            ),
            Presence::Present {
                at: windows,
                version: Version::Silent,
            }
        );
        let mac_system = tempfile::tempdir().unwrap();
        assert_eq!(
            font_in(
                Platform::MacOs,
                FontLocations {
                    home: Some(windows_home.path().to_path_buf()),
                    local_app_data: None,
                    system: Ok(vec![mac_system.path().to_path_buf()]),
                },
                &[FILE],
            ),
            Presence::Absent(Absence::NotInstalled)
        );
    }

    #[test]
    fn windows_user_fonts_precede_scoop_and_system_fonts() {
        let dir = tempfile::tempdir().unwrap();
        let local = dir.path().join("local");
        let user = local.join("Microsoft/Windows/Fonts/font.ttf");
        let scoop = dir
            .path()
            .join("scoop/apps/maple-mono-nf-cn/current/font.ttf");
        let system = dir.path().join("system/Fonts/font.ttf");
        for path in [&user, &scoop, &system] {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, b"font").unwrap();
        }
        let locations = || FontLocations {
            home: Some(dir.path().to_path_buf()),
            local_app_data: Some(local.clone().into_os_string()),
            system: Ok(vec![system.parent().unwrap().to_path_buf()]),
        };
        for path in [&user, &scoop, &system] {
            assert_eq!(
                font_in(Platform::Windows, locations(), &["font.ttf"]),
                Presence::Present {
                    at: path.to_path_buf(),
                    version: Version::Silent
                }
            );
            std::fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn linux_finds_nested_fonts_with_a_depth_limit() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join(".local/share/fonts");
        let nested = root.join("one/two/three/four/font.ttf");
        std::fs::create_dir_all(nested.parent().unwrap()).unwrap();
        std::fs::write(&nested, b"font").unwrap();
        let locations = || FontLocations {
            home: Some(dir.path().to_path_buf()),
            local_app_data: None,
            system: Ok(vec![]),
        };
        assert_eq!(
            font_in(Platform::Linux, locations(), &["font.ttf"]),
            Presence::Present {
                at: nested.clone(),
                version: Version::Silent
            }
        );
        let top = root.join("font.ttf");
        std::fs::write(&top, b"font").unwrap();
        assert_eq!(
            font_in(Platform::Linux, locations(), &["font.ttf"]),
            Presence::Present {
                at: top.clone(),
                version: Version::Silent
            }
        );
        std::fs::remove_file(top).unwrap();
        std::fs::remove_file(nested).unwrap();
        let too_deep = root.join("one/two/three/four/five/font.ttf");
        std::fs::create_dir_all(too_deep.parent().unwrap()).unwrap();
        std::fs::write(too_deep, b"font").unwrap();
        assert_eq!(
            font_in(Platform::Linux, locations(), &["font.ttf"]),
            Presence::Absent(Absence::NotInstalled)
        );
    }

    #[test]
    fn a_broken_first_candidate_does_not_hide_a_present_second_one() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join(".local/share/fonts");
        std::fs::create_dir_all(&root).unwrap();
        let broken = root.join("font.ttf\0");
        let second = root.join("font.ttc");
        std::fs::write(&second, b"font").unwrap();
        let locations = || FontLocations {
            home: Some(dir.path().to_path_buf()),
            local_app_data: None,
            system: Ok(vec![]),
        };
        let files = ["font.ttf\0", "font.ttc"];
        assert_eq!(
            font_in(Platform::Linux, locations(), &files),
            Presence::Present {
                at: second.clone(),
                version: Version::Silent
            }
        );
        std::fs::remove_file(&second).unwrap();
        assert!(matches!(
            font_in(Platform::Linux, locations(), &files),
            Presence::Broken { at, fault: Fault::Unreadable(_) } if at == broken
        ));
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_font_metadata_is_broken() {
        let dir = tempfile::tempdir().unwrap();
        let candidate = dir.path().join("font.ttf");
        std::os::unix::fs::symlink("font.ttf", &candidate).unwrap();
        assert!(matches!(font_under(dir.path(), "font.ttf", 0),
    Some(Presence::Broken { at, fault: Fault::Unreadable(_) }) if at == candidate));
    }

    #[test]
    fn unavailable_windows_system_root_is_reported_after_user_locations() {
        let dir = tempfile::tempdir().unwrap();
        let locations = FontLocations {
            home: Some(dir.path().to_path_buf()),
            local_app_data: None,
            system: Err(UnreadableRoot {
                at: PathBuf::from("HKLM"),
                said: "unreadable registry".to_owned(),
            }),
        };
        assert!(matches!(
            font_in(Platform::Windows, locations, &["font.ttf"]),
            Presence::Broken {
                fault: Fault::Unreadable(_),
                ..
            }
        ));
    }
}
