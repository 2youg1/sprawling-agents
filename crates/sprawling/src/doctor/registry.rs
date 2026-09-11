// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where Windows records a browser that no shell resolves
//! (sprawling-SPEC.md section 8-57).
//!
//! Windows installs a per-user browser under a directory whose spelling
//! carries the person's own account name, so no literal path in the
//! table can find it. The two keys every installer writes do:
//! `App Paths` says what a program name resolves to, and
//! `StartMenuInternet` says what a browser's "open" command is.
//!
//! **Read through `reg.exe`, not through a registry crate.** The
//! alternative is one more dependency in a binary a person downloads,
//! for one platform and one question; `reg.exe` ships with the
//! operating system that has the keys. Nothing here writes: every call
//! is `reg query`.

use std::path::PathBuf;

/// The path a browser is installed at, by whichever key names it.
///
/// `None` off Windows, and `None` when neither key answers. A key whose
/// value is not a path this machine has is no answer: the caller goes
/// on to the next member rather than reporting a browser nobody can
/// start.
pub(super) fn installed_at(program: &str, start_menu: &str) -> Option<PathBuf> {
    if !cfg!(target_os = "windows") {
        return None;
    }
    let app_path = format!("{program}.exe");
    let mut keys = vec![
        format!(r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\{app_path}"),
        format!(r"HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\{app_path}"),
    ];
    if !start_menu.is_empty() {
        keys.push(format!(
            r"HKLM\SOFTWARE\Clients\StartMenuInternet\{start_menu}\shell\open\command"
        ));
        keys.push(format!(
            r"HKCU\SOFTWARE\Clients\StartMenuInternet\{start_menu}\shell\open\command"
        ));
    }
    keys.iter()
        .filter_map(|key| default_value(key))
        .map(|value| PathBuf::from(program_in(&value)))
        .find(|path| path.is_file())
}

/// The default value of one key, as `reg.exe` prints it.
fn default_value(key: &str) -> Option<String> {
    let output = std::process::Command::new("reg")
        .args(["query", key, "/ve"])
        .stdin(std::process::Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let printed = String::from_utf8(output.stdout).ok()?;
    printed
        .lines()
        .find_map(|line| line.split_once("REG_SZ"))
        .map(|(_, value)| value.trim().to_owned())
}

/// The program a command line starts, without its arguments.
///
/// A `StartMenuInternet` command is a quoted path followed by whatever
/// the installer wanted passed; an `App Paths` value is the path alone.
/// Both are read the same way: what is inside the quotes, or everything
/// before the first argument.
fn program_in(value: &str) -> &str {
    if let Some(quoted) = value.strip_prefix('"') {
        return quoted.split('"').next().unwrap_or(quoted);
    }
    match value.find(" -") {
        Some(cut) => value.get(..cut).unwrap_or(value).trim_end(),
        None => value,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::program_in;

    /// Both spellings of the value read back as one path, so a browser
    /// registered either way is found once rather than twice.
    #[test]
    fn a_command_line_reads_back_as_the_program_it_starts() {
        assert_eq!(
            program_in(r#""C:\Program Files\Zen Browser\zen.exe" -osint -url "%1""#),
            r"C:\Program Files\Zen Browser\zen.exe"
        );
        assert_eq!(
            program_in(r"C:\Program Files\Mozilla Firefox\firefox.exe"),
            r"C:\Program Files\Mozilla Firefox\firefox.exe"
        );
        assert_eq!(
            program_in(r"C:\Program Files\Vivaldi\Application\vivaldi.exe -- %1"),
            r"C:\Program Files\Vivaldi\Application\vivaldi.exe"
        );
    }
}
