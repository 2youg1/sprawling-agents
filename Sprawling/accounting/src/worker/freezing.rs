// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::Locator;
use kernel::{Address, AxCode, AxError};

use crate::prompt_chain::{self, Convention, Reading, addressed};
use runtime::prefix::{FrozenPrefix, FrozenSegment, SegmentSlot, SegmentSource};
use runtime::run::RunPlan;

use super::{Assignment, Given, Site, Workbench, held, prompt_segment};
use model_note::model_note;
use run_slot::{Predecessor, run_segment};

pub(super) struct Assembled {
    pub(super) bytes: Vec<u8>,
    pub(super) sources: Vec<SegmentSource>,
}

impl Assembled {
    pub(super) fn of_nothing(bytes: Vec<u8>) -> Assembled {
        Assembled {
            bytes,
            sources: Vec::new(),
        }
    }

    pub(super) fn of_one(addr: Address, bytes: Vec<u8>) -> Assembled {
        Assembled {
            sources: vec![SegmentSource::whole(addr, &bytes)],
            bytes,
        }
    }

    fn extend(&mut self, at: Address, bytes: &[u8]) {
        if !self.bytes.is_empty() {
            self.bytes.push(NEWLINE);
            self.bytes.push(NEWLINE);
        }
        self.bytes.extend_from_slice(bytes);
        self.sources.push(SegmentSource::whole(at, bytes));
    }

    fn freeze(self, slot: SegmentSlot) -> FrozenSegment {
        FrozenSegment::assembled(slot, self.bytes, self.sources)
    }
}

pub(super) const NEWLINE: u8 = 10;

pub(super) fn building_segment(city_root: &Path, building: &Address) -> Result<Assembled, AxError> {
    let mut out = Assembled::of_nothing(building.as_str().as_bytes().to_vec());
    let rules_at = city::rules_path(city_root, building);
    if let Ok(rules) = std::fs::read(&rules_at) {
        out.extend(addressed(city_root, &rules_at)?, &rules);
    }
    out.extend_conventions(
        city_root,
        building,
        prompt_chain::building_conventions(city_root, building),
    )?;
    Ok(out)
}

impl Assembled {
    fn extend_conventions(
        &mut self,
        city_root: &Path,
        building: &Address,
        conventions: impl IntoIterator<Item = Convention>,
    ) -> Result<(), AxError> {
        for Convention {
            path,
            heading,
            whose,
        } in conventions
        {
            let text = match prompt_chain::read_convention(city_root, building, &path)? {
                Reading::Sent(text) => text,
                Reading::Absent | Reading::Withheld(_) => continue,
            };
            let mut block = format!(
                "## {heading}\n\n{whose} Where this and the building's rules above disagree, \
                 the rules decide: they are what the city enforces.\n\n"
            )
            .into_bytes();
            block.extend_from_slice(&text);
            self.extend(addressed(city_root, &path)?, &block);
        }
        Ok(())
    }
}

pub(super) struct Freezing<'a> {
    pub(super) city_root: &'a Path,

    pub(super) cas: &'a mut storage::Cas,

    pub(super) inherited: Vec<kernel::ChatMessage>,

    pub(super) carried_from: Option<kernel::RunId>,
}

impl Freezing<'_> {
    fn intern_prefix(&mut self, prefix: &FrozenPrefix) -> Result<(), AxError> {
        for segment in prefix.segments() {
            self.cas
                .put(segment.bytes())
                .map_err(storage::StorageError::into_ax)?;
        }
        Ok(())
    }

    fn assemble_run_slot(
        &self,
        site: &Site,
        at: &Assignment,
        brief: &city::RunBrief,
    ) -> Result<Assembled, AxError> {
        let mut slot = Assembled::of_nothing(run_segment(
            self.city_root,
            brief,
            Predecessor {
                room: &at.addr,
                run: at.predecessor().or(self.carried_from),
            },
        )?);
        if let Some(note) = model_note(
            self.city_root,
            site.building.addr(),
            &site.provider,
            &site.model.id,
        )? {
            slot.extend(note.at, &note.bytes);
        }
        Ok(slot)
    }

    pub(super) fn freeze_plan(
        mut self,
        site: &Site,
        workbench: &mut Workbench,
        at: &Assignment,
        given: Given,
    ) -> Result<(RunPlan, runtime::handoff::Handoff), AxError> {
        let addr = &at.addr;
        let Given {
            brief,
            task,
            goal,
            job,
        } = given;
        let tools = held(&workbench.catalog, "read the catalog")?.tool_defs();

        let names = naming::session_naming(self.city_root, self.cas, addr)?;

        let mut resident = format!("{}\n\n", addr.as_str()).into_bytes();
        resident.extend_from_slice(&names.called(addr));
        resident.extend_from_slice(&site.identity.segment_bytes());
        resident.push(NEWLINE);
        resident.extend_from_slice(
            held(&workbench.catalog, "read the catalog")?
                .render()
                .as_bytes(),
        );
        let mut resident = Assembled::of_nothing(resident);
        resident.extend_conventions(
            self.city_root,
            site.building.addr(),
            prompt_chain::room_conventions(self.city_root, addr, site.building.addr())?,
        )?;
        let prefix = FrozenPrefix::assemble(
            names
                .city_slot(self.city_root, site.building.addr())?
                .freeze(SegmentSlot::City),
            building_segment(self.city_root, site.building.addr())?.freeze(SegmentSlot::Building),
            resident.freeze(SegmentSlot::Resident),
            self.assemble_run_slot(site, at, &brief)?
                .freeze(SegmentSlot::Run),
        )?;

        let table = held(&workbench.catalog, "read the catalog")?.tool_table()?;
        self.cas
            .put(&table.array)
            .map_err(storage::StorageError::into_ax)?;
        for definition in &table.definitions {
            self.cas
                .put(definition)
                .map_err(storage::StorageError::into_ax)?;
        }
        let prefix = prefix.with_tools(table.record);
        self.intern_prefix(&prefix)?;

        let plan = RunPlan {
            run: site.run_id,
            who: site.who.clone(),
            addr: addr.clone(),
            task,
            goal,

            opening: match &brief {
                city::RunBrief::Job { .. } => runtime::Opening::FromJob,
                city::RunBrief::Principal => runtime::Opening::WithPerson,
            },
            job: job.clone(),
            parent: at.parent,
            predecessor: at.predecessor(),
            dispatched_by: at.dispatched_by.clone(),
            routing: Some(site.routing.clone()),
            run_policy: workbench.take_policy()?,
            inherited: std::mem::take(&mut self.inherited),
            shape: runtime::turn::CallShape {
                model: site.model.id.clone(),

                effort: site.config.effort,
                generation: site.generation.clone(),

                context_tokens: site.model.context_tokens,
            },
            second_threshold: site.config.second_threshold,
            context_lock: site.rules.context_lock(),
            succession: std::sync::Arc::clone(&workbench.succession),
            context: workbench.context.clone(),
            prefix,
            policy: site.rules.policy().clone(),
            tools,

            skills: held(&workbench.catalog, "read the catalog")?.skill_pins(),
            retries: site.retries,
            naming: Some(names.version),
        };

        let mut must_read = Vec::new();

        let origin = storage::BlockOrigin {
            run: site.run_id,
            building: addr.clone(),
        };
        for norm in city::norms(self.city_root, addr)? {
            let bytes = std::fs::read(&norm).map_err(|err| {
                AxError::failure(
                    AxCode::StorageFatal,
                    "read a norm document for the must-read list",
                    format!("{}: {err}", norm.display()),
                )
                .with_recovery("fix the file's permissions, or remove it from the building")
            })?;
            let hash = self
                .cas
                .put_for(&bytes, &origin)
                .map_err(storage::StorageError::into_ax)?;
            must_read.push(Locator::cas(hash));
        }
        must_read.push(job);
        let handoff = self.frozen_handoff(&plan, must_read)?;
        Ok((plan, handoff))
    }
}

mod inherited;
pub(in crate::worker) use inherited::InheritedContext;
mod model_note;
mod naming;
mod run_slot;

mod frozen_handoff;

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
