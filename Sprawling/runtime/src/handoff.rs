// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::HandoffWritten;
use kernel::{AxCode, AxError, Locator, Payload, RunId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handoff {
    must_read: Vec<Locator>,
    overview: String,
    progress: String,
    context: String,
    next_step: String,
}

impl Handoff {

    pub fn new(
        must_read: Vec<Locator>,
        overview: String,
        progress: String,
        context: String,
        next_step: String,
    ) -> Result<Handoff, AxError> {
        if must_read.is_empty() {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "construct handoff",
                "empty must-read list",
            )
            .with_recovery(
                "list the locators the successor must read before working; \
                 norms are machine-filled, add only this run's specifics",
            ));
        }
        Ok(Handoff {
            must_read,
            overview,
            progress,
            context,
            next_step,
        })
    }

    pub fn must_read(&self) -> &[Locator] {
        &self.must_read
    }

    pub fn overview(&self) -> &str {
        &self.overview
    }

    pub fn progress(&self) -> &str {
        &self.progress
    }

    pub fn context(&self) -> &str {
        &self.context
    }

    pub fn next_step(&self) -> &str {
        &self.next_step
    }

    pub fn payload(&self) -> Result<Payload, AxError> {
        Payload::of(&HandoffWritten {
            must_read: self.must_read.clone(),
            overview: self.overview.clone(),
            progress: self.progress.clone(),
            context: self.context.clone(),
            next_step: self.next_step.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumeSeed {
    pub run: RunId,
    pub must_read: Vec<Locator>,
}

pub fn resume(handoff: &Handoff, new_run: RunId) -> ResumeSeed {
    ResumeSeed {
        run: new_run,
        must_read: handoff.must_read().to_vec(),
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    fn locator() -> Locator {
        Locator::parse(&format!("cas:b3-{}", "ab".repeat(32))).unwrap()
    }

    #[test]
    fn empty_must_read_cannot_be_constructed() {
        let err = Handoff::new(vec![], "o".into(), "p".into(), "c".into(), "n".into()).unwrap_err();
        assert_eq!(err.code(), &AxCode::InvalidArgs);
    }

    #[test]
    fn payload_carries_five_sections() {
        let handoff = Handoff::new(
            vec![locator()],
            "overview".into(),
            "progress".into(),
            "context".into(),
            "next".into(),
        )
        .unwrap();
        let json = serde_json::to_value(handoff.payload().unwrap()).unwrap();
        for key in ["must_read", "overview", "progress", "context", "next_step"] {
            assert!(json.get(key).is_some(), "missing section {key}");
        }
        assert_eq!(json["must_read"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn resume_mints_the_supplied_identity_and_carries_must_read() {
        let handoff = Handoff::new(
            vec![locator()],
            String::new(),
            String::new(),
            String::new(),
            "user-specified next action".into(),
        )
        .unwrap();
        let new_run = RunId::parse("0198f6a2-7c4a-7bbb-9d1e-000000000002").unwrap();
        let seed = resume(&handoff, new_run);
        assert_eq!(seed.run, new_run);
        assert_eq!(seed.must_read.len(), 1);
    }
}
