// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How a credential reached this city: the line a vault write leaves,
//! the page a subscription login sends the person to, and the app a
//! toolkit link was asked for.
//!
//! **No line here carries a credential.** A capture names the vault
//! place and never the value; a login's URL carries a PKCE challenge and
//! a state, both public by design; a toolkit link records which app was
//! asked for and not the consent URL, which is a capability and would be
//! handed to every reader of a replayable ledger.

use serde::{Deserialize, Serialize};

use crate::secret::SecretRef;

/// `secret_captured`: a credential entered the vault at this place.
///
/// The place is a [`SecretRef`], so a line whose `ref` is outside the
/// grammar does not read as a capture at all: every reader meets that
/// as one refusal from [`Payload::read`](super::super::Payload::read)
/// rather than each deciding for itself what such a line names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SecretCaptured {
    /// The vault place the credential is stored under.
    #[serde(rename = "ref")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub reference: SecretRef,
    /// How it arrived, in the writer's words: `enrolment`, `pasted`,
    /// `<provider>-subscription` or `<provider>-renewal`. Free text,
    /// because the provider is part of the spelling.
    #[serde(default)]
    pub origin: String,
    /// When the provider said the credential stops working, in the
    /// city's clock (ms). Absent when the provider said nothing: not
    /// knowing is not an expiry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
}

/// `login_started`: a subscription login is waiting for the person.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct LoginStarted {
    /// The provider being signed in to.
    pub provider: String,
    /// The page the person opens.
    pub auth_url: String,
    /// The short code the person types on that page, for a device
    /// login. Absent rather than empty: a redirect login has no code,
    /// and a blank one would read as a code that failed to arrive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_code: Option<String>,
}

/// `toolkit_link_opened`: the person asked to connect this app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ToolkitLinkOpened {
    /// The toolkit's slug.
    pub toolkit: String,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::*;
    use crate::event::Payload;

    /// The golden bytes: each line as the hand-written writers spelled
    /// it before these structs existed, so a ledger already on disk
    /// reads back and a new line is byte-identical to an old one.
    #[test]
    fn each_credential_line_keeps_the_bytes_its_ledger_already_holds() {
        let place = SecretRef::parse("secret:anthropic/oauth").unwrap();
        let cases: [(Payload, &str); 5] = [
            (
                Payload::of(&SecretCaptured {
                    reference: place.clone(),
                    origin: "anthropic-subscription".to_owned(),
                    expires_at: Some(1_700_000_000_000),
                })
                .unwrap(),
                "{\"expires_at\":1700000000000,\"origin\":\"anthropic-subscription\",\"ref\":\"secret:anthropic/oauth\"}",
            ),
            (
                Payload::of(&SecretCaptured {
                    reference: place,
                    origin: "enrolment".to_owned(),
                    expires_at: None,
                })
                .unwrap(),
                "{\"origin\":\"enrolment\",\"ref\":\"secret:anthropic/oauth\"}",
            ),
            (
                Payload::of(&LoginStarted {
                    provider: "openai".to_owned(),
                    auth_url: "https://auth.example.test/device".to_owned(),
                    user_code: Some("ABCD-1234".to_owned()),
                })
                .unwrap(),
                "{\"auth_url\":\"https://auth.example.test/device\",\"provider\":\"openai\",\"user_code\":\"ABCD-1234\"}",
            ),
            (
                Payload::of(&LoginStarted {
                    provider: "anthropic".to_owned(),
                    auth_url: "https://auth.example.test/authorize".to_owned(),
                    user_code: None,
                })
                .unwrap(),
                "{\"auth_url\":\"https://auth.example.test/authorize\",\"provider\":\"anthropic\"}",
            ),
            (
                Payload::of(&ToolkitLinkOpened {
                    toolkit: "gmail".to_owned(),
                })
                .unwrap(),
                "{\"toolkit\":\"gmail\"}",
            ),
        ];
        for (payload, wire) in cases {
            assert_eq!(serde_json::to_string(&payload).unwrap(), wire);
            let back: Payload = serde_json::from_str(wire).unwrap();
            assert_eq!(back, payload);
        }
    }

    /// A reference outside the grammar is not a capture, so no reader
    /// can fold it as one.
    #[test]
    fn a_capture_outside_the_reference_grammar_does_not_read() {
        let payload: Payload =
            serde_json::from_str("{\"origin\":\"pasted\",\"ref\":\"anthropic/oauth\"}").unwrap();
        assert!(payload.read::<SecretCaptured>().is_err());
    }
}
