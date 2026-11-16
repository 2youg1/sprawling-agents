// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one call into the leaf ended with, read rather than trusted: a
//! number from across the boundary becomes a [`Step`] only by matching
//! one, so a leaf and a library from two builds meet a refusal here
//! instead of an enum value nobody defined.

use winsafe::co;

use crate::step::Step;

/// Why an operation of the leaf did not finish.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// The leaf stopped at `step`. `code` is the calling thread's last
    /// error at that moment, which is the machine's reason for the steps
    /// that stop at a Win32 call; the steps that stop at a rule of the
    /// leaf (`NoWindow`, `Measuring`, `ShortRows`, `EmptyBlock`,
    /// `NoRoom`) carry no reason of the machine's in it.
    At { step: Step, code: co::ERROR },
    /// The leaf answered a number no step has: it was built from another
    /// `step.rs` than this library, which `build.rs` exists to prevent.
    Unspelled(u32),
}

/// The step a number names, if one does.
#[must_use]
pub fn read(raw: u32) -> Option<Step> {
    Step::ALL.into_iter().find(|step| step.number() == raw)
}

/// The step a number names, or the failure that a number naming none is.
pub(crate) fn step(raw: u32) -> Result<Step, Failure> {
    read(raw).ok_or(Failure::Unspelled(raw))
}

/// A call that finished, or the failure at the step it stopped at.
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

    /// Every step crosses the boundary as a number that reads back as
    /// itself, and a number past the last step reads as no step.
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
