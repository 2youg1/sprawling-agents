// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The device-code login, as a decision this city can test without a
//! network and without a clock (shape 1 decision).
//!
//! **Two of the four subscriptions sign in on a second device.** The
//! person reads a short code off this machine, types it on the
//! vendor's page, and this city asks the token endpoint over and over
//! until the person is done. How long to wait between asks and when to
//! stop asking is not a preference: RFC 8628 §3.5 fixes both, and a
//! client that picks its own numbers is throttled by the vendor or
//! polls a dead code forever.
//!
//! Every rule below is quoted from the RFC in the item that carries
//! it. The waiting is the caller's: this module states the seconds and
//! reads the elapsed seconds back, so the whole flow is exercised in a
//! test that takes no time at all.

use kernel::{AxCode, AxError};
use serde_json::Value;
use zeroize::Zeroizing;

mod poll;

pub use poll::{DevicePoll, DeviceRefusal, PollStep};

use super::codec::percent_encode;

/// The interval to use when the device authorization response states
/// none. RFC 8628 §3.5: the client "MUST wait at least the number of
/// seconds specified by the `interval` parameter of the device
/// authorization response, or 5 seconds if none was provided".
const INTERVAL_WHEN_UNSTATED_S: u64 = 5;

/// The shortest interval this city will poll at, whatever the vendor
/// states. A response carrying `"interval": 0` would otherwise become
/// a loop with no wait in it, which is the one reading of the RFC that
/// produces a denial of service against the vendor and against this
/// machine.
const SHORTEST_INTERVAL_S: u64 = 1;

/// The form POST this city sends, spelled once.
///
/// The OAuth token endpoint takes `application/x-www-form-urlencoded`
/// (RFC 6749 §4.1.3, carried into RFC 8628 §3.4), which is a different
/// content type from the JSON body the authorization-code path in this
/// crate sends, so the type travels with the body rather than being
/// assumed by whoever sends it.
pub struct FormPost {
    pub url: String,
    pub body: String,
}

impl FormPost {
    /// The one content type a form POST is sent under.
    pub const CONTENT_TYPE: &'static str = "application/x-www-form-urlencoded";
}

impl std::fmt::Debug for FormPost {
    /// States where the post goes, never what is in it: the token
    /// request's body carries the device code, which is the secret
    /// half of this login.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FormPost {{ url: {}, body: <redacted> }}", self.url)
    }
}

/// The device authorization request: RFC 8628 §3.1.
///
/// # Errors
/// `E_CONFIG_INVALID` when the profile states no device authorization
/// endpoint or no client id, because a login begun against an empty
/// URL fails at the socket with nothing for a person to act on.
pub fn device_authorization_request(
    profile: &crate::oauth_profiles::OauthProfile,
) -> Result<FormPost, AxError> {
    let crate::oauth_profiles::Grant::DeviceCode {
        authorization_endpoint,
    } = profile.grant
    else {
        return Err(AxError::failure(
            AxCode::ConfigInvalid,
            "begin a device-code login",
            format!("{} signs in by browser redirect", profile.provider),
        )
        .with_recovery(
            "sign in to this subscription in a browser window, or attach this provider \
             with an API key",
        ));
    };
    if authorization_endpoint.is_empty() || profile.client_id.is_empty() {
        return Err(AxError::failure(
            AxCode::ConfigInvalid,
            "begin a device-code login",
            format!("{}: profile intelligence incomplete", profile.provider),
        )
        .with_recovery(
            "fill the oauth_profiles row for this provider from the upstream path \
             docs/third-party.md section 1 names for it",
        ));
    }
    let body = format!(
        "client_id={}&scope={}",
        percent_encode(profile.client_id),
        percent_encode(&profile.scopes.join(" ")),
    );
    Ok(FormPost {
        url: authorization_endpoint.to_owned(),
        body,
    })
}

/// What the vendor answered when asked for a device code, read into the
/// four facts the rest of the login needs.
pub struct DeviceAuthorization {
    /// What the person types on the vendor's page.
    pub user_code: String,
    /// Where the person types it.
    pub verification_uri: String,
    /// The same page with the code already in it, when the vendor
    /// offers one. Absent is a fact about that vendor, not a failure.
    pub verification_uri_complete: Option<String>,
    /// The secret half: never shown to a person, never logged.
    device_code: Zeroizing<String>,
    interval_s: u64,
    expires_in_s: u64,
}

impl std::fmt::Debug for DeviceAuthorization {
    /// The device code is the half a thief needs and the user code is
    /// the half a person reads aloud, so neither is printed.
    /// `Zeroizing`'s own `Debug` prints the string it wraps, which is
    /// why this is written rather than derived.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DeviceAuthorization {{ verification_uri: {}, interval_s: {}, expires_in_s: {} }}",
            self.verification_uri, self.interval_s, self.expires_in_s
        )
    }
}

impl DeviceAuthorization {
    /// Read the device authorization response (RFC 8628 §3.2).
    ///
    /// `device_code`, `user_code`, `verification_uri` and `expires_in`
    /// are REQUIRED by that section, so a body missing one is refused
    /// rather than defaulted; `interval` is OPTIONAL and its absence
    /// has a value fixed by the RFC.
    ///
    /// # Errors
    /// `E_PROVIDER` when a required field is absent or has the wrong
    /// JSON type. The refusal never quotes the body: a live device code
    /// is in it.
    pub fn parse(answer: &Value) -> Result<DeviceAuthorization, AxError> {
        let text = |key: &str| -> Result<String, AxError> {
            answer
                .get(key)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| missing(key))
        };
        let expires_in_s = answer
            .get("expires_in")
            .and_then(Value::as_u64)
            .ok_or_else(|| missing("expires_in"))?;
        Ok(DeviceAuthorization {
            user_code: text("user_code")?,
            verification_uri: text("verification_uri")?,
            verification_uri_complete: answer
                .get("verification_uri_complete")
                .and_then(Value::as_str)
                .map(str::to_owned),
            device_code: Zeroizing::new(text("device_code")?),
            interval_s: answer
                .get("interval")
                .and_then(Value::as_u64)
                .unwrap_or(INTERVAL_WHEN_UNSTATED_S)
                .max(SHORTEST_INTERVAL_S),
            expires_in_s,
        })
    }

    /// The polling schedule this response prescribes.
    #[must_use]
    pub fn poll(&self) -> DevicePoll {
        DevicePoll::new(self.interval_s, self.expires_in_s)
    }

    /// One ask at the token endpoint (RFC 8628 §3.4).
    #[must_use]
    pub fn token_request(&self, profile: &crate::oauth_profiles::OauthProfile) -> FormPost {
        FormPost {
            url: profile.token_endpoint.to_owned(),
            body: format!(
                "grant_type={}&device_code={}&client_id={}",
                percent_encode(DEVICE_GRANT_TYPE),
                percent_encode(self.device_code.as_str()),
                percent_encode(profile.client_id),
            ),
        }
    }
}

/// The grant type a device-code token request carries (RFC 8628 §3.4).
const DEVICE_GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";

/// A required field of the device authorization response that the
/// vendor did not send.
fn missing(key: &str) -> AxError {
    AxError::failure(
        AxCode::Provider,
        "begin a device-code login",
        format!("the vendor's answer states no `{key}`"),
    )
    .with_recovery(
        "check docs/third-party.md section 1 for this vendor's watched path: an answer \
         missing a field RFC 8628 requires is an endpoint that moved",
    )
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
    use crate::oauth_profiles::{Grant, profile_for};
    use crate::provider::registry::Family;

    fn answer(interval: Option<u64>, expires_in: u64) -> Value {
        let mut body = serde_json::json!({
            "device_code": "dev-code-1",
            "user_code": "ABCD-EFGH",
            "verification_uri": "https://auth.example.test/device",
            "expires_in": expires_in,
        });
        if let Some(interval) = interval
            && let Some(map) = body.as_object_mut()
        {
            map.insert("interval".to_owned(), Value::from(interval));
        }
        body
    }

    #[test]
    fn an_answer_stating_no_interval_is_polled_at_the_rfc_default() {
        let read = DeviceAuthorization::parse(&answer(None, 1800)).unwrap();
        assert_eq!(read.poll().first_wait_s(), 5);
        assert_eq!(read.user_code, "ABCD-EFGH");
        assert!(read.verification_uri_complete.is_none());
    }

    #[test]
    fn a_stated_interval_is_honoured_and_a_zero_one_is_floored() {
        assert_eq!(
            DeviceAuthorization::parse(&answer(Some(9), 1800))
                .unwrap()
                .poll()
                .first_wait_s(),
            9
        );
        assert_eq!(
            DeviceAuthorization::parse(&answer(Some(0), 1800))
                .unwrap()
                .poll()
                .first_wait_s(),
            1,
            "a zero interval is a loop with no wait in it"
        );
    }

    #[test]
    fn a_required_field_the_vendor_did_not_send_is_refused_without_quoting_the_body() {
        let mut body = answer(None, 1800);
        if let Some(map) = body.as_object_mut() {
            map.remove("device_code");
        }
        let refusal = DeviceAuthorization::parse(&body).unwrap_err();
        assert_eq!(*refusal.code(), AxCode::Provider);
        assert!(refusal.subject().contains("device_code"));
        assert!(!refusal.subject().contains("ABCD-EFGH"));
    }

    #[test]
    fn the_two_posts_carry_the_grant_the_rfc_names_and_no_secret_in_the_url() {
        let profile = profile_for(Family::GrokBuild).unwrap();
        let begin = device_authorization_request(profile).unwrap();
        assert_eq!(begin.url, "https://auth.x.ai/oauth2/device/code");
        assert!(begin.body.starts_with("client_id="));
        assert!(begin.body.contains("&scope=openid%20profile"));
        let read = DeviceAuthorization::parse(&answer(Some(5), 1800)).unwrap();
        let ask = read.token_request(profile);
        assert_eq!(ask.url, profile.token_endpoint);
        assert!(
            ask.body
                .contains("grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code")
        );
        assert!(ask.body.contains("device_code=dev-code-1"));
        assert_eq!(FormPost::CONTENT_TYPE, "application/x-www-form-urlencoded");
    }

    /// A family that signs in through a browser is refused here rather
    /// than sent a form the vendor never promised to read.
    #[test]
    fn a_redirect_family_cannot_be_driven_down_the_device_path() {
        let profile = profile_for(Family::ClaudeCode).unwrap();
        assert!(matches!(profile.grant, Grant::AuthorizationCode { .. }));
        let refusal = device_authorization_request(profile).unwrap_err();
        assert_eq!(*refusal.code(), AxCode::ConfigInvalid);
        assert!(refusal.subject().contains("browser redirect"));
    }
}
