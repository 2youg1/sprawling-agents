// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Progressive disclosure and the truncation lock
//! (`crates/runtime/Spec.lean` §8-60, §8-61). Only what this session can
//! actually reach is listed — admission is the caller's evidence: the
//! assembler registers the tools a building's vocation, rules and book
//! allow, and reads each building's reading room off its `city::policy`
//! rules. Of what is admitted, the mode's core travels as tools; the rest
//! is one line each in the dormant index, and a run fetches a guide with
//! `describe` and runs a dormant tool through `call`.
//!
//! `tool_defs` is the single source of `ChatRequest.tools`: a tool absent
//! here and absent from the dormant index does not exist for the model.

use std::collections::BTreeMap;

pub use kernel::event::record::SkillPin;

use kernel::{AxCode, AxError, B3Hash, ToolCall, ToolDef, ToolMeta};

use crate::mode::catalog_entry;
use kernel::Mode;

mod dormant;
mod fit;
mod guide;

/// The most bytes the dormant index may take, header and `+N more` line
/// included, whatever the building admits (runtime D19).
pub const DORMANT_INDEX_CEILING: usize = 1024;

/// One disclosed row: "what it is + when to use it" resident-side, the
/// "how to use it" expansion fetched on demand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogEntry {
    pub name: String,
    pub disclosure: String,
    pub expansion: String,
    /// What the document behind this entry hashed to when the shelf was
    /// read. `None` for an entry the catalog holds as text of its own,
    /// which has no document to change behind anybody's back.
    pub hash: Option<B3Hash>,
    /// The package directory, when the skill is filed as one: the files
    /// beside its document open by `<name>/<path>`. Said by the shelf's
    /// scan, never guessed from how `expansion` is spelled.
    pub package: Option<String>,
}

/// What a second-level disclosure turns out to be.
///
/// Exhaustive: an entry either lives somewhere the run can open, or is
/// text the catalog itself holds. Neither is in the prompt — the prompt
/// carries one line per entry, and this is what that line was standing
/// in for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expansion {
    /// A skill, the address it is kept at, and its package directory
    /// when it is filed as one.
    Skill {
        addr: String,
        package: Option<String>,
    },
    /// Text the catalog holds: the mode's own discipline, the developer
    /// entry's, or a skill carried in from a shelf outside the city.
    Said { text: String },
}

#[derive(Debug, Default)]
pub struct Catalog {
    tools: BTreeMap<String, ToolDef>,
    skills: BTreeMap<String, Admitted>,
    mode: Option<Mode>,
}

/// One admitted skill, and which door it came in by: the door decides
/// whether its `expansion` is an address or the document's text.
#[derive(Debug)]
struct Admitted {
    entry: CatalogEntry,
    kept: Kept,
}

/// Where a run finds the text of an admitted skill.
#[derive(Debug, Clone, Copy)]
enum Kept {
    /// On a shelf inside the city, at the address `expansion` spells.
    Shelved,
    /// In the catalog itself: `expansion` is the text the scan read off a
    /// shelf outside the city, which has no address (`crates/runtime/spec/Tools/Read.lean`
    /// §8-29-6).
    Carried,
}

impl Catalog {
    pub fn new() -> Catalog {
        Catalog::default()
    }

    /// Registers an L2 tool (or an L0 tool for wire schema purposes).
    /// Eight-field completeness is the type's business; what is checked
    /// here is catalog hygiene: non-empty disclosure, no duplicate name.
    pub fn admit_tool(&mut self, meta: &ToolMeta) -> Result<(), AxError> {
        if meta.disclosure.trim().is_empty() {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "admit tool to catalog",
                format!("{}: empty disclosure", meta.name),
            )
            .with_recovery(format!(
                "write one sentence in the `disclosure` field of `{}`'s `ToolMeta` \
                 saying what the tool does",
                meta.name
            )));
        }
        let key = meta.name.as_str().to_owned();
        if self.tools.contains_key(&key) {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "admit tool to catalog",
                format!("{key}: duplicate name"),
            )
            .with_recovery(format!(
                "rename one of the two tools declaring `{key}` in its `ToolMeta`: a \
                 catalog name points at one tool"
            )));
        }
        self.tools.insert(
            key,
            ToolDef {
                name: meta.name.clone(),
                description: meta.disclosure.clone(),
                input_schema: meta.params.clone(),
            },
        );
        Ok(())
    }

    /// Registers one reading-room-admitted SKILL entry kept inside the
    /// city: `expansion` is the address a run opens it at.
    ///
    /// # Errors
    /// An empty name or disclosure, and a name already admitted by
    /// either door, are `E_INVALID_ARGS`.
    pub fn admit_skill(&mut self, entry: CatalogEntry) -> Result<(), AxError> {
        self.admit(entry, Kept::Shelved)
    }

    /// Registers one reading-room-admitted SKILL entry from a shelf
    /// outside the city: `expansion` is the document's text, which a
    /// read by name hands back as it stands (`crates/runtime/spec/Tools/Read.lean` §8-29-6).
    ///
    /// # Errors
    /// The same refusals as [`Catalog::admit_skill`].
    pub fn admit_carried_skill(&mut self, entry: CatalogEntry) -> Result<(), AxError> {
        self.admit(entry, Kept::Carried)
    }

    /// The one hygiene check both doors share.
    fn admit(&mut self, entry: CatalogEntry, kept: Kept) -> Result<(), AxError> {
        if entry.name.trim().is_empty() || entry.disclosure.trim().is_empty() {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "admit skill to catalog",
                "empty name or disclosure",
            )
            .with_recovery(
                "give the SKILL file a title and a one-line description; the catalog \
                 shows both and a resident picks the skill by them",
            ));
        }
        if self.skills.contains_key(&entry.name) {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "admit skill to catalog",
                format!("{}: duplicate name", entry.name),
            )
            .with_recovery(format!(
                "rename one of the two SKILL files titled `{}`: a catalog name points \
                 at one skill",
                entry.name
            )));
        }
        self.skills
            .insert(entry.name.clone(), Admitted { entry, kept });
        Ok(())
    }

    /// The run sits in exactly one mode; the catalog shows only it.
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = Some(mode);
    }

    /// The Resident-segment text: the header line, the mode this run sits
    /// in, the developer entry, then the dormant index. BTreeMap order
    /// makes the bytes a pure function of the content.
    ///
    /// **The core tools are not among them.** Their name, disclosure and
    /// schema travel in `ChatRequest.tools` on every turn, and writing
    /// them here as well put every tool's sentence into the prompt twice.
    /// Every other admitted tool and every admitted skill is one line of
    /// the dormant index, which stays under [`DORMANT_INDEX_CEILING`]
    /// however much the building admits.
    pub fn render(&self) -> String {
        let mut out = String::from(
            "Catalog: what you can reach beyond the tools listed with this request. \
             Open an entry by name with `read` before first use.\n",
        );
        if let Some(mode) = self.mode {
            let entry = catalog_entry(mode);
            out.push_str("- ");
            out.push_str(&entry.name);
            out.push_str(": ");
            out.push_str(&entry.disclosure);
            out.push('\n');
        }
        // The one line that says this city is changeable from inside
        // it. The discipline behind it is fetched, not carried.
        let dev = crate::mode::dev_entry();
        out.push_str("- ");
        out.push_str(&dev.name);
        out.push_str(": ");
        out.push_str(&dev.disclosure);
        out.push('\n');
        out.push_str(&dormant::index(&self.dormant_entries()));
        out
    }

    /// Every admitted capability a session does not carry as a tool, in
    /// the index's order: the tools outside the core, then the skills.
    fn dormant_entries(&self) -> Vec<dormant::Entry<'_>> {
        self.tools
            .iter()
            .filter(|(name, _)| !self.is_core(name))
            .map(|(name, def)| dormant::Entry {
                label: name.clone(),
                about: &def.description,
            })
            .chain(self.skills.iter().map(|(name, admitted)| dormant::Entry {
                label: format!("skill {name}"),
                about: &admitted.entry.disclosure,
            }))
            .collect()
    }

    /// Whether `name` travels as a tool in this run's requests: it is in
    /// the core of the mode the run sits in (`mode::core_tools`). A
    /// catalog told no mode carries no tool, so nothing reaches a model
    /// that the mode did not decide.
    fn is_core(&self, name: &str) -> bool {
        self.mode
            .is_some_and(|mode| crate::mode::core_tools(mode).contains(&name))
    }

    /// The call a `call` stands for, with the id the model gave it, so its
    /// result pairs with the `call` in the conversation and its tool
    /// passes its own doors (`crates/runtime/Spec.lean` §8-61).
    ///
    /// # Errors
    /// `E_INVALID_ARGS` when the arguments name no tool or carry no
    /// object, when they name `call` itself, when the name is not a tool
    /// this run was admitted, and when the arguments do not fit the
    /// tool's input schema; the last carries the schema.
    pub fn resolve_call(&self, call: &ToolCall) -> Result<ToolCall, AxError> {
        fit::resolve(&self.tools, call)
    }

    /// The only source of `ChatRequest.tools`: the admitted tools in the
    /// core of this run's mode, and no other.
    pub fn tool_defs(&self) -> Vec<ToolDef> {
        self.tools
            .iter()
            .filter(|(name, _)| self.is_core(name))
            .map(|(_, def)| def.clone())
            .collect()
    }

    /// What this run was given, and what each one hashed to.
    ///
    /// Taken from the catalog rather than from a second scan of the
    /// shelves: the catalog is already the authority on what a run can
    /// reach, and a second reading would be a second answer to that
    /// question taken at a different instant.
    pub fn skill_pins(&self) -> Vec<SkillPin> {
        self.skills
            .iter()
            .filter_map(|(name, admitted)| {
                admitted.entry.hash.map(|hash| SkillPin {
                    name: name.clone(),
                    hash,
                })
            })
            .collect()
    }

    /// Second-level disclosure. Tools expand to their schema via
    /// `tool_defs` (the wire always carries it); skills and the mode
    /// expand through here.
    ///
    /// The two answers are different kinds of thing — a place to open
    /// and a text already in the prompt — so they are different
    /// variants. Collapsing both into one string made the caller guess
    /// which it had by trying to parse it as an address, and a mode
    /// whose text happened to parse would have been opened as a file.
    pub fn expand(&self, name: &str) -> Option<Expansion> {
        if let Some(Admitted { entry, kept }) = self.skills.get(name) {
            return Some(match kept {
                Kept::Shelved => Expansion::Skill {
                    addr: entry.expansion.clone(),
                    package: entry.package.clone(),
                },
                Kept::Carried => Expansion::Said {
                    text: entry.expansion.clone(),
                },
            });
        }
        if let Some(mode) = self.mode {
            let entry = catalog_entry(mode);
            if entry.name == name {
                return Some(Expansion::Said {
                    text: entry.expansion,
                });
            }
        }
        if name == crate::mode::DEV_ENTRY {
            return Some(Expansion::Said {
                text: crate::mode::dev_entry().expansion,
            });
        }
        None
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
