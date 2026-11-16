// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {

    pub text: String,

    pub complained: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {

    Settled { looks: u32 },

    LookAgain { looks: u32 },

    Complained { looks: u32 },

    GaveUp { looks: u32, why: String },
}

pub const LOOKS_MAX: u32 = 8;

pub const QUIET_LOOKS: u32 = 2;

#[derive(Debug, Clone, Default)]
pub struct DevLoop {
    looks: u32,
    same_in_a_row: u32,
    last: Option<String>,
}

impl DevLoop {
    #[must_use]
    pub fn new() -> DevLoop {
        DevLoop::default()
    }

    pub fn observe(&mut self, observation: &Observation) -> Result<Step, AxError> {
        if self.looks >= LOOKS_MAX {
            return Err(AxError::failure(
                AxCode::LoopSuspected,
                "continue a development loop",
                format!("this loop already took its {LOOKS_MAX} looks"),
            )
            .with_recovery("start a new loop, or show the page to the User"));
        }
        self.looks = self.looks.saturating_add(1);
        if observation.complained {
            return Ok(Step::Complained { looks: self.looks });
        }
        let unchanged = self.last.as_deref() == Some(observation.text.as_str());
        self.same_in_a_row = if unchanged {
            self.same_in_a_row.saturating_add(1)
        } else {
            0
        };
        self.last = Some(observation.text.clone());
        if self.same_in_a_row >= QUIET_LOOKS {
            return Ok(Step::Settled { looks: self.looks });
        }
        if self.looks >= LOOKS_MAX {
            return Ok(Step::GaveUp {
                looks: self.looks,
                why: format!("the page was still changing after {LOOKS_MAX} looks"),
            });
        }
        Ok(Step::LookAgain { looks: self.looks })
    }

    #[must_use]
    pub fn looks(&self) -> u32 {
        self.looks
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
    reason = "test code"
)]
mod tests {
    use super::*;

    fn look(text: &str) -> Observation {
        Observation {
            text: text.to_owned(),
            complained: false,
        }
    }

    #[test]
    fn a_page_that_stops_changing_settles() {
        let mut loop_ = DevLoop::new();
        assert_eq!(
            loop_.observe(&look("a")).unwrap(),
            Step::LookAgain { looks: 1 }
        );
        assert_eq!(
            loop_.observe(&look("b")).unwrap(),
            Step::LookAgain { looks: 2 }
        );
        assert_eq!(
            loop_.observe(&look("b")).unwrap(),
            Step::LookAgain { looks: 3 }
        );
        assert_eq!(
            loop_.observe(&look("b")).unwrap(),
            Step::Settled { looks: 4 }
        );
    }

    #[test]
    fn a_complaint_ends_the_loop_rather_than_being_retried_through() {
        let mut loop_ = DevLoop::new();
        loop_.observe(&look("a")).unwrap();
        let step = loop_
            .observe(&Observation {
                text: "a".to_owned(),
                complained: true,
            })
            .unwrap();
        assert_eq!(step, Step::Complained { looks: 2 });
    }

    #[test]
    fn a_page_that_never_stops_changing_gives_up_and_says_so() {
        let mut loop_ = DevLoop::new();
        let mut last = Step::LookAgain { looks: 0 };
        for n in 0..LOOKS_MAX {
            last = loop_.observe(&look(&format!("frame {n}"))).unwrap();
        }
        match last {
            Step::GaveUp { looks, why } => {
                assert_eq!(looks, LOOKS_MAX);
                assert!(why.contains("still changing"));
            }
            other => panic!("a loop that cannot converge has to end: {other:?}"),
        }
    }

    #[test]
    fn every_loop_ends_whatever_the_page_does() {

        for pattern in [0u32, 1, 2, 3] {
            let mut loop_ = DevLoop::new();
            let mut ended = false;
            for n in 0..LOOKS_MAX {
                let text = match pattern {
                    0 => "same".to_owned(),
                    1 => format!("{n}"),
                    2 => format!("{}", n % 2),
                    _ => format!("{}", n / 3),
                };
                match loop_.observe(&look(&text)).unwrap() {
                    Step::LookAgain { .. } => {}
                    _ => {
                        ended = true;
                        break;
                    }
                }
            }
            assert!(ended, "pattern {pattern} never reached an ending");
        }
    }

    #[test]
    fn looking_after_the_ending_is_the_callers_mistake_and_is_named() {
        let mut loop_ = DevLoop::new();
        for n in 0..LOOKS_MAX {
            let _ = loop_.observe(&look(&format!("{n}")));
        }
        let err = loop_.observe(&look("again")).unwrap_err();
        assert_eq!(err.code(), &AxCode::LoopSuspected);
        assert!(err.recovery().contains("User"));
    }
}
