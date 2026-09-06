// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
//! OAuth types: pending, request, tokens.

use zeroize::Zeroizing;

pub struct OauthPending {
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
