// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::PathBuf;

use kernel::{Address, AxError, ToolCall, ToolOutcome};
use runtime::clock::ClockStamp;
use runtime::offload::OffloadSite;
use runtime::{FilterTable, SieveHistory, SieveSite};
use storage::Cas;

pub struct SieveWorld {
    pub cas: Cas,
    pub root: PathBuf,
    pub address: Address,
    pub table: FilterTable,
    history: SieveHistory,
}

impl SieveWorld {
    pub fn open(root: &std::path::Path, table: FilterTable) -> Result<SieveWorld, AxError> {
        let cas = Cas::open(&root.join("cas")).map_err(storage::StorageError::into_ax)?;
        Ok(SieveWorld {
            cas,
            root: root.to_path_buf(),
            address: Address::parse("room")?,
            table,
            history: SieveHistory::default(),
        })
    }
}

pub(crate) fn package_exec(
    call: &ToolCall,
    outcome: &ToolOutcome,
    world: &mut SieveWorld,
    stamp: Option<ClockStamp>,
) -> Result<ToolOutcome, AxError> {
    runtime::package_exec(
        call,
        outcome.clone(),
        SieveSite {
            offload: OffloadSite {
                cas: &mut world.cas,
                city_root: &world.root,
                room: &world.address,
                origin: storage::BlockOrigin {
                    run: kernel::RunId::CITY,
                    building: world.address.clone(),
                },
            },
            table: &world.table,
            history: &mut world.history,
        },
        stamp,
    )
}
