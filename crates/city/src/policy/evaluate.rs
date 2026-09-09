// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Reading a `BUILDING.md` into the rules a machine holds.
//!
//! The file is Markdown a person writes, so this is a reader rather than
//! a parser of a format: it looks for the headings and the `key: value`
//! lines it knows and passes over everything else, which is what lets a
//! building's rules also be a document its residents read.

use kernel::{Address, AxCode, AxError, BuildingPolicy, EgressAllowlist};

use super::reach::DomainReach;
use super::{
    BROWSER_KEY, BUILDING_FILE, BuildingRules, CONFIDENTIAL_KEY, DESKTOP_KEY, EGRESS_HEADING,
    READING_HEADING, REVIEW_KEY, WRITE_HEADING, WRITE_KEY,
};

/// Evaluates the text of a `BUILDING.md`.
///
/// # Errors
/// Refuses a file with no confidential declaration, or one whose value is
/// neither `true` nor `false` — a privacy setting that reads as a typo
/// must not resolve to the permissive side.
pub fn evaluate(addr: &Address, text: &str) -> Result<BuildingRules, AxError> {
    let mut confidential: Option<bool> = None;
    let mut reach = DomainReach::Everything;
    let mut review = false;
    let mut browser = false;
    let mut desktop = false;
    let mut write_prefixes = Vec::new();
    let mut egress_entries: Vec<String> = Vec::new();
    let mut in_write_section = false;
    let mut in_egress_section = false;
    let mut in_reading_section = false;
    let mut reading_room: Vec<String> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        let bare = trimmed.trim_start_matches(['#', '>', '`', '-', '*', ' ']);
        if trimmed.starts_with('#') {
            let heading = trimmed.to_ascii_lowercase();
            in_write_section = heading.contains(WRITE_HEADING);
            in_egress_section = heading.contains(EGRESS_HEADING);
            in_reading_section = heading.contains(READING_HEADING);
            continue;
        }
        if in_reading_section && (trimmed.starts_with("- ") || trimmed.starts_with("* ")) {
            let entry = bare.trim().trim_matches('`').trim();
            if !entry.is_empty() {
                reading_room.push(entry.to_owned());
            }
            continue;
        }
        if in_egress_section && (trimmed.starts_with("- ") || trimmed.starts_with("* ")) {
            let entry = bare.trim().trim_matches('`').trim();
            if !entry.is_empty() {
                egress_entries.push(entry.to_owned());
            }
            continue;
        }
        if let Some(rest) = bare.strip_prefix(WRITE_KEY) {
            reach = DomainReach::parse(rest.trim().trim_matches('`').trim())?;
            continue;
        }
        if let Some(rest) = bare.strip_prefix(REVIEW_KEY) {
            review = flag("review", rest)?;
            continue;
        }
        if let Some(rest) = bare.strip_prefix(BROWSER_KEY) {
            browser = flag("browser", rest)?;
            continue;
        }
        if let Some(rest) = bare.strip_prefix(DESKTOP_KEY) {
            desktop = flag("desktop", rest)?;
            continue;
        }
        if let Some(rest) = bare.strip_prefix(CONFIDENTIAL_KEY) {
            confidential = Some(flag("confidential", rest)?);
            continue;
        }
        if in_write_section
            && (trimmed.starts_with("- ") || trimmed.starts_with("* "))
            && let Ok(prefix) = Address::parse(bare.trim().trim_matches('`'))
        {
            write_prefixes.push(prefix);
        }
    }
    let Some(confidential) = confidential else {
        return Err(AxError::failure(
            AxCode::ConfigInvalid,
            "evaluate a building's rules",
            format!("{BUILDING_FILE} does not say whether this building is confidential"),
        )
        .with_recovery("add a `confidential: false` line, or `true` and read what it changes"));
    };
    if confidential && !egress_entries.is_empty() {
        return Err(AxError::failure(
            AxCode::ConfigInvalid,
            "evaluate a building's rules",
            format!(
                "a confidential building lists {} egress domain(s)",
                egress_entries.len()
            ),
        )
        .with_recovery(
            "remove the egress list, or drop `confidential: true`; a confidential building's \
             data does not leave, so a domain list under it contradicts the setting above it",
        ));
    }
    if confidential && browser {
        return Err(AxError::failure(
            AxCode::ConfigInvalid,
            "evaluate a building's rules",
            "a confidential building asks for the browser tool",
        )
        .with_recovery(
            "remove the `browser: true` line, or drop `confidential: true`; a browser opens \
             whatever address it is given, so it is a way out of a building whose data does \
             not leave",
        ));
    }
    if confidential && desktop {
        return Err(AxError::failure(
            AxCode::ConfigInvalid,
            "evaluate a building's rules",
            "a confidential building asks for this machine's desktop",
        )
        .with_recovery(
            "remove the `desktop: true` line, or drop `confidential: true`; a desktop holds              other programs, other windows and one shared clipboard, and none of them belong              to a building whose data does not leave",
        ));
    }
    Ok(BuildingRules {
        addr: addr.clone(),
        policy: BuildingPolicy::new(confidential),
        write_prefixes,
        reach,
        egress: EgressAllowlist::new(egress_entries),
        review,
        browser,
        desktop,
        reading_room,
    })
}

/// One `key: true|false` line, read the same way for every key.
///
/// # Errors
/// Refuses anything but the two words. A setting that reads as a typo
/// resolves to neither side: which side is the safe one differs per key,
/// and a reader who has to know that is a reader who will guess wrong.
fn flag(name: &str, rest: &str) -> Result<bool, AxError> {
    match rest.trim().trim_matches('`').trim() {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(AxError::failure(
            AxCode::ConfigInvalid,
            "evaluate a building's rules",
            format!("`{name}: {other}` is neither true nor false"),
        )
        .with_recovery(format!("write `{name}: true` or `{name}: false`"))),
    }
}
