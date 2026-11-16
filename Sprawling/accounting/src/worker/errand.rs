// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::mpsc;

use kernel::{AxCode, AxError};

use super::RunWorker;
use super::credentials::endpoints::{Attaching, Probed};
use super::keeping_warm::Renewed;
use super::relay::Wake;

pub(crate) enum Answer {

    Renewed(Renewed),

    Probed(Probed),

    Attached(Box<Attaching>),
}

pub(in crate::worker) struct Errands {
    bell: mpsc::Sender<Wake>,

    out: u32,
}

impl Errands {
    pub(in crate::worker) fn open(bell: mpsc::Sender<Wake>) -> Errands {
        Errands { bell, out: 0 }
    }

    pub(in crate::worker) fn send(
        &mut self,
        request: impl FnOnce() -> Answer + Send + 'static,
    ) -> Result<(), AxError> {
        let bell = self.bell.clone();
        std::thread::Builder::new()
            .name("sprawling-errand".to_owned())
            .spawn(move || {

                drop(bell.send(Wake::Answered(Box::new(request()))));
            })
            .map_err(|source| {
                AxError::failure(
                    AxCode::StorageFatal,
                    "send a provider request off the accounting thread",
                    source.to_string(),
                )
                .with_recovery("check process thread limits, then ask again")
            })?;
        self.out = self.out.saturating_add(1);
        Ok(())
    }

    pub(in crate::worker) fn answered(&mut self) {
        self.out = self.out.saturating_sub(1);
    }

    pub(in crate::worker) fn out(&self) -> bool {
        self.out > 0
    }
}

impl RunWorker {

    pub(in crate::worker) fn take_answer(&mut self, answer: Answer) {
        match answer {
            Answer::Renewed(renewed) => self.record_renewal(renewed),
            Answer::Probed(probed) => self.record_probe(probed),
            Answer::Attached(attaching) => self.finish_attach(*attaching),
        }
    }
}
