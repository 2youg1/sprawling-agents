// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::AxError;

use super::holding::Views;
use super::snapshot::start::start_audited;

pub fn ask(city_root: &Path, query: &wire::Query) -> Result<wire::Answer, AxError> {
    Views::rebuild(&kernel::layout::CityLayout::new(city_root).ledger())?
        .prepare(query)
        .answer()
}

impl Views {
    pub fn rebuild(ledger_dir: &Path) -> Result<Views, AxError> {
        start_audited::<Views>(ledger_dir).map(|audited| audited.started.folded)
    }
}
