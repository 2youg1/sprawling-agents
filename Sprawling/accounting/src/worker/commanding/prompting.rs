// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::GovernedDocumentWritten;
use kernel::{AxCode, AxError, EventKind, Payload};

use super::super::RunWorker;

impl RunWorker {

    pub(in crate::worker) fn put_prompt(
        &mut self,
        write: &wire::PromptWrite,
    ) -> Result<(), AxError> {
        let at = &write.at;
        let doc = city::PromptDocument::of(at).ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "change a prompt document",
                at.as_str().to_owned(),
            )
            .with_recovery(
                "name City.md at the city root, MAYOR.md or CLERK.md in the city's reserved \
                 subtree, a building's own City.md, or a model's note",
            )
        })?;
        let written = city::put_prompt(&self.city_root, &doc, write.base, step_of(write.step))?;

        let (naming, place) = match doc {
            city::PromptDocument::Shipped(city::Shipped::Mayor) => (self.naming_version()?, None),
            city::PromptDocument::Shipped(city::Shipped::Clerk) => (None, None),
            city::PromptDocument::Shipped(city::Shipped::City)
            | city::PromptDocument::BuildingCity { .. }
            | city::PromptDocument::ModelNote { .. } => (None, Some(at.clone())),
        };
        self.record(
            EventKind::GovernedDocumentWritten,
            Payload::of(&GovernedDocumentWritten {
                which: at
                    .as_str()
                    .rsplit_once('/')
                    .map_or(at.as_str(), |(_, file)| file)
                    .to_owned(),
                bytes: written.bytes,
                naming,
                at: place,
                removed: written.removed,
            })?,
        )
    }
}

fn step_of(step: wire::PromptStep) -> city::PromptStep {
    match step {
        wire::PromptStep::Own => city::PromptStep::Own,
        wire::PromptStep::Inherit => city::PromptStep::Inherit,
        wire::PromptStep::Restore => city::PromptStep::Restore,
    }
}
