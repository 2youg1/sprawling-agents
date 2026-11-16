// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::{
    CancelReceived, HarnessAnswered, HarnessReported, HarnessStop, SecretEgressBlocked,
};
use kernel::{AxError, Completion, EventKind, EventRef, Evidence, Ledger, Payload, TimeMs};

use crate::handoff::Handoff;

use super::charter::{Charter, Verdict};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cut {

    Halt,

    Deadline,
}

pub struct Conclusion<'c> {

    pub committed: Payload,
    pub answered: HarnessAnswered,
    pub handoff: &'c Handoff,
}

pub struct HarnessRun<'a> {
    charter: Charter<'a>,

    cut: Option<Cut>,
}

impl<'a> HarnessRun<'a> {

    pub fn open(
        charter: Charter<'a>,
        ledger: &mut dyn Ledger,
        now: &mut dyn FnMut() -> Result<TimeMs, AxError>,
    ) -> Result<HarnessRun<'a>, AxError> {
        charter.open(ledger, now)?;
        Ok(HarnessRun { charter, cut: None })
    }

    pub fn report(
        &self,
        ledger: &mut dyn Ledger,
        reported: &HarnessReported,
        t: TimeMs,
    ) -> Result<(), AxError> {
        ledger.append(
            self.charter
                .line(EventKind::HarnessReported, Payload::of(reported)?, t),
        )?;
        Ok(())
    }

    pub fn cancel(&mut self, ledger: &mut dyn Ledger, cut: Cut, t: TimeMs) -> Result<(), AxError> {
        if self.cut.is_some() {
            return Ok(());
        }
        ledger.append(self.charter.line(
            EventKind::CancelReceived,
            Payload::of(&CancelReceived {})?,
            t,
        ))?;
        self.cut = Some(cut);
        Ok(())
    }

    pub fn refusal(
        &self,
        ledger: &mut dyn Ledger,
        report: &SecretEgressBlocked,
        t: TimeMs,
    ) -> Result<(), AxError> {
        ledger.append(self.charter.line(
            EventKind::SecretEgressBlocked,
            Payload::of(report)?,
            t,
        ))?;
        Ok(())
    }

    pub fn conclude(
        self,
        ledger: &mut dyn Ledger,
        conclusion: Conclusion<'_>,
        t: TimeMs,
    ) -> Result<Completion, AxError> {
        let Conclusion {
            committed,
            answered,
            handoff,
        } = conclusion;
        ledger.append(
            self.charter
                .line(EventKind::CheckpointCommitted, committed, t),
        )?;
        let cited = ledger.append(self.charter.line(
            EventKind::HarnessAnswered,
            Payload::of(&answered)?,
            t,
        ))?;
        let completion = ending(&answered, self.cut, cited)?;
        self.charter.close(
            ledger,
            handoff,
            Verdict {
                completion: &completion,
                failure: None,
            },
            t,
        )?;
        Ok(completion)
    }

    pub fn abandon(
        self,
        ledger: &mut dyn Ledger,
        handoff: &Handoff,
        failure: Option<&AxError>,
        t: TimeMs,
    ) -> Result<Completion, AxError> {
        self.charter.close(
            ledger,
            handoff,
            Verdict {
                completion: &Completion::Cancelled,
                failure,
            },
            t,
        )?;
        Ok(Completion::Cancelled)
    }
}

fn ending(
    answered: &HarnessAnswered,
    cut: Option<Cut>,
    cited: EventRef,
) -> Result<Completion, AxError> {
    Ok(match answered.stop {
        HarnessStop::EndTurn if answered.text.trim().is_empty() => Completion::Limit,
        HarnessStop::EndTurn => Completion::Done(Evidence::new(vec![cited])?),
        HarnessStop::Cancelled => match cut {
            Some(Cut::Deadline) => Completion::Limit,
            Some(Cut::Halt) | None => Completion::Cancelled,
        },
        HarnessStop::MaxTokens | HarnessStop::MaxTurnRequests | HarnessStop::Refusal => {
            Completion::Limit
        }
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests;
