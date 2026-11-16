// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use kernel::event::record::CheckpointCommitted;
use kernel::{Address, AxError, EventKind, Payload, RunId};

use crate::effect;

use super::super::{Agreed, Assignment, Given, RunWorker, Stamping, run_id_for};
use super::Site;

pub(in crate::worker) fn provenance(
    city: kernel::B3Hash,
    addr: &Address,
    run_id: RunId,
    chosen: storage::ModelChoice,
) -> storage::Provenance {
    storage::Provenance::new(run_id, addr.clone(), city, chosen)
}

fn filter_table(city_root: &Path, building: &Address) -> Result<runtime::FilterTable, AxError> {
    let layer = |file: PathBuf| -> Result<Option<String>, AxError> {
        match std::fs::read_to_string(&file) {
            Ok(text) => Ok(Some(text)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(AxError::failure(
                kernel::AxCode::StorageFatal,
                "read the filter table",
                format!("{}: {err}", file.display()),
            )
            .with_recovery("make the file readable, or remove it to fall back a layer")),
        }
    };

    let layout = kernel::layout::CityLayout::new(city_root);
    let city = layer(
        city_root
            .join(kernel::RESERVED_PREFIX)
            .join(kernel::layout::FILTERS_FILE),
    )?;
    let building = layer(layout.filters(building))?;
    runtime::FilterTable::resolve(city.as_deref(), building.as_deref())
}

impl Site {
    pub(in crate::worker) fn block_origin(&self) -> storage::BlockOrigin {
        storage::BlockOrigin {
            run: self.run_id,
            building: self.building.addr().clone(),
        }
    }

    pub(in crate::worker) fn provenance(
        &self,
        city: kernel::B3Hash,
        addr: &Address,
    ) -> storage::Provenance {
        let signed = provenance(
            city,
            addr,
            self.run_id,
            storage::ModelChoice {
                id: self.model.id.clone(),
                effort: self.config.effort,
            },
        );
        match self.predecessor {
            Some(predecessor) => signed.succeeding(predecessor),
            None => signed,
        }
    }
}

pub(in crate::worker) struct Placing<'a> {
    pub(in crate::worker) city_root: &'a Path,
    pub(in crate::worker) city: kernel::B3Hash,

    pub(in crate::worker) clock: &'a (dyn crate::Clock + Send + Sync),
}

impl Site {
    fn works_apart(&self, at: &super::super::Assignment) -> bool {
        match at.policy.landing {
            kernel::LandingPolicy::Experiment => true,
            kernel::LandingPolicy::Ordinary => self.rules.review(),
        }
    }

    pub(in crate::worker) fn place_tree<L: kernel::Ledger>(
        &mut self,
        at: &super::super::Assignment,
        placing: &Placing<'_>,
        lines: &mut Stamping<'_, L>,
    ) -> Result<(), AxError> {
        if !self.works_apart(at) {
            return Ok(());
        }
        let addr = &at.addr;

        let of = provenance(
            placing.city,
            addr,
            self.run_id,
            storage::ModelChoice {
                id: self.model.id.clone(),
                effort: self.config.effort,
            },
        );
        let claimed = lend_tree(
            &Lending {
                addr,
                building: &self.building,
                run_id: self.run_id,
                who: &self.who,
                of: &of,
                entropy_allow: self.rules.checkpoint_entropy_allow(),
            },
            placing,
            lines,
        )?;
        self.write_root = claimed.path().to_path_buf();
        self.branch = Some(claimed.name().as_str().to_owned());
        self.lease = Some(claimed);
        Ok(())
    }
}

pub(in crate::worker) struct Lending<'a> {
    pub(in crate::worker) addr: &'a Address,
    pub(in crate::worker) building: &'a city::Building,
    pub(in crate::worker) run_id: RunId,
    pub(in crate::worker) who: &'a str,
    pub(in crate::worker) of: &'a storage::Provenance,
    pub(in crate::worker) entropy_allow: &'a kernel::EntropyAllowlist,
}

pub(in crate::worker) fn lend_tree<L: kernel::Ledger>(
    lending: &Lending<'_>,
    placing: &Placing<'_>,
    lines: &mut Stamping<'_, L>,
) -> Result<storage::WorktreeLease, AxError> {
    let base = storage::Checkpoint::open(placing.city_root)
        .map_err(storage::StorageError::into_ax)?
        .with_entropy_allowlist(lending.entropy_allow.clone())
        .ensure_base(
            &[lending.addr.as_str().to_owned()],
            placing.clock.now()?,
            lending.of,
        )
        .map_err(storage::StorageError::into_ax)?;

    if let Some(base) = base {
        lines.record_for(
            lending.run_id,
            effect::Line {
                who: lending.who.to_owned(),
                addr: lending.addr.clone(),
                kind: EventKind::CheckpointCommitted,
                data: Payload::of(&CheckpointCommitted::Committed(base))?,
            },
        )?;
    }
    let claimed = storage::Worktrees::open(placing.city_root)
        .map_err(storage::StorageError::into_ax)?
        .claim(
            &tree_of(lending.addr)?,
            &super::tree_scope(lending.building),
        )
        .map_err(storage::StorageError::into_ax)?;
    lines.record_for(
        lending.run_id,
        effect::Line {
            who: lending.who.to_owned(),
            addr: lending.addr.clone(),
            kind: EventKind::WorktreeOpened,
            data: claimed
                .opened_payload()
                .map_err(storage::StorageError::into_ax)?,
        },
    )?;
    Ok(claimed)
}

impl Site {
    pub(in crate::worker) fn name_tree(
        &mut self,
        at: &super::super::Assignment,
    ) -> Result<(), AxError> {
        if self.works_apart(at) {
            self.branch = Some(tree_of(&at.addr)?.as_str().to_owned());
        }
        Ok(())
    }
}

impl RunWorker {
    pub(in crate::worker) fn stand_up(
        &mut self,
        agreed: Agreed,
        at: &Assignment,
        given: &Given,
        generation: kernel::FrozenGeneration,
    ) -> Result<Site, AxError> {
        let addr = &at.addr;
        let Agreed {
            building,
            rules,
            model,
            routing,
            provider,
            adapter,
            retries,
        } = agreed;

        let mut config = city::load_config(&self.city_root, addr)?;
        let selection = city::catalog_for_session(&self.city_root, addr, &rules, &mut config)?;

        let identity = city::Identity::load(&self.city_root, addr)?;
        let who = identity.who();

        let run_id = run_id_for(&given.job, addr, self.clock.now()?);

        let filters = filter_table(&self.city_root, building.addr())?;
        Ok(Site {
            building,
            rules,
            config,
            reading_room: selection.reading_room,
            model,
            routing,
            generation,
            provider,
            adapter: Some(adapter),
            identity,
            who,
            run_id,
            predecessor: at.predecessor(),
            lease: None,
            write_root: self.city_root.clone(),
            branch: None,
            filters,
            clock: runtime::ClockReading::default(),
            retries,
        })
    }
}

fn tree_of(addr: &Address) -> Result<storage::WorktreeName, AxError> {
    let digest: String = kernel::B3Hash::digest(addr.as_str().as_bytes())
        .to_string()
        .chars()
        .take(16)
        .collect();
    storage::WorktreeName::parse(&format!("room-{digest}")).map_err(storage::StorageError::into_ax)
}
