// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::Arc;

use kernel::{AxCode, AxError, Locator, Sealed, SecretRef};

pub type SecretResolver = Box<dyn Fn(&SecretRef) -> Result<Sealed<String>, AxError> + Send>;

pub type ImageResolver = Arc<dyn Fn(&Locator) -> Result<Vec<u8>, AxError> + Send + Sync>;

pub struct Redemption {
    pub(crate) secrets: SecretResolver,
    pub(crate) images: ImageResolver,
}

impl Redemption {
    pub fn new(secrets: SecretResolver, images: ImageResolver) -> Redemption {
        Redemption { secrets, images }
    }

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

    pub(crate) fn holds(&self, reference: &SecretRef) -> bool {
        match (self.secrets)(reference) {
            Ok(_dropped) => true,
            Err(err) => *err.code() != AxCode::CredentialMissing,
        }
    }

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
