// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#[cfg(test)]
use std::path::Path;

#[cfg(test)]
use kernel::degradation::VolumeSpace;
use kernel::degradation::{self, Recovery};
use kernel::{AxCode, AxError};

use super::super::RunWorker;

impl RunWorker {

    pub(in crate::worker) fn room_for_new_work(&self) -> Result<(), AxError> {
        (self.read_volume)(&self.city_root)
            .map_or(Ok(()), degradation::admit_work)
            .map_err(|low| {
                AxError::failure(
                    AxCode::BackpressureShed,
                    "take on new work while the city's volume is below its free-space floor",
                    self.city_root.display().to_string(),
                )
                .with_recovery(match low.recovery() {
                    Recovery::FreeDiskSpace { at_least_bytes } => {
                        format!("free at least {at_least_bytes} bytes on the city's volume")
                    }
                    Recovery::ReduceDiskLoad => {
                        "reduce the other writes to the city's disk".to_owned()
                    }
                    Recovery::FreeMemory => "close programs to free memory".to_owned(),
                    Recovery::ReduceCpuLoad => "stop other work that keeps the CPU busy".to_owned(),
                })
            })
    }

    #[cfg(test)]
    pub(in crate::worker) fn read_volume_with(&mut self, read: fn(&Path) -> Option<VolumeSpace>) {
        self.read_volume = read;
    }
}
