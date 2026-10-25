// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A whole-document step on one prompt document, and the line that
//! records it (`crates/accounting/Spec.lean` §8-22,
//! `crates/wire/Spec.lean` §8-98).
//!
//! Which documents take which step, and the version check, are the
//! city's (`city::put_prompt`); this module turns the frame into that
//! call and writes the line.

use kernel::event::record::GovernedDocumentWritten;
use kernel::{Address, AxCode, AxError, B3Hash, EventKind, Payload};

use super::super::RunWorker;

impl RunWorker {
    /// Takes `step` on the prompt document at `at`, only while it still
    /// holds the version `base` names, and records what it left.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` for an address that is not on the city's list of
    /// prompt documents; whatever `city::put_prompt` refuses (a step the
    /// document does not take, a file that moved since `base`, a file that
    /// will not read, write or go); and a history that will not take the
    /// line.
    pub(in crate::worker) fn put_prompt(
        &mut self,
        at: &Address,
        base: B3Hash,
        step: wire::PromptStep,
    ) -> Result<(), AxError> {
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
        let written = city::put_prompt(&self.city_root, &doc, base, step_of(step))?;
        // The three in the city's reserved subtree are named by their file
        // alone; every other prompt document needs its address.
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

/// The city's spelling of the step the frame names.
fn step_of(step: wire::PromptStep) -> city::PromptStep {
    match step {
        wire::PromptStep::Own => city::PromptStep::Own,
        wire::PromptStep::Inherit => city::PromptStep::Inherit,
        wire::PromptStep::Restore => city::PromptStep::Restore,
    }
}
