// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! From a chosen endpoint to a facility that transcribes.
//!
//! The twin of [`adapter_for`](crate::adapter_for), and it is here for
//! the same reason that one is: which endpoint answers a kind of job is
//! the book's to say, and turning that choice into something callable is
//! one step that must happen in one place. A caller that built a
//! `TranscriberConfig` itself would be a second place where a base URL
//! and a credential are joined.

use kernel::AxError;

use crate::endpoint::SecretResolver;
use crate::router::Chosen;

use super::transcriber::{Transcriber, TranscriberConfig};

/// How long one transcription may take before this city stops waiting.
///
/// Longer than a chat call's first byte and shorter than a person's
/// patience: the whole recording is uploaded before the provider
/// answers anything, so the deadline covers an upload as well as a
/// round trip.
const TRANSCRIBE_TIMEOUT_MS: u64 = 120_000;

/// Builds the facility the endpoint chosen for
/// [`ModelTag::Transcribe`](kernel::ModelTag::Transcribe) describes.
///
/// # Errors
/// Propagates the HTTP client's own refusal to be built for this
/// deadline.
pub fn transcriber_for(
    chosen: &Chosen<'_>,
    secrets: SecretResolver,
) -> Result<Transcriber, AxError> {
    Transcriber::attach(
        TranscriberConfig {
            // The base URL the person entered, not the chat URL: the
            // audio face knows which path hangs off it, and a chat path
            // with an audio path joined onto it names nothing.
            base_url: chosen.endpoint.base_url.clone(),
            model: chosen.entry.id.clone(),
            auth: chosen.endpoint.auth.clone(),
            timeout_ms: TRANSCRIBE_TIMEOUT_MS,
            proxying: chosen.endpoint.tuning.proxying,
        },
        secrets,
    )
}
