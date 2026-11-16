// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use kernel::Address;

use super::{Prepared, unavailable};

pub struct LiveAsk {
    pub(crate) city_root: PathBuf,
    pub(crate) city: Option<Address>,
    pub(crate) vault: Option<Arc<Mutex<gateway::Custodian>>>,
}

impl Prepared {

    pub(super) fn preferences_answer() -> wire::Answer {
        match crate::person::read() {
            Ok(settled) => wire::Answer::Preferences(Box::new(settled)),
            Err(_) => unavailable("Preferences".to_owned()),
        }
    }

    pub(super) fn guide_answer(city_root: PathBuf) -> wire::Answer {
        match crate::guide::read(&city_root) {
            Ok(progress) => wire::Answer::Guide(progress),
            Err(_) => unavailable("Guide".to_owned()),
        }
    }

    pub(super) fn release_answer(newest: Option<fn() -> wire::ReleaseAnswer>) -> wire::Answer {
        match newest {
            Some(newest) => wire::Answer::Release(Box::new(newest())),
            None => unavailable("NewestRelease".to_owned()),
        }
    }

    pub fn answer(self) -> Result<wire::Answer, kernel::AxError> {
        let Prepared::Refused(refusal) = self else {
            return Ok(self.finish());
        };
        Err(refusal)
    }

    pub(super) fn upstream_answer(
        ask: Option<fn(&str) -> wire::DoctorUpstream>,
        item: String,
    ) -> wire::Answer {
        match ask {
            Some(newest) => wire::Answer::Upstream(Box::new(newest(&item))),
            None => unavailable(format!("UpstreamVersion({item})")),
        }
    }
}
