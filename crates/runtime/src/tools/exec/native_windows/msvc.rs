// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The MSVC search directories an AppContainer child cannot discover for
//! itself, specified by `crates/runtime/spec/Tools/Exec/NativeWindows.lean` D60.

use std::ffi::{OsStr, OsString};
use std::process::Command;

use find_msvc_tools::{Env, EnvGetter};

/// The search lists the linker's environment carries. The discovery never
/// reads the child's values of these, so runtime alone joins them.
const SEARCH_LISTS: [&str; 3] = ["PATH", "LIB", "INCLUDE"];

/// Prepends the host linker's PATH, LIB and INCLUDE directories to the values
/// the child declared, so a bare `link.exe` or `cl.exe` inside the container
/// resolves to MSVC. Leaves the command unchanged when the building declared a
/// developer prompt (`VCINSTALLDIR`) or no MSVC is installed.
pub(super) fn prepend_search_directories(command: &mut Command) {
    let declared = Declared(
        command
            .get_envs()
            .filter_map(|(name, value)| Some((name.to_os_string(), value?.to_os_string())))
            .collect(),
    );
    if declared.value("VCINSTALLDIR").is_some() {
        return;
    }
    let Some(linker) =
        find_msvc_tools::find_tool_with_env(std::env::consts::ARCH, "link.exe", &declared)
    else {
        return;
    };
    for (name, directories) in linker.env() {
        if !is_search_list(name) {
            continue;
        }
        let joined = std::env::split_paths(directories)
            .map(std::path::PathBuf::into_os_string)
            .chain(declared.value(name).map(OsStr::to_os_string))
            .filter(|entry| !entry.is_empty())
            .fold(OsString::new(), |mut joined, entry| {
                if !joined.is_empty() {
                    joined.push(";");
                }
                joined.push(entry);
                joined
            });
        command.env(name, joined);
    }
}

fn is_search_list(name: impl AsRef<OsStr>) -> bool {
    SEARCH_LISTS
        .iter()
        .any(|list| name.as_ref().eq_ignore_ascii_case(list))
}

/// The child's declared environment, as the discovery sees it.
struct Declared(Vec<(OsString, OsString)>);

impl Declared {
    fn value(&self, name: impl AsRef<OsStr>) -> Option<&OsStr> {
        self.0
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name.as_ref()))
            .map(|(_, value)| value.as_os_str())
    }
}

impl EnvGetter for Declared {
    fn get_env(&self, name: &'static str) -> Option<Env> {
        if is_search_list(name) {
            return None;
        }
        self.value(name)
            .map(|value| Env::Owned(value.to_os_string()))
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use std::ffi::OsStr;
    use std::path::PathBuf;
    use std::process::Command;

    fn declared<'a>(command: &'a Command, name: &str) -> Vec<&'a OsStr> {
        command
            .get_envs()
            .filter(|(key, _)| key.eq_ignore_ascii_case(name))
            .filter_map(|(_, value)| value)
            .collect()
    }

    /// Runs where MSVC is installed, as on every Windows CI runner: the
    /// declared spelling `Path` and the linker's `PATH` are one variable, the
    /// linker directory comes first and the declared directory is kept last.
    #[test]
    fn the_linker_directory_precedes_the_declared_search_path() {
        let mut command = Command::new("cmd.exe");
        command.env_clear().env("Path", r"C:\declared");
        super::prepend_search_directories(&mut command);

        let path = declared(&command, "PATH");
        assert_eq!(path.len(), 1, "one PATH under every spelling: {path:?}");
        let directories: Vec<PathBuf> = std::env::split_paths(path[0]).collect();
        assert!(
            directories
                .first()
                .is_some_and(|first| first.join("link.exe").is_file()),
            "MSVC's link.exe directory leads PATH: {directories:?}"
        );
        assert_eq!(directories.last(), Some(&PathBuf::from(r"C:\declared")));
        assert_eq!(declared(&command, "LIB").len(), 1);
    }

    #[test]
    fn a_declared_developer_prompt_is_left_alone() {
        let mut command = Command::new("cmd.exe");
        command
            .env_clear()
            .env("VCINSTALLDIR", r"C:\vc\")
            .env("PATH", r"C:\declared");
        super::prepend_search_directories(&mut command);

        assert_eq!(declared(&command, "PATH"), [OsStr::new(r"C:\declared")]);
        assert!(declared(&command, "LIB").is_empty());
    }
}
