// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The sieve's world in a scenario: a CAS and a room whose rest
//! directory holds the tee, one filter table, and the run's history. An `exec`
//! result goes through `runtime::package_exec`, the same door the city
//! uses, so the window a scenario replays is the window the product
//! showed.

use std::path::PathBuf;

use kernel::{Address, AxError, ToolCall, ToolOutcome};
use memory::Cas;
use runtime::clock::ClockStamp;
use runtime::offload::OffloadSite;
use runtime::{FilterTable, SieveHistory, SieveSite};

pub struct SieveWorld {
    pub cas: Cas,
    pub root: PathBuf,
    pub address: Address,
    pub table: FilterTable,
    history: SieveHistory,
}

impl SieveWorld {
    /// Opens the CAS under `root` and materializes rest files beside it.
    /// Opening the same root twice is the same world: the CAS is
    /// content-addressed and a rest file is written once.
    pub fn open(root: &std::path::Path, table: FilterTable) -> Result<SieveWorld, AxError> {
        let cas = Cas::open(&root.join("cas")).map_err(memory::MemoryError::into_ax)?;
        Ok(SieveWorld {
            cas,
            root: root.to_path_buf(),
            address: Address::parse("room")?,
            table,
            history: SieveHistory::default(),
        })
    }
}

/// One `exec` outcome, sieved and packaged through the product's door.
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
                origin: memory::BlockOrigin {
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
