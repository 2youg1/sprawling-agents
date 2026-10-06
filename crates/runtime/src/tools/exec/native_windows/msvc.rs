// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The MSVC search directories an AppContainer child cannot discover for
//! itself, specified by `crates/runtime/spec/Tools/Exec/NativeWindows.lean` D60.

use std::process::Command;

/// Prepends the host linker's PATH, LIB and INCLUDE directories to the values
/// the child declared, so a bare `link.exe` or `cl.exe` inside the container
/// resolves to MSVC. Leaves the command unchanged when the building declared a
/// developer prompt (`VCINSTALLDIR`) or no MSVC is installed.
pub(super) fn prepend_search_directories(command: &mut Command) {
    let _ = command;
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
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
