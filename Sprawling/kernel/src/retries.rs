// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Retries {
    #[default]
    UntilHalted,

    AtMost(u32),
}

impl Retries {
    #[must_use]
    pub const fn without_a_brake(self) -> u32 {
        match self {
            Retries::UntilHalted => 1,
            Retries::AtMost(ceiling) => ceiling,
        }
    }

    #[must_use]
    pub const fn stated(self) -> Option<u32> {
        match self {
            Retries::UntilHalted => None,
            Retries::AtMost(ceiling) => Some(ceiling),
        }
    }

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
