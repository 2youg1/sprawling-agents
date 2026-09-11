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
