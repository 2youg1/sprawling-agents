// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Progressive disclosure and the truncation lock
//! (`crates/runtime/Spec.lean` §8-60, §8-61). Only what this session can
//! actually reach is listed — admission is the caller's evidence: the
//! assembler registers the tools a building's vocation, rules and book
//! allow, and reads each building's reading room off its `city::policy`
//! rules. Each tool it registers takes an exposure here, from the
//! building's and the city's `[catalog]` tables or the default
//! ([`exposure`]): the core travels as tools, a dormant tool waits in the
//! dormant index, one line per namespace and one per skill, and a run
//! lists, finds, details and runs it through `tool`; an `Off` tool is
//! kept only as a row of the table this run records.
//!
//! `tool_defs` is the single source of `ChatRequest.tools`: a tool absent
//! here and absent from the dormant index does not exist for the model.

use std::collections::BTreeMap;

pub use kernel::event::record::SkillPin;

use kernel::config::{CatalogExposure, Exposure, ExposureFrom};
use kernel::event::record::{PromptTool, PromptTools};
use kernel::{AxCode, AxError, B3Hash, ReservedNamespace, ToolCall, ToolDef, ToolMeta};

use crate::mode::catalog_entry;
use kernel::Mode;

mod dormant;
pub mod exposure;
mod fit;
mod guide;
mod namespace;

pub use guide::{FIND_HITS, LIST_CEILING};
pub use namespace::Namespace;

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
    /// The admitted tools a run can reach, core or dormant.
    tools: BTreeMap<String, Filed>,
    /// The tools a table turned off: no request, index or call reaches
    /// them, and they are kept only for the table this run records.
    withheld: BTreeMap<String, Filed>,
    skills: BTreeMap<String, Admitted>,
    mode: Option<Mode>,
    exposure: CatalogExposure,
}

/// One admitted tool: what the model is told it is, the namespace it is
/// filed under, and how it reaches the run.
#[derive(Debug)]
struct Filed {
    def: ToolDef,
    namespace: Namespace,
    exposure: Exposure,
    from: ExposureFrom,
}

/// The tool table one run was frozen with: the core array as the catalog
/// serialises it, which the assembler keeps in the content store, and the
/// record `prompt_assembled` carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolTable {
    pub array: Vec<u8>,
    pub record: PromptTools,
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

    /// Registers a built tool under the namespace it is filed in, at the
    /// exposure [`exposure::exposure_of`] gives it, and answers that
    /// exposure: an `Off` tool is withheld, and the caller registers no
    /// route for it, so a call by its name reaches nothing.
    /// Eight-field completeness is the type's business; what is checked
    /// here is catalog hygiene: non-empty disclosure, no duplicate name.
    ///
    /// # Errors
    /// An empty disclosure and a name already registered are
    /// `E_INVALID_ARGS`.
    pub fn admit_tool(
        &mut self,
        meta: &ToolMeta,
        namespace: Namespace,
    ) -> Result<Exposure, AxError> {
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
        if self.tools.contains_key(&key) || self.withheld.contains_key(&key) {
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
        let (exposure, from) = exposure::exposure_of(&self.exposure, &key, namespace.as_str());
        let filed = Filed {
            def: ToolDef {
                name: meta.name.clone(),
                description: meta.disclosure.clone(),
                input_schema: meta.params.clone(),
            },
            namespace,
            exposure,
            from,
        };
        match exposure {
            Exposure::Core | Exposure::Dormant => self.tools.insert(key, filed),
            Exposure::Off => self.withheld.insert(key, filed),
        };
        Ok(exposure)
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

    /// The `[catalog]` tables this run is frozen with. Set before the
    /// first tool is admitted: a tool takes its exposure as it is
    /// admitted.
    pub fn set_exposure(&mut self, stated: CatalogExposure) {
        self.exposure = stated;
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
    /// the index's order: one entry per namespace of the tools outside
    /// the core, then the skills under one line that says what a skill
    /// is, one entry each (`crates/runtime/spec/Catalog.lean` §8-60).
    ///
    /// The `skill` namespace's tools are counted in that line rather than
    /// given one of their own, so the sentence is said once.
    fn dormant_entries(&self) -> Vec<dormant::Entry> {
        let skill = Namespace::Own(ReservedNamespace::Skill);
        let mut namespaces = self.dormant_namespaces();
        let filed_as_skill = namespaces
            .remove(&(skill.as_str(), &skill))
            .unwrap_or_default();
        let skills = filed_as_skill.len().saturating_add(self.skills.len());
        let opening = (skills > 0).then(|| dormant::Entry {
            label: format!("{} ({skills})", skill.as_str()),
            about: skill.about(filed_as_skill.iter().copied()),
        });
        namespaces
            .into_iter()
            .map(|((_, namespace), members)| dormant::Entry {
                label: format!("{} ({})", namespace.as_str(), members.len()),
                about: namespace.about(members.iter().copied()),
            })
            .chain(opening)
            .chain(self.skills.iter().map(|(name, admitted)| dormant::Entry {
                label: format!("skill {name}"),
                about: admitted.entry.disclosure.clone(),
            }))
            .collect()
    }

    /// The tools outside the core, by namespace in the byte order of its
    /// name, each namespace's members in name order. The name leads the
    /// key because `Namespace` orders the city's own before any server's.
    fn dormant_namespaces(&self) -> BTreeMap<(&str, &Namespace), Vec<&str>> {
        let mut by_namespace: BTreeMap<(&str, &Namespace), Vec<&str>> = BTreeMap::new();
        for (name, filed) in self.tools.iter().filter(|(name, _)| !self.is_core(name)) {
            by_namespace
                .entry((filed.namespace.as_str(), &filed.namespace))
                .or_default()
                .push(name.as_str());
        }
        by_namespace
    }

    /// Whether `name` travels as a tool in this run's requests: it was
    /// admitted at `Core`, whatever mode the run sits in (runtime D25,
    /// D111). A catalog told no mode carries no tool, so nothing reaches
    /// a model before a mode was decided.
    fn is_core(&self, name: &str) -> bool {
        self.mode.is_some()
            && self
                .tools
                .get(name)
                .is_some_and(|filed| filed.exposure == Exposure::Core)
    }

    /// The call a `tool` `run` stands for, with the id the model gave it,
    /// so its result pairs with the `tool` call in the conversation and
    /// its tool passes its own doors (`crates/runtime/Spec.lean` §8-61).
    ///
    /// # Errors
    /// `E_INVALID_ARGS` when the call is not a `run`, when the arguments
    /// name no tool or carry no object, when they name `tool` itself,
    /// when the name is not a tool this run was admitted, and when the
    /// arguments do not fit the tool's input schema; the last carries the
    /// schema.
    pub fn resolve_run(&self, call: &ToolCall) -> Result<ToolCall, AxError> {
        fit::resolve(&self.tools, call)
    }

    /// The only source of `ChatRequest.tools`: the admitted tools in the
    /// union of the modes' cores, and no other.
    pub fn tool_defs(&self) -> Vec<ToolDef> {
        self.tools
            .iter()
            .filter(|(name, _)| self.is_core(name))
            .map(|(_, filed)| filed.def.clone())
            .collect()
    }

    /// The tool table this run is frozen with: the core array as
    /// [`Catalog::tool_defs`] lists it, serialised once, and one row per
    /// registered tool in name order, withheld ones included.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` for a definition that will not serialise, which a
    /// schema held as a `Payload` cannot be, and a length past `u64`.
    pub fn tool_table(&self) -> Result<ToolTable, AxError> {
        let array = serialised(&self.tool_defs())?;
        let mut rows = Vec::with_capacity(self.tools.len().saturating_add(self.withheld.len()));
        for (name, filed) in self.tools.iter().chain(self.withheld.iter()) {
            rows.push(PromptTool {
                name: name.clone(),
                namespace: filed.namespace.as_str().to_owned(),
                exposure: filed.exposure,
                from: filed.from,
                len: counted(serialised(&filed.def)?.len())?,
                hint: dormant::hint(&filed.def.description).to_owned(),
            });
        }
        rows.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(ToolTable {
            record: PromptTools {
                hash: B3Hash::digest(&array),
                len: counted(array.len())?,
                index: counted(dormant::index(&self.dormant_entries()).len())?,
                rows,
            },
            array,
        })
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

    /// Second-level disclosure. Tools expand to their schema through
    /// `tool`'s `detail`; skills and the mode expand through here.
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

/// One value as the tool table spells it.
fn serialised(value: &impl serde::Serialize) -> Result<Vec<u8>, AxError> {
    serde_json::to_vec(value).map_err(|err| {
        AxError::failure(
            AxCode::InvalidArgs,
            "record the tool table",
            err.to_string(),
        )
        .with_recovery("give the tool a schema that serialises as JSON")
    })
}

/// A byte count as the record holds it.
fn counted(len: usize) -> Result<u64, AxError> {
    u64::try_from(len).map_err(|_| {
        AxError::failure(
            AxCode::InvalidArgs,
            "record the tool table",
            "a length past u64",
        )
        .with_recovery("admit fewer or smaller tools: a table this long cannot be recorded")
    })
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
