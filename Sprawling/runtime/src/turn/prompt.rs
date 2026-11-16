// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::Payload;

use crate::prefix::FrozenPrefix;

#[derive(Debug)]
pub struct RunPrompt<'r> {
    pub(super) prefix: &'r FrozenPrefix,
    pub(super) recorded: &'r mut PromptRecord,
}

impl<'r> RunPrompt<'r> {
    pub fn new(prefix: &'r FrozenPrefix, recorded: &'r mut PromptRecord) -> RunPrompt<'r> {
        RunPrompt { prefix, recorded }
    }
}

#[derive(Debug, Default)]
pub struct PromptRecord {
    written: Option<Payload>,
}

impl PromptRecord {
    pub(super) fn holds(&self, payload: &Payload) -> bool {
        self.written.as_ref() == Some(payload)
    }

    pub(super) fn remember(&mut self, payload: Payload) {
        self.written = Some(payload);
    }
}
