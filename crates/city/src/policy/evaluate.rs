// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Reading a `RULES.toml` into the rules a machine holds.
//!
//! **Every key this version reads is a field of [`Written`].** A key it
//! does not know is refused rather than passed over, and a key is a key
//! only where the grammar puts one — which is the whole reason this
//! file is TOML and the prose beside it is not. A reader that matched
//! `confidential:`, `write:` or `desktop:` on *any* line of a prose
//! file would let a sentence that began `desktop: true` grant this
//! machine's desktop, and would resolve a `write:` line nobody wrote to
//! `Everything`. Both fail towards the permissive side, which is the one
//! direction a permission reader may never fail in.
//!
//! There is no second document. The prefix carries this file's own
//! bytes, so the prose a resident reads and the settings the city
//! enforces cannot describe two different buildings.
//!
//! Specified by `crates/city/spec/Policy.lean` §8-2.

use std::path::Path;

use serde::Deserialize;

use kernel::{Address, AxCode, AxError, BuildingPolicy, EgressAllowlist, EntropyAllowlist};

use super::reach::DomainReach;
use super::{
    BuildingRules, RULES_FILE, UserBrowser, UserBrowserEndpoint, WorkspaceModelPolicy, rules_path,
};

/// Replaces a building's rules with a page's whole text, evaluated
/// first, and only if the file still holds `base`: a page left open
/// while the file was edited elsewhere must not write over that edit
/// (`crates/city/spec/Policy.lean` §8-34).
///
/// The evaluation comes before the write, so the file on disk is always
/// one this build can read: a governance document that stopped parsing
/// halfway through a rewrite would take its building with it. The path
/// is inside the scope's reserved subtree, which no write domain
/// reaches, which is why this is a door of its own and not an `edit`.
///
/// # Errors
/// The evaluation's refusal, `E_VERSION_CONFLICT` for a file that is no
/// longer `base`, and a directory or file that cannot be written.
pub fn write_rules_against(
    city_root: &Path,
    addr: &Address,
    base: &str,
    text: &str,
) -> Result<BuildingRules, AxError> {
    write_rules_against_with(city_root, addr, RulesEdit { base, text }, |_written| Ok(()))
}

/// The text a page proposes, and the version it read first. One value
/// because a write that named only one of them could land over an edit
/// nobody saw, and both are always supplied together.
pub struct RulesEdit<'a> {
    pub base: &'a str,
    pub text: &'a str,
}

/// Writes rules after both structural evaluation and one caller-supplied
/// registry check. The check runs before the baseline comparison and write,
/// so neither kind of refusal leaves a partial policy behind.
///
/// # Errors
/// The evaluation's refusal, the validator's own refusal, and
/// `E_VERSION_CONFLICT` for a file that is no longer `base`.
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

/// Validates a Reading Room replacement without writing it.
pub fn check_reading_room(
    city_root: &Path,
    addr: &Address,
    names: &[String],
) -> Result<(), AxError> {
    reading_room_document(&rules_path(city_root, addr), addr, names).map(|_| ())
}

/// Replaces only the Reading Room's name list, preserving the other RULES
/// keys. The complete result is evaluated before it lands.
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

/// The file as a person wrote it, before any rule is judged.
///
/// One struct, so the set of keys this version reads is a thing the
/// compiler holds rather than a list some reader has to stay in step
/// with. `write` and `confidential` have no default: they are the two
/// answers whose absence would otherwise resolve to the permissive side.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Written {
    /// Judged by `format::admit_format` before this struct is read, and
    /// declared so `deny_unknown_fields` still covers every other key.
    #[serde(default)]
    #[expect(dead_code, reason = "judged on the text before this struct is read")]
    format: Option<toml::Value>,
    /// What this building is, in the person's words, and how work is
    /// done here.
    ///
    /// Declared and deliberately not read. Their reader is the resident,
    /// which is handed this file's own bytes; parsing them into
    /// [`BuildingRules`] as well would be a second rendering of text
    /// that already went out whole. They are fields rather than TOML
    /// comments so that `deny_unknown_fields` still covers the file: a
    /// person who misspells `conventions` is told, instead of writing a
    /// paragraph nothing keeps.
    #[serde(default)]
    #[expect(
        dead_code,
        reason = "the resident reads these as bytes; a parsed copy would be a second rendering"
    )]
    does: String,
    /// How work is done here, on the same terms as `does`.
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
    /// Absent is a building with no domain list, which reaches any host
    /// its browser keys allow; a list, empty included, is the only hosts
    /// it reaches (`crates/city/spec/Policy.lean` D27).
    #[serde(default)]
    egress: Option<Vec<String>>,
    #[serde(default)]
    reading_room: Vec<String>,
    /// How long a harness run here may take, in minutes; zero is
    /// refused by the type (city D8 (b)).
    #[serde(default)]
    harness_minutes: Option<std::num::NonZeroU32>,
    /// Optional native-model routing for delegated work in this workspace.
    #[serde(default)]
    workspace_models: Option<WorkspaceModelPolicy>,
    /// Exact repository paths whose entropy-only checkpoint hits are exempt.
    #[serde(default)]
    checkpoint_entropy_allow: Vec<String>,
    /// Absolute context lock in input tokens; absent is reminder-only.
    #[serde(default)]
    context_lock_tokens: Option<u64>,
}

/// What `usersbrowser` may say: off, on with no address declared yet,
/// or the address itself.
///
/// One key rather than two, for the reason [`UserBrowser`] gives: a
/// person who wrote the address has answered both questions at once,
/// and two keys would be two answers that can disagree.
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

/// The refusal for text this version cannot read.
///
/// **It does not restate the schema.** `deny_unknown_fields` makes the
/// deserialiser name the key it met and the keys it expected, and a
/// second list written here would be the one that goes stale the first
/// time [`Written`] grows a field. What this adds is what the
/// deserialiser cannot know: whose file this is and who may change it.
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

/// Evaluates the text of a `RULES.toml`.
///
/// # Errors
/// Refuses text that is not this version's TOML — a key it does not
/// know, a missing `confidential` or `write`, a value of the wrong
/// type — a write prefix that is not an address, and the four settings
/// a confidential building contradicts by asking for.
pub fn evaluate(addr: &Address, text: &str) -> Result<BuildingRules, AxError> {
    crate::format::admit_format(crate::format::FormatFile::Rules, text)?;
    let written: Written = toml::from_str(text).map_err(|err| refuse(addr, err.to_string()))?;
    let reach = DomainReach::parse(&written.write)?;
    let mut write_prefixes = Vec::with_capacity(written.prefixes.len());
    for prefix in &written.prefixes {
        // Propagated rather than skipped. A prefix the grammar refuses,
        // dropped where it was read, would leave a building writing less
        // than the person granted it and saying so nowhere.
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

/// Two settings that are each legal and cannot both hold.
///
/// Separate from [`refuse`] because the recovery is the pair, not the
/// schema: a reader told to consult the key list here would read a list
/// on which both of their settings appear.
fn contradiction(subject: String, recovery: &str) -> AxError {
    AxError::failure(
        AxCode::ConfigInvalid,
        "evaluate a building's rules",
        subject,
    )
    .with_recovery(recovery)
}
