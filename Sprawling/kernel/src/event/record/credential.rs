// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How a credential reached this city: the line a vault write leaves,
//! and the app a toolkit link was asked for.
//!
//! **No line here carries a credential.** A capture names the vault
//! place and never the value; a toolkit link records which app was
//! asked for and not the consent URL, which is a capability and would be
//! handed to every reader of a replayable ledger.
//!
//! Older builds also wrote `login_started` for a subscription login and
//! an `expires_at` key on a capture. This build signs in to no
//! subscription (`crates/gateway/Spec.lean` §8-5): the kind stays in the vocabulary
//! so such a ledger still reads, nothing reads its payload, and a
//! capture's `expires_at` is ignored on the way in.

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
    /// How it arrived, in the writer's words: `enrolment` or `pasted`.
    /// An older build also wrote `<provider>-subscription` and
    /// `<provider>-renewal`. Free text, because those spellings carry a
    /// provider name.
    #[serde(default)]
    pub origin: String,
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
        let place = SecretRef::parse("secret:anthropic/api-key").unwrap();
        let cases: [(Payload, &str); 2] = [
            (
                Payload::of(&SecretCaptured {
                    reference: place,
                    origin: "enrolment".to_owned(),
                })
                .unwrap(),
                "{\"origin\":\"enrolment\",\"ref\":\"secret:anthropic/api-key\"}",
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

    /// A capture an older build wrote for a subscription token, with its
    /// `expires_at`, still reads: the key is ignored rather than refused,
    /// so a ledger that holds one replays (`crates/gateway/Spec.lean` §8-5).
    #[test]
    fn a_capture_with_an_expiry_an_older_build_wrote_still_reads() {
        let payload: Payload = serde_json::from_str(
            "{\"expires_at\":1700000000000,\"origin\":\"anthropic-subscription\",\"ref\":\"secret:anthropic/oauth\"}",
        )
        .unwrap();
        assert_eq!(
            payload.read::<SecretCaptured>().unwrap(),
            SecretCaptured {
                reference: SecretRef::parse("secret:anthropic/oauth").unwrap(),
                origin: "anthropic-subscription".to_owned(),
            }
        );
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
