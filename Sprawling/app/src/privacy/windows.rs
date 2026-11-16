// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

pub(super) mod environment;
pub(super) mod host;
pub(super) mod registry;
pub(super) mod task;

use kernel::{AxCode, AxError};
use winreg::RegKey;
use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE, KEY_WOW64_64KEY};
use wire::PrivacyEdition;

const CURRENT_VERSION: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";

pub(crate) const PATIENCE: u32 = 1200;

pub(crate) fn powershell() -> Result<std::path::PathBuf, AxError> {
    system32(r"WindowsPowerShell\v1.0\powershell.exe")
}

pub(crate) fn system32(program: &str) -> Result<std::path::PathBuf, AxError> {
    Ok(system_root()?.join("System32").join(program))
}

pub(crate) fn system_root() -> Result<std::path::PathBuf, AxError> {
    let installation = current_version()
        .and_then(|key| key.get_value::<std::ffi::OsString, _>("SystemRoot"))
        .map_err(|_| unlocated("protected Windows installation path unavailable"))?;
    let root = std::path::PathBuf::from(installation);
    if root.is_absolute() {
        Ok(root)
    } else {
        Err(unlocated("Windows installation path is not absolute"))
    }
}

pub(crate) fn system_root_location() -> std::path::PathBuf {
    std::path::PathBuf::from(format!(r"HKLM\{CURRENT_VERSION}\SystemRoot"))
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct HostFacts {

    pub(super) edition_id: String,

    pub(super) edition: Option<PrivacyEdition>,

    pub(super) build: String,

    pub(super) display_version: Option<String>,
}

pub(super) fn host_facts() -> Result<HostFacts, AxError> {
    let record =
        current_version().map_err(|_| unavailable("Windows version record unavailable"))?;
    let text = |name: &str| record.get_value::<String, _>(name);
    let edition_id =
        text("EditionID").map_err(|_| unavailable("Windows edition is not recorded"))?;
    let current_build =
        text("CurrentBuild").map_err(|_| unavailable("Windows build is not recorded"))?;
    let build = match record.get_value::<u32, _>("UBR") {
        Ok(revision) => format!("{current_build}.{revision}"),
        Err(missing) if missing.kind() == std::io::ErrorKind::NotFound => current_build,
        Err(_) => return Err(unavailable("Windows update revision is unreadable")),
    };
    let display_version = match text("DisplayVersion") {
        Ok(version) => Some(version),
        Err(missing) if missing.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => return Err(unavailable("Windows display version is unreadable")),
    };
    Ok(HostFacts {
        edition: edition(&edition_id),
        edition_id,
        build,
        display_version,
    })
}

fn edition(edition_id: &str) -> Option<PrivacyEdition> {
    [
        ("Core", PrivacyEdition::Home),
        ("Professional", PrivacyEdition::Pro),
        ("Education", PrivacyEdition::Education),
        ("Enterprise", PrivacyEdition::Enterprise),
        ("IoTEnterprise", PrivacyEdition::IotEnterprise),
        ("Server", PrivacyEdition::Server),
    ]
    .into_iter()
    .find_map(|(prefix, edition)| edition_id.starts_with(prefix).then_some(edition))
}

pub(super) fn bytes_of_hex(hex: &str) -> Option<Vec<u8>> {
    let nibble = |byte: u8| {
        (0_u8..)
            .zip(b"0123456789abcdef")
            .find_map(|(value, digit)| (*digit == byte).then_some(value))
    };
    let (pairs, rest) = hex.as_bytes().as_chunks::<2>();
    if !rest.is_empty() {
        return None;
    }
    pairs
        .iter()
        .map(|[high, low]| nibble(*high)?.checked_mul(16)?.checked_add(nibble(*low)?))
        .collect()
}

#[cfg(test)]
pub(crate) fn parses(script: &str) -> bool {
    use crate::doctor::asking::{self, Ended};

    let Ok(executable) = powershell() else {
        return false;
    };
    let mut command = child::command(executable);
    command
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "$errors = $null; [void][Management.Automation.Language.Parser]::ParseInput($env:SPRAWLING_SCRIPT, [ref]$null, [ref]$errors); if ($errors.Count -gt 0) { exit 1 }",
        ])
        .env("SPRAWLING_SCRIPT", script);
    matches!(
        asking::ask(&mut command, PATIENCE, |_| false),
        Ended::Exited { code: Some(0), .. }
    )
}

fn current_version() -> std::io::Result<RegKey> {
    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(CURRENT_VERSION, KEY_QUERY_VALUE | KEY_WOW64_64KEY)
}

fn unlocated(subject: &str) -> AxError {
    AxError::failure(
        AxCode::ToolUnavailable,
        "find a Windows system program",
        subject,
    )
    .with_recovery("run on Windows with a readable system installation record; nothing was started")
}

fn unavailable(subject: &str) -> AxError {
    AxError::failure(AxCode::ToolUnavailable, "read the Windows host record", subject)
        .with_recovery(
            "run on Windows with a readable system installation; nothing was written and privacy history is unchanged",
        )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn the_host_reports_its_edition_and_build() {
        let facts = host_facts().unwrap();
        assert!(!facts.edition_id.is_empty());
        assert!(
            facts
                .build
                .split('.')
                .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit())),
            "{}",
            facts.build
        );
    }

    #[test]
    fn an_edition_id_names_its_family_by_prefix() {
        for (edition_id, named) in [
            ("Core", Some(PrivacyEdition::Home)),
            ("CoreCountrySpecific", Some(PrivacyEdition::Home)),
            ("CoreSingleLanguage", Some(PrivacyEdition::Home)),
            ("Professional", Some(PrivacyEdition::Pro)),
            ("ProfessionalWorkstation", Some(PrivacyEdition::Pro)),
            ("Education", Some(PrivacyEdition::Education)),
            ("Enterprise", Some(PrivacyEdition::Enterprise)),
            ("EnterpriseS", Some(PrivacyEdition::Enterprise)),
            ("IoTEnterprise", Some(PrivacyEdition::IotEnterprise)),
            ("IoTEnterpriseS", Some(PrivacyEdition::IotEnterprise)),
            ("ServerDatacenter", Some(PrivacyEdition::Server)),
            ("ServerStandard", Some(PrivacyEdition::Server)),
            ("", None),
            ("Cloud", None),
            ("core", None),
        ] {
            assert_eq!(edition(edition_id), named, "{edition_id}");
        }
    }

    #[test]
    fn powershell_is_named_under_the_protected_installation() {
        let path = powershell().unwrap();
        assert!(path.is_absolute());
        assert!(path.ends_with(r"System32\WindowsPowerShell\v1.0\powershell.exe"));
    }
}
