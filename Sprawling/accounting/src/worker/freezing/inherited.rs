// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, AxError, ChatMessage, RunId};

use super::super::{Assignment, RunWorker};

#[derive(Debug)]
pub(in crate::worker) struct InheritedContext {
    pub(in crate::worker) messages: Vec<ChatMessage>,
    pub(in crate::worker) delivered: std::collections::BTreeSet<Address>,
}

impl RunWorker {

    pub(in crate::worker) fn inherited(
        &mut self,
        at: &Assignment,
        run: RunId,
    ) -> Result<InheritedContext, AxError> {
        let Some(origin) = at.origin else {
            return Ok(InheritedContext {
                messages: Vec::new(),
                delivered: std::collections::BTreeSet::new(),
            });
        };
        let dir = kernel::layout::CityLayout::new(&self.city_root).ledger();
        self.index
            .refresh(&dir)
            .map_err(storage::StorageError::into_ax)?;
        let rebuilt = runtime::fork::inherited_indexed(&self.index, &dir, origin.at_seq)?;
        self.note_lineage(
            &at.addr,
            run,
            kernel::Origin {
                run: origin.run,
                at_seq: rebuilt.at,
            },
        )?;
        for warning in rebuilt.rule_warnings {
            self.note(
                runtime::diagnostics::Level::Refuse,
                "runtime::fork",
                &warning,
            );
        }
        Ok(InheritedContext {
            messages: rebuilt.messages,
            delivered: rebuilt.delivered,
        })
    }
}
