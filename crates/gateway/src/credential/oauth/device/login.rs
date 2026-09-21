// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A device-code login in flight: what the vendor answered, when the
//! next ask is allowed, and what one ask returns (shape 2 adapter).
//!
//! **The person is the poll loop.** RFC 8628 has a client ask the
//! token endpoint over and over while the person approves on another
//! device; this city asks once per time the person says they are done,
//! because the alternative is a worker thread parked for up to half an
//! hour in the middle of the loop that runs the city. The RFC's two
//! numbers are still obeyed and are still this module's to enforce:
//! an ask that arrives before the stated interval has passed is
//! refused here rather than sent, and a session past its `expires_in`
//! stops. [`DevicePoll`] holds those rules; this holds the clock
//! readings they are applied to and the one exchange they govern.
//!
//! Nothing here samples a clock. The caller reads the city's own
//! clock and hands the reading in, which is what lets the interval and
//! the expiry be exercised in a test that takes no time at all.

use kernel::{AxCode, AxError};
use serde_json::Value;

use super::super::exchange;
use super::super::types::{OauthPending, OauthTokens};
use super::{
    DeviceAuthorization, DevicePoll, DeviceRefusal, PollStep, device_authorization_request,
};

/// A thousand milliseconds in a second, for turning the city's clock
/// reading into the seconds RFC 8628 states its two numbers in.
const MS_PER_S: u64 = 1_000;

/// What one ask at the token endpoint settled.
///
/// Exhaustive: either the person has approved and this city holds the
/// tokens, or they have not and the vendor states how long to wait.
/// Every other answer is a refusal that ends the login, and arrives as
/// an error rather than as a third variant.
pub enum DeviceStep {
    /// The person approved; these are theirs.
    Signed(OauthTokens),
    /// Not yet. Ask again no sooner than this many seconds from now.
    NotYet { seconds: u64 },
}

/// A device-code login this city began and the person has not
/// finished.
pub struct DeviceLogin {
    authorization: DeviceAuthorization,
    poll: DevicePoll,
    /// The city's clock when the vendor issued the code, which every
    /// elapsed-seconds figure below is measured from.
    began_at_ms: u64,
    /// The earliest the next ask may be sent, in the same clock.
    next_ask_at_ms: u64,
    /// How long one ask may take. Fixed when the login began rather
    /// than passed per ask: it is a property of this login's vendor,
    /// and two asks of one login waiting different lengths would be
    /// two answers to one question.
    ask_timeout_ms: u64,
}

impl std::fmt::Debug for DeviceLogin {
    /// The authorization's own `Debug` already withholds both codes;
    /// this adds only the schedule.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DeviceLogin {{ {:?}, began_at_ms: {}, next_ask_at_ms: {} }}",
            self.authorization, self.began_at_ms, self.next_ask_at_ms
        )
    }
}

/// Ask the vendor for a device code, and hold what it answered.
///
/// `now_ms` is the city's clock, and `timeout_ms` is how long one
/// exchange with this vendor may take.
///
/// # Errors
/// `E_CONFIG_INVALID` when the profile states no device-code grant,
/// `E_PROVIDER` when the vendor refuses the request or answers
/// something RFC 8628 §3.2 does not describe. No refusal quotes the
/// body: the device code is in it.
pub fn device_login_begin(
    profile: &crate::oauth_profiles::OauthProfile,
    now_ms: u64,
    timeout_ms: u64,
) -> Result<OauthPending, AxError> {
    let asked = device_authorization_request(profile)?;
    let answer = exchange::post_form(&asked.url, asked.body, timeout_ms)?;
    if !answer.ok {
        return Err(AxError::failure(
            AxCode::Provider,
            "begin a device-code login",
            format!("the vendor answered {}", answer.status),
        )
        .with_recovery(
            "try again in a moment, or attach this provider with an API key; a vendor \
             that keeps refusing is one whose watched path in docs/third-party.md \
             section 1 has moved",
        ));
    }
    let authorization = DeviceAuthorization::parse(&answer.body)?;
    let poll = authorization.poll();
    // RFC 8628 §3.3 has the client wait one interval before its first
    // ask, and an immediate ask can only answer `authorization_pending`.
    let next_ask_at_ms = now_ms.saturating_add(poll.first_wait_s().saturating_mul(MS_PER_S));
    Ok(OauthPending::Device(DeviceLogin {
        authorization,
        poll,
        began_at_ms: now_ms,
        next_ask_at_ms,
        ask_timeout_ms: timeout_ms,
    }))
}

impl DeviceLogin {
    /// The page the person opens. The vendor's own combined page when
    /// it offers one, because a page with the code already in it is
    /// one fewer thing for a person to copy correctly.
    #[must_use]
    pub fn open_url(&self) -> &str {
        self.authorization
            .verification_uri_complete
            .as_deref()
            .unwrap_or(&self.authorization.verification_uri)
    }

    /// The short code the person types on that page.
    #[must_use]
    pub fn user_code(&self) -> &str {
        &self.authorization.user_code
    }

    /// One ask at the token endpoint, on behalf of a person who says
    /// they have approved this login.
    ///
    /// `user_code` is what they read back off the vendor's page. It is
    /// compared rather than trusted, so that finishing one login with
    /// another's code is refused before a byte is sent — the same job
    /// the `state` parameter does on the redirect path.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` when that code is not this login's.
    /// `E_PROVIDER` when the vendor denies the login, when the code
    /// has expired, or when it answers a refusal RFC 8628 §3.5 says to
    /// stop polling on.
    pub fn ask(
        &mut self,
        profile: &crate::oauth_profiles::OauthProfile,
        user_code: &str,
        now_ms: u64,
    ) -> Result<DeviceStep, AxError> {
        if user_code.trim() != self.authorization.user_code {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "finish a device-code login",
                "that is not the code this login showed",
            )
            .with_recovery(
                "type the code this city showed you, which is the one the vendor's page \
                 is waiting for",
            ));
        }
        if now_ms < self.next_ask_at_ms {
            // RFC 8628 §3.5 makes the interval a MUST, and a vendor
            // answers a client that ignores it with `slow_down` or a
            // block. Counted from this city's clock rather than from
            // how often a person presses a button.
            let wait_ms = self.next_ask_at_ms.saturating_sub(now_ms);
            return Ok(DeviceStep::NotYet {
                seconds: wait_ms.div_euclid(MS_PER_S).saturating_add(1),
            });
        }
        let asked = self.authorization.token_request(profile);
        let answer = exchange::post_form(&asked.url, asked.body, self.ask_timeout_ms)?;
        if answer.ok {
            return Ok(DeviceStep::Signed(exchange::tokens(&answer.body)?));
        }
        let elapsed_s = now_ms.saturating_sub(self.began_at_ms).div_euclid(MS_PER_S);
        // RFC 8628 §3.5 answers a refusal with an `error` field; a
        // body without one names nothing to decide on, so the login
        // stops rather than polling an endpoint whose answer this
        // build cannot read.
        let Some(code) = answer.body.get("error").and_then(Value::as_str) else {
            return Err(AxError::failure(
                AxCode::Provider,
                "finish a device-code login",
                format!("the vendor answered {} and named no error", answer.status),
            )
            .with_recovery(
                "start the login again; an answer RFC 8628 section 3.5 does not describe \
                 is an endpoint whose watched path in docs/third-party.md section 1 has \
                 moved",
            ));
        };
        match self.poll.step(&DeviceRefusal::read(code), elapsed_s) {
            PollStep::AskAgainIn { seconds } => {
                self.next_ask_at_ms = now_ms.saturating_add(seconds.saturating_mul(MS_PER_S));
                Ok(DeviceStep::NotYet { seconds })
            }
            PollStep::Stop(refusal) => Err(refusal),
        }
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
    use crate::oauth_profiles::profile_for;
    use crate::provider::registry::Family;

    /// A login that began at clock zero, with the schedule both
    /// device-code vendors state: five seconds between asks, half an
    /// hour of life.
    fn began() -> DeviceLogin {
        let authorization = DeviceAuthorization::parse(&serde_json::json!({
            "device_code": "dev-code-1",
            "user_code": "ABCD-EFGH",
            "verification_uri": "https://auth.example.test/device",
            "interval": 5,
            "expires_in": 1800,
        }))
        .unwrap();
        let poll = authorization.poll();
        DeviceLogin {
            authorization,
            poll,
            began_at_ms: 0,
            next_ask_at_ms: 5_000,
            ask_timeout_ms: 30_000,
        }
    }

    /// Both refusals below happen before a socket is opened, which is
    /// why this test needs no vendor at all.
    #[test]
    fn another_logins_code_is_refused_before_anything_is_sent() {
        let profile = profile_for(Family::GrokBuild).unwrap();
        let Err(refusal) = began().ask(profile, "WXYZ-1234", 600_000) else {
            panic!("a code this login never showed finishes nothing");
        };
        assert_eq!(*refusal.code(), AxCode::InvalidArgs);
        assert!(refusal.subject().contains("not the code this login showed"));
    }

    /// RFC 8628 section 3.5 makes the interval a MUST. A person who
    /// presses the button a second after the code appeared is told to
    /// wait rather than being sent at the vendor.
    #[test]
    fn an_ask_inside_the_stated_interval_waits_instead_of_asking() {
        let profile = profile_for(Family::KimiCli).unwrap();
        let Ok(DeviceStep::NotYet { seconds }) = began().ask(profile, "ABCD-EFGH", 1_000) else {
            panic!("an ask four seconds early is not sent");
        };
        assert_eq!(seconds, 5, "four seconds left, rounded up to a whole one");
    }

    /// The code is read off the page the person is sent to, so the
    /// combined page wins when the vendor offers one.
    #[test]
    fn the_page_a_person_opens_is_the_one_with_the_code_already_in_it() {
        let mut login = began();
        assert_eq!(login.open_url(), "https://auth.example.test/device");
        assert_eq!(login.user_code(), "ABCD-EFGH");
        login.authorization.verification_uri_complete =
            Some("https://auth.example.test/device?code=ABCD-EFGH".to_owned());
        assert_eq!(
            login.open_url(),
            "https://auth.example.test/device?code=ABCD-EFGH"
        );
        assert!(!format!("{login:?}").contains("dev-code-1"));
    }
}
