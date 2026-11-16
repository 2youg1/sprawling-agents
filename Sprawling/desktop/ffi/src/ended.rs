// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use winsafe::co;

use crate::step::Step;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    At { step: Step, code: co::ERROR },

    Unspelled(u32),
}

#[must_use]
pub fn read(raw: u32) -> Option<Step> {
    Step::ALL.into_iter().find(|step| step.number() == raw)
}

pub(crate) fn step(raw: u32) -> Result<Step, Failure> {
    read(raw).ok_or(Failure::Unspelled(raw))
}

pub(crate) fn finished(raw: u32, code: co::ERROR) -> Result<(), Failure> {
    let ended = step(raw)?;
    if ended == Step::Finished {
        return Ok(());
    }
    Err(Failure::At { step: ended, code })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn every_step_reads_back_from_its_number_and_no_other_number_does() {
        let read_back: Vec<Option<Step>> =
            Step::ALL.iter().map(|step| read(step.number())).collect();
        assert_eq!(
            read_back,
            Step::ALL.iter().copied().map(Some).collect::<Vec<_>>()
        );
        let past = u32::try_from(Step::ALL.len()).unwrap();
        assert_eq!(step(past), Err(Failure::Unspelled(past)));
    }
}
