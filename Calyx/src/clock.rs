// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError, TimeMs};

pub struct Wall;

impl accounting::Clock for Wall {
    fn now(&self) -> Result<TimeMs, AxError> {
        #[expect(
            clippy::disallowed_methods,
            reason = "Calyx binary is its own assembly root for the Mei face"
        )]
        let elapsed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|err| {
                AxError::failure(AxCode::ConfigInvalid, "sample wall clock", err.to_string())
                    .with_recovery("fix the system clock; it reads before the unix epoch")
            })?;
        let millis = u64::try_from(elapsed.as_millis()).map_err(|_| {
            AxError::failure(
                AxCode::ConfigInvalid,
                "sample wall clock",
                "beyond u64 millis",
            )
            .with_recovery(
                "set this machine's clock to the present day; it reads more than half a \
                 billion years after the unix epoch",
            )
        })?;
        Ok(TimeMs::new(millis))
    }
}
