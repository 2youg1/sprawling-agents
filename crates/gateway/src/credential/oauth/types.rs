// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
//! OAuth types: pending, request, tokens.

use zeroize::Zeroizing;

use super::device::DeviceLogin;

/// A login this city began and has not finished, in whichever of the
/// two shapes the vendor's grant prescribes.
///
/// One type rather than two tables, because the caller holds exactly
/// one half-finished login per provider and the shape is the vendor's
/// statement rather than the caller's choice. What a person is shown
/// is the same in both shapes — a page to open — so [`Self::open_url`]
/// answers for both, and only the device shape has a code to read
/// aloud.
pub enum OauthPending {
    /// The vendor sends the person back to a redirect with a code.
    Redirect(RedirectPending),
    /// The person types a code on the vendor's page and this city asks
    /// the token endpoint whether they are done.
    Device(DeviceLogin),
}

impl OauthPending {
    /// The page the person opens to approve this login.
    #[must_use]
    pub fn open_url(&self) -> &str {
        match self {
            OauthPending::Redirect(redirect) => &redirect.auth_url,
            OauthPending::Device(device) => device.open_url(),
        }
    }

    /// The short code the person types on that page, for the shape
    /// that has one. Not a secret: it is meaningless without the
    /// device code this city keeps.
    #[must_use]
    pub fn user_code(&self) -> Option<&str> {
        match self {
            OauthPending::Redirect(_) => None,
            OauthPending::Device(device) => Some(device.user_code()),
        }
    }
}

/// The browser-redirect half of a login: where to send the person, and
/// the two proofs the redemption checks.
pub struct RedirectPending {
    pub auth_url: String,
    pub state: String,
    pub(crate) code_verifier: Zeroizing<String>,
}

/// The token-endpoint POST, ready to send: url plus JSON body.
pub struct TokenRequest {
    pub url: String,
    pub body: String,
}
pub struct OauthTokens {
    pub access: Zeroizing<String>,
    /// Absent when the provider issues no refresh token, which is a
    /// fact about that provider rather than a failure.
    pub refresh: Option<Zeroizing<String>>,
    /// Seconds from now, as the provider states them.
    pub expires_in_s: Option<u64>,
}

impl std::fmt::Debug for OauthTokens {
    /// States what was returned, never what it was. `Zeroizing`'s own
    /// `Debug` prints the string, so a derive here would put a live
    /// token into the first panic message that formats one.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "OauthTokens {{ access: <redacted>, refresh: {}, expires_in_s: {:?} }}",
            if self.refresh.is_some() {
                "<redacted>"
            } else {
                "none"
            },
            self.expires_in_s
        )
    }
}
