// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::PathBuf;

pub(super) fn installed_at(program: &str, start_menu: &str) -> Option<PathBuf> {
    match super::Platform::current() {
        Some(super::Platform::Windows) => {}
        Some(super::Platform::MacOs | super::Platform::Linux) | None => return None,
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

fn default_value(key: &str) -> Option<String> {
    let output = child::command("reg")
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
