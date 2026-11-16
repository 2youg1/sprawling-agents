// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError, ContentBlock, ModelUsage, StopReason};

use super::Entry;

#[derive(Debug, Clone, PartialEq)]
pub struct TurnReport {
    pub(super) redacted: u32,
    pub(super) shortened: std::collections::BTreeSet<String>,
    pub(super) model_returned: Entry,
    pub(super) calls_made: usize,
    pub(super) assistant: Vec<ContentBlock>,
    pub(super) wave_results: Vec<ContentBlock>,
    pub(super) usage: Option<ModelUsage>,
    pub(super) stop: Option<StopReason>,
}

impl TurnReport {
    pub fn shortened(&self) -> &std::collections::BTreeSet<String> {
        &self.shortened
    }

    pub fn usage(&self) -> Option<&ModelUsage> {
        self.usage.as_ref()
    }

    pub fn redacted(&self) -> u32 {
        self.redacted
    }

    pub fn model_returned(&self) -> Entry {
        self.model_returned
    }

    pub fn calls_made(&self) -> usize {
        self.calls_made
    }

    pub fn assistant(&self) -> &[ContentBlock] {
        &self.assistant
    }

    pub fn wave_results(&self) -> &[ContentBlock] {
        &self.wave_results
    }

    pub fn stop(&self) -> Option<StopReason> {
        self.stop
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallShape {
    pub model: String,
    pub effort: Option<kernel::Effort>,

    pub generation: kernel::FrozenGeneration,

    pub context_tokens: u64,
}

impl CallShape {
    pub fn verified_against(&self, frozen: &CallShape) -> Result<(), AxError> {
        if self.model != frozen.model {
            return Err(shape_moved(format!(
                "the model changed from `{}` to `{}`",
                frozen.model, self.model
            )));
        }
        if self.effort != frozen.effort {
            return Err(shape_moved(format!(
                "the effort changed from {:?} to {:?}",
                frozen.effort, self.effort
            )));
        }
        if self.generation.max_tokens != frozen.generation.max_tokens {
            return Err(shape_moved("the output ceiling changed".to_owned()));
        }
        if self.generation.settings != frozen.generation.settings {
            return Err(shape_moved("the sampling settings changed".to_owned()));
        }
        if self.generation.overrides != frozen.generation.overrides {
            return Err(shape_moved("the raw request overrides changed".to_owned()));
        }
        Ok(())
    }
}

fn shape_moved(subject: String) -> AxError {
    AxError::failure(AxCode::ConfigInvalid, "dispatch a turn", subject).with_recovery(format!(
        "put the field the subject names back to what this session froze, or \
         {}: the model, its ceiling and its effort freeze with the session, \
         and a mid-run change would invalidate the prefix the run is paying to cache",
        crate::prefix::ANOTHER_ADDRESS
    ))
}
