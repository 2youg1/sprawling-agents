// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use serde::Deserialize;

use kernel::{Address, AxCode, AxError, BuildingPolicy, EgressAllowlist, EntropyAllowlist};

use super::reach::DomainReach;
use super::{
    BuildingRules, RULES_FILE, UserBrowser, UserBrowserEndpoint, WorkspaceModelPolicy, rules_path,
};

pub fn write_rules_against(
    city_root: &Path,
    addr: &Address,
    base: &str,
    text: &str,
) -> Result<BuildingRules, AxError> {
    write_rules_against_with(city_root, addr, RulesEdit { base, text }, |_written| Ok(()))
}

pub struct RulesEdit<'a> {
    pub base: &'a str,
    pub text: &'a str,
}

pub fn write_rules_against_with(
    city_root: &Path,
    addr: &Address,
    edit: RulesEdit<'_>,
    validate: impl FnOnce(&BuildingRules) -> Result<(), AxError>,
) -> Result<BuildingRules, AxError> {
    let RulesEdit { base, text } = edit;
    let rules = evaluate(addr, text)?;
    validate(&rules)?;
    crate::document::edit_against(
        &rules_path(city_root, addr),
        base.as_bytes(),
        text.as_bytes(),
    )?;
    Ok(rules)
}

pub fn check_reading_room(
    city_root: &Path,
    addr: &Address,
    names: &[String],
) -> Result<(), AxError> {
    reading_room_document(&rules_path(city_root, addr), addr, names).map(|_| ())
}

pub fn write_reading_room(
    city_root: &Path,
    addr: &Address,
    names: &[String],
) -> Result<BuildingRules, AxError> {
    let path = rules_path(city_root, addr);
    crate::document::edit(&path, |held| {
        let (rendered, rules) = reading_room_document(&path, addr, names)?;
        held.replace(rendered.as_bytes())?;
        Ok(rules)
    })
}

fn reading_room_document(
    path: &Path,
    addr: &Address,
    names: &[String],
) -> Result<(String, BuildingRules), AxError> {
    let text = std::fs::read_to_string(path).map_err(|err| {
        AxError::failure(
            kernel::AxCode::StorageFatal,
            "read a building's rules",
            format!("{}: {err}", path.display()),
        )
        .with_recovery("make RULES.toml readable, then save the catalog again")
    })?;
    let mut document: toml::Table =
        toml::from_str(&text).map_err(|err| refuse(addr, err.to_string()))?;
    document.insert(
        "reading_room".to_owned(),
        toml::Value::Array(
            names
                .iter()
                .map(|name| toml::Value::String(name.clone()))
                .collect(),
        ),
    );
    let rendered =
        toml::to_string_pretty(&document).map_err(|err| refuse(addr, err.to_string()))?;
    let rules = evaluate(addr, &rendered)?;
    Ok((rendered, rules))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Written {
    #[serde(default)]
    #[expect(dead_code, reason = "judged on the text before this struct is read")]
    format: Option<toml::Value>,

    #[serde(default)]
    #[expect(
        dead_code,
        reason = "the resident reads these as bytes; a parsed copy would be a second rendering"
    )]
    does: String,

    #[serde(default)]
    #[expect(
        dead_code,
        reason = "the resident reads these as bytes; a parsed copy would be a second rendering"
    )]
    conventions: String,
    confidential: bool,
    write: String,
    #[serde(default)]
    prefixes: Vec<String>,
    #[serde(default)]
    review: bool,
    #[serde(default)]
    browser: bool,
    #[serde(default)]
    usersbrowser: Granted,
    #[serde(default)]
    desktop: bool,

    #[serde(default)]
    egress: Option<Vec<String>>,
    #[serde(default)]
    reading_room: Vec<String>,

    #[serde(default)]
    harness_minutes: Option<std::num::NonZeroU32>,

    #[serde(default)]
    workspace_models: Option<WorkspaceModelPolicy>,

    #[serde(default)]
    checkpoint_entropy_allow: Vec<String>,

    #[serde(default)]
    context_lock_tokens: Option<u64>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Granted {
    Switch(bool),
    At(String),
}

impl Default for Granted {
    fn default() -> Granted {
        Granted::Switch(false)
    }
}

fn refuse(addr: &Address, subject: String) -> AxError {
    AxError::failure(AxCode::ConfigInvalid, "read a building's rules", subject).with_recovery(
        format!(
            "fix the TOML error reported above in `{}/.sprawling/{RULES_FILE}`; the User can \
             edit this file directly, on the raise page, or on the building settings page; \
             residents only read it",
            addr.as_str()
        ),
    )
}

pub fn evaluate(addr: &Address, text: &str) -> Result<BuildingRules, AxError> {
    crate::format::admit_format(crate::format::FormatFile::Rules, text)?;
    let written: Written = toml::from_str(text).map_err(|err| refuse(addr, err.to_string()))?;
    let reach = DomainReach::parse(&written.write)?;
    let mut write_prefixes = Vec::with_capacity(written.prefixes.len());
    for prefix in &written.prefixes {
        write_prefixes.push(Address::parse(prefix)?);
    }
    let checkpoint_entropy_allow = written
        .checkpoint_entropy_allow
        .iter()
        .map(|path| Address::parse(path))
        .collect::<Result<EntropyAllowlist, AxError>>()?;
    let usersbrowser = match written.usersbrowser {
        Granted::Switch(false) => None,
        Granted::Switch(true) => Some(UserBrowser::Waiting),
        Granted::At(address) => Some(UserBrowser::At(UserBrowserEndpoint::parse(&address)?)),
    };
    let confidential = written.confidential;
    let listed = written.egress.as_ref().map_or(0, Vec::len);
    if confidential && listed > 0 {
        return Err(contradiction(
            format!("a confidential building lists {listed} egress domain(s)"),
            "remove the egress list, or drop `confidential = true`; a confidential building's \
             data does not leave, so a domain list beside it contradicts the setting",
        ));
    }
    if confidential && written.browser {
        return Err(contradiction(
            "a confidential building asks for the browser tool".to_owned(),
            "remove `browser = true`, or drop `confidential = true`; a browser opens whatever \
             address it is given, so it is a way out of a building whose data does not leave",
        ));
    }
    if confidential && usersbrowser.is_some() {
        return Err(contradiction(
            "a confidential building asks to drive the User's browser".to_owned(),
            "set `usersbrowser = false`, or drop `confidential = true`; attaching to that \
             browser reads every login it holds, so the per-building isolation this setting \
             depends on does not survive it",
        ));
    }
    if confidential && written.desktop {
        return Err(contradiction(
            "a confidential building asks for this machine's desktop".to_owned(),
            "remove `desktop = true`, or drop `confidential = true`; a desktop holds other \
             programs, other windows and one shared clipboard, and none of them belong to a \
             building whose data does not leave",
        ));
    }
    if let Some(policy) = &written.workspace_models {
        policy.validate()?;
    }
    let context_lock = written
        .context_lock_tokens
        .map(kernel::ContextLock::parse)
        .transpose()?;
    Ok(BuildingRules {
        addr: addr.clone(),
        policy: BuildingPolicy::new(confidential),
        write_prefixes,
        reach,
        egress: if confidential {
            Some(EgressAllowlist::default())
        } else {
            written.egress.map(EgressAllowlist::new)
        },
        review: written.review,
        browser: written.browser,
        usersbrowser,
        desktop: written.desktop,
        reading_room: written.reading_room,
        harness_minutes: written.harness_minutes,
        workspace_models: written.workspace_models,
        checkpoint_entropy_allow,
        context_lock,
    })
}

fn contradiction(subject: String, recovery: &str) -> AxError {
    AxError::failure(
        AxCode::ConfigInvalid,
        "evaluate a building's rules",
        subject,
    )
    .with_recovery(recovery)
}
