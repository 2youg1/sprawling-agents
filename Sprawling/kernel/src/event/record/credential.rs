// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::secret::SecretRef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SecretCaptured {

    #[serde(rename = "ref")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub reference: SecretRef,

    #[serde(default)]
    pub origin: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ToolkitLinkOpened {

    pub toolkit: String,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::*;
    use crate::event::Payload;

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

    #[test]
    fn a_capture_outside_the_reference_grammar_does_not_read() {
        let payload: Payload =
            serde_json::from_str("{\"origin\":\"pasted\",\"ref\":\"anthropic/oauth\"}").unwrap();
        assert!(payload.read::<SecretCaptured>().is_err());
    }
}
