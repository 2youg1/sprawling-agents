// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which adapter a chosen model gets, and why.
//!
//! The assembly point used to own this choice as `RunWorker::adapter_for`:
//! a credential cluster holding an assembly line. The choice reads only
//! the chosen endpoint and a redemption closure, so it lives here, next
//! to the adapters it mints — one wiring place, not two.

use kernel::AxError;

use crate::endpoint::{AuthSpec, Endpoint, EndpointConfig, Redemption};
use crate::router::Chosen;

/// How long one model call may take. Moved with the choice it serves:
/// a timeout nobody here reads is a number nobody here can justify.
pub const CALL_TIMEOUT_MS: u64 = 120_000;

/// The adapter for one chosen model.
///
/// A loopback endpoint speaking the OpenAI shape goes through the
/// local adapter, which is loopback-only by construction; everything
/// else goes through the general one, which refuses to carry a
/// confidential building's bytes off this machine.
///
/// A credential decides the route too: the local adapter has no
/// authentication surface, so a loopback endpoint that was attached
/// with a secret (a local proxy, LiteLLM, a corporate gateway) must
/// take the general path. So does a header or a body override the
/// person set: the local adapter can send neither, and an adapter that
/// quietly dropped them would call the endpoint a different way from
/// the one the form showed.
pub fn adapter_for(
    chosen: &Chosen<'_>,
    redemption: Redemption,
    dialect_headers: Vec<(String, String)>,
) -> Result<Box<dyn kernel::Model + Send>, AxError> {
    let endpoint = chosen.endpoint;
    let tuning = &endpoint.tuning;
    let timeout_ms = tuning.timeout_ms.unwrap_or(CALL_TIMEOUT_MS);
    if endpoint.is_local()
        && matches!(endpoint.dialect, kernel::DialectKind::OpenAi)
        && matches!(endpoint.auth, AuthSpec::None)
        && tuning.is_plain()
    {
        let native = crate::Native::new(crate::NativeConfig {
            base_url: endpoint.chat_url(),
            model: chosen.entry.id.clone(),
            timeout_ms,
            pricing: Some(chosen.entry.clone()),
            proxying: tuning.proxying,
        })?;
        return Ok(Box::new(native));
    }
    // A person who names a header the dialect also sets replaces it,
    // rather than adding a second line with the same name: two
    // `anthropic-version` headers is a request no provider promises to
    // read the way either of them meant.
    let mut extra_headers: Vec<(String, String)> = dialect_headers
        .into_iter()
        .filter(|(name, _)| {
            !tuning
                .extra_headers
                .iter()
                .any(|(given, _)| given.eq_ignore_ascii_case(name))
        })
        .collect();
    extra_headers.extend(tuning.extra_headers.iter().cloned());
    let endpoint = Endpoint::new(
        EndpointConfig {
            base_url: endpoint.chat_url(),
            dialect: endpoint.dialect,
            model: chosen.entry.id.clone(),
            auth: endpoint.auth.clone(),
            extra_headers,
            overrides: tuning.applied_overrides(),
            timeout_ms,
            stream_deadline_ms: tuning.stream_deadline_ms,
            pricing: Some(chosen.entry.clone()),
            proxying: tuning.proxying,
        },
        redemption,
    )?;
    Ok(Box::new(endpoint))
}
