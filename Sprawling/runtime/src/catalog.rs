// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

pub use dormant::hint;
pub use guide::{FIND_HITS, LIST_CEILING};
pub use namespace::Namespace;

pub const DORMANT_INDEX_CEILING: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogEntry {
    pub name: String,
    pub disclosure: String,
    pub expansion: String,

    pub hash: Option<B3Hash>,

    pub document: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expansion {
    Skill { manifest: B3Hash },

    Said { text: String },
}

#[derive(Debug, Default)]
pub struct Catalog {
    tools: BTreeMap<String, Filed>,

    withheld: BTreeMap<String, Filed>,
    skills: BTreeMap<String, Admitted>,

    withheld_skills: BTreeMap<String, CatalogEntry>,
    mode: Option<Mode>,
    exposure: CatalogExposure,
}

#[derive(Debug)]
struct Filed {
    def: ToolDef,
    namespace: Namespace,
    exposure: Exposure,
    from: ExposureFrom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolTable {
    pub array: Vec<u8>,

    pub definitions: Vec<Vec<u8>>,
    pub record: PromptTools,
}

#[derive(Debug)]
struct Admitted {
    entry: CatalogEntry,
    exposure: Exposure,
}

impl Catalog {
    pub fn new() -> Catalog {
        Catalog::default()
    }

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

    pub fn admit_skill(&mut self, entry: CatalogEntry) -> Result<Exposure, AxError> {
        self.admit(entry)
    }

    fn admit(&mut self, entry: CatalogEntry) -> Result<Exposure, AxError> {
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
        if self.skills.contains_key(&entry.name) || self.withheld_skills.contains_key(&entry.name) {
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
        let (exposure, _) = self
            .exposure
            .stated_skill(&entry.name)
            .unwrap_or((Exposure::Dormant, ExposureFrom::Default));
        if exposure == Exposure::Off {
            self.withheld_skills.insert(entry.name.clone(), entry);
        } else {
            self.skills
                .insert(entry.name.clone(), Admitted { entry, exposure });
        }
        Ok(exposure)
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = Some(mode);
    }

    pub fn set_exposure(&mut self, stated: CatalogExposure) {
        self.exposure = stated;
    }

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

        let dev = crate::mode::dev_entry();
        out.push_str("- ");
        out.push_str(&dev.name);
        out.push_str(": ");
        out.push_str(&dev.disclosure);
        out.push('\n');
        out.push_str(&dormant::index(&self.dormant_entries()));
        for (name, admitted) in self
            .skills
            .iter()
            .filter(|(_, admitted)| admitted.exposure == Exposure::Core)
        {
            out.push_str("\n## skill ");
            out.push_str(name);
            out.push_str("\n\n");
            out.push_str(&admitted.entry.document);
            if !admitted.entry.document.ends_with('\n') {
                out.push('\n');
            }
        }
        out
    }

    fn dormant_entries(&self) -> Vec<dormant::Entry> {
        let skill = Namespace::Own(ReservedNamespace::Skill);
        let mut namespaces = self.dormant_namespaces();
        let filed_as_skill = namespaces
            .remove(&(skill.as_str(), &skill))
            .unwrap_or_default();
        let dormant_skills = self
            .skills
            .values()
            .filter(|admitted| admitted.exposure == Exposure::Dormant)
            .count();
        let skills = filed_as_skill.len().saturating_add(dormant_skills);
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
            .chain(
                self.skills
                    .iter()
                    .filter(|(_, admitted)| admitted.exposure == Exposure::Dormant)
                    .map(|(name, admitted)| dormant::Entry {
                        label: format!("skill {name}"),
                        about: admitted.entry.disclosure.clone(),
                    }),
            )
            .collect()
    }

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

    fn is_core(&self, name: &str) -> bool {
        self.mode.is_some()
            && self
                .tools
                .get(name)
                .is_some_and(|filed| filed.exposure == Exposure::Core)
    }

    pub fn resolve_run(&self, call: &ToolCall) -> Result<ToolCall, AxError> {
        fit::resolve(&self.tools, call)
    }

    pub fn tool_defs(&self) -> Vec<ToolDef> {
        self.tools
            .iter()
            .filter(|(name, _)| self.is_core(name))
            .map(|(_, filed)| filed.def.clone())
            .collect()
    }

    pub fn tool_table(&self) -> Result<ToolTable, AxError> {
        let array = serialised(&self.tool_defs())?;
        let mut rows = Vec::with_capacity(self.tools.len().saturating_add(self.withheld.len()));
        let mut definitions = Vec::with_capacity(rows.capacity());
        for (name, filed) in self.tools.iter().chain(self.withheld.iter()) {
            definitions.push(filed.def.pin_bytes()?);
            rows.push(PromptTool {
                name: name.clone(),
                namespace: filed.namespace.as_str().to_owned(),
                exposure: filed.exposure,
                from: filed.from,
                digest: Some(filed.def.digest()?),
                len: counted(serialised(&filed.def)?.len())?,
                hint: dormant::hint(&filed.def.description).to_owned(),
            });
        }
        rows.sort_by(|left, right| left.name.cmp(&right.name));
        let reading = self
            .skills
            .values()
            .filter(|admitted| admitted.exposure == Exposure::Core)
            .map(|admitted| admitted.entry.document.len())
            .sum::<usize>();
        Ok(ToolTable {
            record: PromptTools {
                hash: B3Hash::digest(&array),
                len: counted(array.len())?,
                index: counted(dormant::index(&self.dormant_entries()).len())?,
                reading: counted(reading)?,
                always: counted_u32(self.tool_defs().len())?,
                dormant: counted_u32(
                    self.tools
                        .values()
                        .filter(|filed| filed.exposure == Exposure::Dormant)
                        .count(),
                )?,
                skills: counted_u32(self.skills.len())?,
                rows,
            },
            array,
            definitions,
        })
    }

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

    pub fn expand(&self, name: &str) -> Option<Expansion> {
        if let Some(Admitted { entry, .. }) = self.skills.get(name) {
            let manifest = entry.expansion.parse::<B3Hash>().ok()?;
            return Some(Expansion::Skill { manifest });
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

fn counted_u32(len: usize) -> Result<u32, AxError> {
    u32::try_from(len).map_err(|_| {
        AxError::failure(
            AxCode::InvalidArgs,
            "record the tool table",
            "a capability count past u32",
        )
        .with_recovery("admit fewer capabilities: this table cannot be recorded")
    })
}

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
