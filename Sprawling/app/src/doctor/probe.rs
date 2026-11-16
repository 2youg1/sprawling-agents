// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::ffi::OsString;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

use accounting::Runnable;
use kernel::{AxCode, AxError};

use crate::serving::standing::{Standing, raise_this_thread};

use super::scanning::Scanning;
use super::{Absence, Detection, Fault, Platform, Presence, Requirement, Version};

pub(crate) trait Machine: accounting::Machine + Sync {

    fn look(&self, requirement: &Requirement) -> Presence;

    fn core_standing(&self) -> Result<Standing, AxError>;

    fn scanning(&self, city: &Path) -> Scanning;
}

pub(crate) struct ThisMachine {
    platform: Option<Platform>,
    patience: Duration,
}

impl ThisMachine {
    pub(crate) fn new(platform: Option<Platform>, patience: Duration) -> ThisMachine {
        ThisMachine { platform, patience }
    }
}

impl Machine for ThisMachine {
    fn look(&self, requirement: &Requirement) -> Presence {
        let search_path = super::host::search_path();
        match &requirement.detect {
            Detection::Program {
                program,
                version_arg,
                places,
            } => {
                let found = on_search_path(&search_path, program).or_else(|| {
                    self.platform
                        .and_then(|platform| at_a_known_place(places.at(platform)))
                });
                match found {
                    None => Presence::Absent(Absence::NotOnSearchPath),
                    Some(path) => ask_version(&path, version_arg, self.patience),
                }
            }

            Detection::Listed {
                program,
                args,
                line,
            } => match on_search_path(&search_path, program) {
                None => Presence::Absent(Absence::NotOnSearchPath),
                Some(path) => match ask(&path, args, Wanted::StartingWith(line), self.patience) {
                    listed @ Presence::Present {
                        version: Version::Said(_),
                        ..
                    } => listed,
                    Presence::Present { .. } => Presence::Absent(Absence::NotOnSearchPath),
                    unusable @ (Presence::Broken { .. } | Presence::Absent(_)) => unusable,
                },
            },
            Detection::Component { variable, file } => {
                let dir = super::host::components_dir().map(|dir| dir.join(requirement.name));
                let value = std::env::var_os(variable);
                if *variable == super::table::PYTHON_WASM_VARIABLE {
                    super::host::python_component(value, dir, file)
                } else {
                    component_at(variable, value, dir, file)
                }
            }
            Detection::Interpreter { variable, fallback } => {
                let Some(platform) = self.platform else {
                    return Presence::Absent(Absence::NotOnSearchPath);
                };
                let named = interpreter_named(
                    std::env::var_os(variable.at(platform)),
                    fallback.at(platform),
                );
                let found = if named.is_absolute() {
                    named.is_file().then_some(named)
                } else {
                    on_search_path(&search_path, &named.display().to_string())
                };
                match found {
                    None => Presence::Absent(Absence::NotOnSearchPath),
                    Some(path) => ask_version(&path, "--version", self.patience),
                }
            }
            Detection::Built { carried } => super::host::built(*carried),
            Detection::Family(family) => super::family::look(*family, self.platform, &search_path),
            Detection::Font { files } => match self.platform {
                Some(platform) => {
                    let home = accounting::home::Home::detect()
                        .ok()
                        .map(|home| home.path().to_path_buf());
                    font_at(platform, home, std::env::var_os("LOCALAPPDATA"), files)
                }
                None => Presence::Absent(Absence::NotInstalled),
            },
        }
    }

    fn core_standing(&self) -> Result<Standing, AxError> {
        let setting = accounting::person::core_priority()?;
        let unasked = |cause: String| {
            AxError::failure(
                AxCode::ToolUnavailable,
                "ask the platform to raise a thread above normal",
                cause,
            )
            .with_recovery("run doctor again; the core's own threads are asked when a city serves")
        };
        std::thread::Builder::new()
            .name("doctor-standing".to_owned())
            .spawn(move || raise_this_thread(setting))
            .map_err(|err| unasked(err.to_string()))?
            .join()
            .map_err(|_| unasked("the thread that asked stopped before it answered".to_owned()))
    }

    fn scanning(&self, city: &Path) -> Scanning {
        super::scanning::read(self.platform, city)
    }
}

impl accounting::Machine for ThisMachine {
    fn report(&self, city: &std::path::Path) -> wire::DoctorAnswer {
        super::answer(self, city)
    }

    fn install(&self, item: &str, runnable: &Runnable<'_>) -> Result<(), AxError> {
        super::running::run(
            item,
            runnable,
            super::running::patience_for(runnable),
            &super::host::install_log(item),
        )
    }
}

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

pub(super) fn names_of(program: &str) -> Vec<String> {
    match super::Platform::current() {
        Some(super::Platform::Windows) => ["exe", "cmd", "bat"]
            .into_iter()
            .map(|extension| format!("{program}.{extension}"))
            .chain(std::iter::once(program.to_owned()))
            .collect(),
        Some(super::Platform::MacOs | super::Platform::Linux) | None => {
            vec![program.to_owned()]
        }
    }
}

pub(crate) fn on_search_path(search_path: &OsString, program: &str) -> Option<PathBuf> {
    let separator = match super::Platform::current() {
        Some(platform) => platform.search_path_separator(),

        None => super::Platform::Linux.search_path_separator(),
    };
    let search_path = search_path.to_str()?.to_owned();
    let names = names_of(program);
    search_path
        .split(separator)
        .filter(|entry| !entry.trim().is_empty())
        .flat_map(|entry| names.iter().map(move |name| Path::new(entry).join(name)))
        .find(|candidate| candidate.is_file())
}

fn at_a_known_place(places: &[&str]) -> Option<PathBuf> {
    places
        .iter()
        .map(PathBuf::from)
        .find(|candidate| candidate.is_file())
}

fn interpreter_named(value: Option<OsString>, fallback: &str) -> PathBuf {
    match value {
        Some(named) if !named.is_empty() => PathBuf::from(named),
        _ => PathBuf::from(fallback),
    }
}

pub(super) fn component_at(
    variable: &'static str,
    value: Option<OsString>,
    dir: Option<PathBuf>,
    file: &str,
) -> Presence {
    if let Some(named) = value.filter(|named| !named.is_empty()) {
        let path = PathBuf::from(named);
        return match std::fs::metadata(&path) {
            Ok(meta) if meta.is_file() => Presence::Present {
                at: path,
                version: Version::Silent,
            },
            Ok(_) => Presence::Broken {
                at: path,
                fault: Fault::Unreadable("not a file".to_owned()),
            },
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                Presence::Absent(Absence::VariableNamesNothing { variable, path })
            }
            Err(err) => Presence::Broken {
                at: path,
                fault: Fault::Unreadable(err.to_string()),
            },
        };
    }
    let Some(dir) = dir else {
        return Presence::Absent(Absence::NoHome);
    };
    match std::fs::metadata(&dir) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            Presence::Absent(Absence::NoComponent { dir })
        }
        Err(err) => Presence::Broken {
            at: dir,
            fault: Fault::Unreadable(err.to_string()),
        },
        Ok(_) => {
            let path = dir.join(file);
            match std::fs::metadata(&path) {
                Ok(meta) if meta.is_file() => Presence::Present {
                    at: path,
                    version: Version::Silent,
                },
                Ok(_) => Presence::Broken {
                    at: path,
                    fault: Fault::Unreadable("not a file".to_owned()),
                },
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => Presence::Broken {
                    at: dir,
                    fault: Fault::HalfWritten,
                },
                Err(err) => Presence::Broken {
                    at: path,
                    fault: Fault::Unreadable(err.to_string()),
                },
            }
        }
    }
}

pub(super) fn ask_version(program: &Path, version_arg: &str, patience: Duration) -> Presence {
    ask(program, &[version_arg], Wanted::First, patience)
}

#[derive(Clone, Copy)]
enum Wanted {
    First,
    StartingWith(&'static str),
}

fn ask(program: &Path, args: &[&str], wanted: Wanted, patience: Duration) -> Presence {
    let spawned = child::command(program)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn();
    let mut child = match spawned {
        Ok(child) => child,
        Err(err) => {
            return Presence::Broken {
                at: program.to_path_buf(),
                fault: Fault::WillNotStart(err.to_string()),
            };
        }
    };
    let version = match child.stdout.take() {
        Some(stdout) => line_of(stdout, wanted, patience),
        None => Version::Silent,
    };

    match super::running::stop(&mut child) {
        None => Presence::Present {
            at: program.to_path_buf(),
            version,
        },

        Some(trouble) => Presence::Broken {
            at: program.to_path_buf(),
            fault: Fault::Unreadable(trouble),
        },
    }
}

fn line_of(stdout: std::process::ChildStdout, wanted: Wanted, patience: Duration) -> Version {
    let (sender, answers) = std::sync::mpsc::channel();
    let reader = std::thread::Builder::new()
        .name("doctor-version".to_owned())
        .spawn(move || {
            let mut lines = std::io::BufReader::new(stdout).lines();
            let first = match wanted {
                Wanted::First => lines.next(),
                Wanted::StartingWith(start) => lines.find(|line| {
                    line.as_ref().map_or(true, |line| {
                        !line.trim().is_empty() && line.starts_with(start)
                    })
                }),
            };

            drop(sender.send(first));
        });
    if reader.is_err() {
        return Version::Silent;
    }
    match answers.recv_timeout(patience) {
        Ok(Some(Ok(line))) if !line.trim().is_empty() => Version::Said(line.trim().to_owned()),
        Ok(Some(Ok(_)) | None) => Version::Silent,
        Ok(Some(Err(_))) => Version::Unreadable,
        Err(RecvTimeoutError::Disconnected) => Version::Silent,
        Err(RecvTimeoutError::Timeout) => Version::Late,
    }
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
