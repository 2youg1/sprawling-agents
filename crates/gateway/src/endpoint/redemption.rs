// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one endpoint redeems at the wire: the credential that authorises
//! the call, and the pictures the conversation refers to.
//!
//! Both are closures the assembly point supplies, so this crate never
//! learns where a secret is kept or where content is stored. Neither is
//! cached: a value is resolved for one operation and dropped.

use std::sync::Arc;

use kernel::{AxCode, AxError, Locator, Sealed, SecretRef};

/// The redemption face `credential` provides: resolve a
/// reference into a sealed value, per operation, never cached.
pub type SecretResolver = Box<dyn Fn(&SecretRef) -> Result<Sealed<String>, AxError> + Send>;

/// The picture face `bin::assembly` provides: read the bytes a `cas:`
/// locator names. Shared rather than owned, because one content store
/// answers every endpoint this process builds.
pub type ImageResolver = Arc<dyn Fn(&Locator) -> Result<Vec<u8>, AxError> + Send + Sync>;

/// Everything one endpoint redeems at the wire.
///
/// The two closures always travel together — a call writes its auth
/// header and fetches its pictures in the same breath — so they are one
/// named value rather than two constructor parameters that every call
/// site has to keep in the same order.
pub struct Redemption {
    pub(crate) secrets: SecretResolver,
    pub(crate) images: ImageResolver,
}

impl Redemption {
    pub fn new(secrets: SecretResolver, images: ImageResolver) -> Redemption {
        Redemption { secrets, images }
    }

    /// Redeems the credential of the account this endpoint calls as.
    ///
    /// A reference the store does not hold is this account's matter, so
    /// its `E_CREDENTIAL_MISSING` says the next account should take the
    /// request (gateway D31); every other refusal passes through as it
    /// came, because a locked or broken vault fails every account alike
    /// (kernel D55).
    pub(crate) fn account_credential(
        &self,
        reference: &SecretRef,
    ) -> Result<Sealed<String>, AxError> {
        (self.secrets)(reference).map_err(|err| {
            if *err.code() == AxCode::CredentialMissing {
                AxError::failure(AxCode::CredentialMissing, err.action(), err.subject())
                    .with_nearby(err.nearby().to_vec())
                    .account_unusable()
                    .with_recovery(err.recovery())
            } else {
                err
            }
        })
    }

    /// For a call that has no business carrying a picture — a probe
    /// asking an endpoint which models it serves. A picture reaching it
    /// is a wiring mistake, and the refusal says so rather than sending
    /// an empty one.
    pub fn without_images(secrets: SecretResolver) -> Redemption {
        Redemption {
            secrets,
            images: Arc::new(|at: &Locator| {
                Err(
                    AxError::failure(AxCode::InvalidArgs, "resolve a picture", at.to_string())
                        .with_recovery(
                            "this endpoint was built without a content store; \
                     send the request through a run adapter instead",
                        ),
                )
            }),
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test helper"
)]
pub(crate) fn resolver() -> SecretResolver {
    Box::new(|_reference: &SecretRef| {
        // Runtime-assembled sample: no complete token literal at rest.
        let token = ["sk-test-", "0123456789"].concat();
        Ok(Sealed::new(Box::new(token)))
    })
}
#[cfg(test)]
#[allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test helper"
)]
pub(crate) fn redemption() -> Redemption {
    Redemption::new(
        resolver(),
        Arc::new(|_at: &Locator| Ok(b"the-pixels".to_vec())),
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
    use super::super::config::Endpoint;
    use super::super::fakes::{config, request};
    use super::*;

    /// gateway D31: the credential the account calls with is this
    /// account's matter, so its absence switches to the next account; a
    /// reference in an extra header belongs to the endpoint every account
    /// shares, and switching would not bring it back.
    #[test]
    fn a_missing_account_credential_switches_the_account_and_a_missing_header_does_not() {
        let account = SecretRef::parse("secret:anthropic/api").unwrap();
        let only = move |present: Option<SecretRef>| -> Redemption {
            Redemption::new(
                Box::new(move |reference: &SecretRef| {
                    if present.as_ref() == Some(reference) {
                        Ok(Sealed::new(Box::new("sk-test".to_owned())))
                    } else {
                        Err(AxError::failure(
                            AxCode::CredentialMissing,
                            "resolve credential",
                            reference.to_string(),
                        )
                        .with_recovery("store the credential"))
                    }
                }),
                Arc::new(|_at: &Locator| Ok(Vec::new())),
            )
        };
        let refused = |endpoint_config, redemption| {
            let mut endpoint = Endpoint::new(endpoint_config, redemption).unwrap();
            let err = endpoint.call(&request()).unwrap_err();
            let json = serde_json::to_value(&err).unwrap();
            (
                *err.code(),
                json["retry"].clone(),
                json.get("account").cloned(),
            )
        };
        let mut shared = config("http://127.0.0.1:9/v1");
        shared.extra_headers.push((
            "x-gateway-key".to_owned(),
            crate::endpoint::HeaderValue::Redeemed(SecretRef::parse("secret:gateway/key").unwrap()),
        ));
        assert_eq!(
            [
                refused(config("http://127.0.0.1:9/v1"), only(None)),
                refused(shared, only(Some(account))),
            ],
            [
                (
                    AxCode::CredentialMissing,
                    serde_json::json!("no"),
                    Some(serde_json::json!("advance"))
                ),
                (AxCode::CredentialMissing, serde_json::json!("no"), None),
            ]
        );
    }
}
