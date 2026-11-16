// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxError, EventRecord};

use super::holding::Views;

impl Views {
    pub(super) fn fold_ruling(&mut self, record: &EventRecord) -> Result<(), AxError> {
        let ruled = record
            .data()
            .read::<kernel::event::record::ApprovalResolved>()?;
        self.decided.push(wire::Decision {
            item: ruled.id.as_str().to_owned(),
            verdict: ruled.verdict,
            cluster: ruled.cluster,
            at: record.t(),
        });
        Ok(())
    }
}
