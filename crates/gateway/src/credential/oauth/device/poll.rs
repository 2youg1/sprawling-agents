// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How often to ask, and when to stop asking (shape 1 decision).
//!
//! RFC 8628 §3.5 fixes both, and the two rules that matter are easy to
//! get wrong in opposite directions: a client that ignores `slow_down`
//! is throttled by the vendor, and a client that never stops polls a
//! code the vendor has already forgotten. The elapsed time arrives as
//! an argument, so every rule below is exercised by a test that takes
//! no time at all.

use kernel::{AxCode, AxError};

/// What one `slow_down` adds. RFC 8628 §3.5: "the interval MUST be
/// increased by 5 seconds for this and all subsequent requests".
const SLOW_DOWN_INCREMENT_S: u64 = 5;

/// What the token endpoint said when it did not hand over a token.
///
/// Exhaustive over the four codes RFC 8628 §3.5 defines, with every
/// other code — including the ones RFC 6749 §5.2 defines — carried
/// whole in the last variant. The RFC splits on exactly this
/// distinction: two codes mean keep polling and everything else means
/// stop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceRefusal {
    /// "The authorization request is still pending as the end user
    /// hasn't yet completed the user-interaction steps."
    AuthorizationPending,
    /// "A variant of `authorization_pending` … but the interval MUST
    /// be increased by 5 seconds."
    SlowDown,
    /// "The authorization request was denied."
    AccessDenied,
    /// "The `device_code` has expired, and the device authorization
    /// session has concluded."
    ExpiredToken,
    /// Any other code. RFC 8628 §3.5: "If the client receives an error
    /// response with any other error code, it MUST stop polling".
    Other(String),
}

impl DeviceRefusal {
    /// The refusal one `error` field names.
    #[must_use]
    pub fn read(code: &str) -> DeviceRefusal {
        match code {
            "authorization_pending" => DeviceRefusal::AuthorizationPending,
            "slow_down" => DeviceRefusal::SlowDown,
            "access_denied" => DeviceRefusal::AccessDenied,
            "expired_token" => DeviceRefusal::ExpiredToken,
            other => DeviceRefusal::Other(other.to_owned()),
        }
    }
}

/// What to do after one refusal.
#[derive(Debug, PartialEq, Eq)]
pub enum PollStep {
    /// Wait this many seconds, then ask the token endpoint again.
    AskAgainIn { seconds: u64 },
    /// Stop asking, and tell the person this.
    Stop(AxError),
}

/// The polling schedule of one device-code login.
///
/// Holds the two numbers the vendor stated and the increases it has
/// asked for since; the elapsed time arrives as an argument, because
/// this city samples no clock outside its assembly.
pub struct DevicePoll {
    interval_s: u64,
    expires_in_s: u64,
}

impl DevicePoll {
    /// The schedule one device authorization response prescribes: how
    /// long to wait between asks, and how long the code lives.
    ///
    /// Built from the response rather than from a caller's preference,
    /// which is why this is reachable only from the module that reads
    /// that response.
    pub(super) const fn new(interval_s: u64, expires_in_s: u64) -> DevicePoll {
        DevicePoll {
            interval_s,
            expires_in_s,
        }
    }

    /// The wait before the first ask, which is one interval: RFC 8628
    /// §3.3 has the client poll after the device authorization
    /// response, and an immediate ask can only answer
    /// `authorization_pending`.
    #[must_use]
    pub const fn first_wait_s(&self) -> u64 {
        self.interval_s
    }

    /// What one refusal means for the next ask.
    ///
    /// `elapsed_s` is the time since the device authorization response
    /// was read. The session ends when it passes `expires_in`, which
    /// RFC 8628 §3.2 defines as "the lifetime in seconds of the
    /// `device_code` and `user_code`": polling past it asks about a
    /// code the vendor has already forgotten.
    pub fn step(&mut self, said: &DeviceRefusal, elapsed_s: u64) -> PollStep {
        match said {
            DeviceRefusal::AuthorizationPending => self.keep_polling(elapsed_s),
            DeviceRefusal::SlowDown => {
                self.interval_s = self.interval_s.saturating_add(SLOW_DOWN_INCREMENT_S);
                self.keep_polling(elapsed_s)
            }
            DeviceRefusal::AccessDenied => PollStep::Stop(
                AxError::failure(
                    AxCode::Provider,
                    "sign in with a device code",
                    "the login was refused on the vendor's page",
                )
                .with_recovery(
                    "start the login again and approve it, or attach this provider with \
                     an API key",
                ),
            ),
            DeviceRefusal::ExpiredToken => PollStep::Stop(self.expired()),
            DeviceRefusal::Other(code) => PollStep::Stop(
                AxError::failure(
                    AxCode::Provider,
                    "sign in with a device code",
                    format!("the vendor answered `{code}` and this login cannot continue"),
                )
                .with_recovery(
                    "start the login again; a code this city no longer understands is not \
                     retried, because polling past a refusal is what gets a client \
                     throttled",
                ),
            ),
        }
    }

    /// The next ask, unless the code has run out of life.
    fn keep_polling(&self, elapsed_s: u64) -> PollStep {
        if elapsed_s.saturating_add(self.interval_s) >= self.expires_in_s {
            return PollStep::Stop(self.expired());
        }
        PollStep::AskAgainIn {
            seconds: self.interval_s,
        }
    }

    fn expired(&self) -> AxError {
        AxError::failure(
            AxCode::Provider,
            "sign in with a device code",
            format!(
                "the code the vendor issued lasted {} seconds and has expired",
                self.expires_in_s
            ),
        )
        .with_recovery("start the login again and enter the new code on the vendor's page")
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

    /// Five seconds between asks and half an hour of life, which is
    /// the schedule both device-code vendors this city follows state.
    fn schedule() -> DevicePoll {
        DevicePoll::new(5, 1800)
    }

    /// RFC 8628 section 3.5: one `slow_down` raises the interval by
    /// five seconds "for this and all subsequent requests".
    #[test]
    fn slow_down_raises_the_interval_for_every_later_ask() {
        let mut poll = schedule();
        assert_eq!(
            poll.step(&DeviceRefusal::SlowDown, 5),
            PollStep::AskAgainIn { seconds: 10 }
        );
        assert_eq!(
            poll.step(&DeviceRefusal::AuthorizationPending, 15),
            PollStep::AskAgainIn { seconds: 10 }
        );
        assert_eq!(
            poll.step(&DeviceRefusal::SlowDown, 25),
            PollStep::AskAgainIn { seconds: 15 }
        );
    }

    #[test]
    fn a_denial_and_an_unknown_code_both_stop_the_polling() {
        let mut poll = schedule();
        let PollStep::Stop(denied) = poll.step(&DeviceRefusal::AccessDenied, 10) else {
            panic!("a refused login is not polled again");
        };
        assert_eq!(*denied.code(), AxCode::Provider);
        assert!(denied.subject().contains("refused on the vendor's page"));
        let unknown = DeviceRefusal::read("invalid_client");
        assert_eq!(unknown, DeviceRefusal::Other("invalid_client".to_owned()));
        let PollStep::Stop(other) = poll.step(&unknown, 15) else {
            panic!("RFC 8628 section 3.5: any other error code stops the polling");
        };
        assert_eq!(*other.code(), AxCode::Provider);
    }

    /// The session ends with the code, whether the vendor says so or
    /// the clock does: an ask that would land after `expires_in` is an
    /// ask about a code the vendor has forgotten.
    #[test]
    fn polling_stops_when_the_code_runs_out_of_life() {
        let mut poll = DevicePoll::new(5, 30);
        assert_eq!(
            poll.step(&DeviceRefusal::AuthorizationPending, 10),
            PollStep::AskAgainIn { seconds: 5 }
        );
        let PollStep::Stop(late) = poll.step(&DeviceRefusal::AuthorizationPending, 26) else {
            panic!("the next ask would land after the code expired");
        };
        assert!(late.subject().contains("30 seconds"));
        let PollStep::Stop(told) = poll.step(&DeviceRefusal::ExpiredToken, 12) else {
            panic!("the vendor said the code expired");
        };
        assert_eq!(*told.code(), AxCode::Provider);
    }
}
