// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How many times a failed request to one endpoint is made again.
//!
//! **One value, one absence.** The same setting used to mean three
//! things: a dispatch read a missing figure as "retry until somebody
//! halts the run", a model-list probe read it as "do not retry", and
//! the form offered `4` before anybody had settled anything. The same
//! person, the same endpoint, and two different behaviours. The figure
//! is a value here, its absence is [`Retries::UntilHalted`] by
//! construction, and no reader can spell a fourth answer.

/// The ceiling a person set on retrying one endpoint.
///
/// Exhaustive rather than a count with a sentinel: "keep trying until
/// somebody stops this" and "try four times" are different intentions,
/// and a number cannot spell the first one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Retries {
    /// No ceiling. What stops the run is `Halt`, the city's one brake.
    /// This is what a person who settled nothing asked for.
    #[default]
    UntilHalted,
    /// Give up once this many retries have been spent. Zero is a real
    /// answer and means "once, then report".
    AtMost(u32),
}

impl Retries {
    /// How many retries a request gets where nothing can halt it.
    ///
    /// A person watching a settings page is waiting on one request and
    /// holds no brake, so `UntilHalted` cannot be honoured literally
    /// there: a probe that retried forever would hang the page it was
    /// drawn for. It is honoured as **one** retry, which is what a
    /// dropped connection costs and what the probe already documented
    /// as its intent — written here so the reading is the setting's own
    /// and not a second default at the call site.
    #[must_use]
    pub const fn without_a_brake(self) -> u32 {
        match self {
            Retries::UntilHalted => 1,
            Retries::AtMost(ceiling) => ceiling,
        }
    }

    /// The figure a record carries, or nothing when no ceiling was set.
    ///
    /// Absence on the wire and absence in the ledger are the same fact,
    /// so both are written by this one reading.
    #[must_use]
    pub const fn stated(self) -> Option<u32> {
        match self {
            Retries::UntilHalted => None,
            Retries::AtMost(ceiling) => Some(ceiling),
        }
    }

    /// What a stated figure, or the absence of one, means.
    #[must_use]
    pub const fn of(stated: Option<u32>) -> Retries {
        match stated {
            None => Retries::UntilHalted,
            Some(ceiling) => Retries::AtMost(ceiling),
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    use super::*;

    #[test]
    fn absence_is_one_answer_wherever_it_is_read() {
        assert_eq!(Retries::of(None), Retries::UntilHalted);
        assert_eq!(Retries::default(), Retries::UntilHalted);
        assert_eq!(Retries::UntilHalted.stated(), None);
        // The one reading a probe takes, and it is bounded.
        assert_eq!(Retries::UntilHalted.without_a_brake(), 1);
    }

    #[test]
    fn a_stated_ceiling_survives_the_record_it_is_written_into() {
        for ceiling in [0_u32, 1, 4, u32::MAX] {
            let held = Retries::AtMost(ceiling);
            assert_eq!(Retries::of(held.stated()), held);
            assert_eq!(held.without_a_brake(), ceiling);
        }
    }
}
