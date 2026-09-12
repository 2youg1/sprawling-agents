// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Attached or absent, and the refusal absence answers with.
//!
//! The attached state holds a real `Endpoint`, so the credential
//! travels the one path every other call in this crate uses: resolved
//! at the last moment, exposed only while the header is written. A
//! second way to hold a credential would be a second way to leak one.

use kernel::{AxCode, AxError, DialectKind};
use serde_json::Value;

use crate::endpoint::{AuthSpec, Endpoint, EndpointConfig, Redemption, SecretResolver};
use crate::router::join;

use super::recording::Recording;
use super::wire::{form_body, transcription_of, transcription_path};

/// What a person entered when they attached a transcription endpoint.
pub struct TranscriberConfig {
    /// The provider's base URL, without a path of its own: the audio
    /// wire format knows which path hangs off it.
    pub base_url: String,
    /// The provider-side name of the model that transcribes.
    pub model: String,
    pub auth: AuthSpec,
    pub timeout_ms: u64,
    /// Settled on the endpoint this facility was chosen from, so a
    /// recording takes the same path to the provider that a chat call
    /// to the same endpoint takes.
    pub proxying: kernel::Proxying,
}

/// The city's transcription facility, which may not exist.
///
/// `absent` is not a degraded `attach`: it is the ordinary state of a
/// city nobody gave a transcription endpoint, and every call on it is
/// answered by a refusal rather than by silence.
pub struct Transcriber {
    attached: Option<Endpoint>,
}

impl Transcriber {
    /// A city with no transcription endpoint. Every `transcribe` on it
    /// refuses by name.
    #[must_use]
    pub fn absent() -> Transcriber {
        Transcriber { attached: None }
    }

    /// # Errors
    /// `E_CONFIG_INVALID` when the HTTP client cannot be built for the
    /// deadline given.
    pub fn attach(
        config: TranscriberConfig,
        secrets: SecretResolver,
    ) -> Result<Transcriber, AxError> {
        let endpoint = Endpoint::new(
            EndpointConfig {
                base_url: join(&config.base_url, transcription_path()),
                // The audio face is the OpenAI-compatible one; there is
                // no second audio wire in this build to route between.
                dialect: DialectKind::OpenAi,
                model: config.model,
                auth: config.auth,
                extra_headers: Vec::new(),
                overrides: Vec::new(),
                timeout_ms: config.timeout_ms,
                stream_deadline_ms: None,
                pricing: None,
                proxying: config.proxying,
            },
            // A recording is not a picture: this endpoint resolves a
            // credential and nothing else, and a locator reaching it
            // would be a wiring mistake rather than an attachment.
            Redemption::without_images(secrets),
        )?;
        Ok(Transcriber {
            attached: Some(endpoint),
        })
    }

    /// Whether this city can transcribe at all. The only question a
    /// settings page needs, and the one a caller should ask before
    /// drawing a control.
    #[must_use]
    pub fn is_attached(&self) -> bool {
        self.attached.is_some()
    }

    /// One recording, one HTTP round trip, one line of text.
    ///
    /// An empty answer is a real answer — silence transcribes to
    /// nothing — so emptiness never stands in for a failure here.
    ///
    /// # Errors
    /// `E_TOOL_UNAVAILABLE` when no endpoint is attached;
    /// `E_PROVIDER` on transport failure or a non-success status, with
    /// the status and URL and never the provider's own body;
    /// `E_WIRE_MISMATCH` when the answer carries no `text`.
    pub fn transcribe(&self, recording: &Recording) -> Result<String, AxError> {
        let endpoint = self.attached.as_ref().ok_or_else(unconfigured)?;
        let url = endpoint.config.base_url.clone();
        let body = form_body(&endpoint.config.model, recording);
        let request = endpoint
            .client
            .post(&url)
            .header("content-type", body.content_type)
            .header("accept", "application/json");
        let response = endpoint
            .authorize(request)?
            .body(body.bytes)
            .send()
            .map_err(|err| provider_refusal(&url, &err.to_string()))?;
        let status = response.status();
        if !status.is_success() {
            return Err(provider_refusal(
                &url,
                &format!("answered {}", status.as_u16()),
            ));
        }
        let wire: Value = response
            .json()
            .map_err(|err| provider_refusal(&url, &err.to_string()))?;
        transcription_of(&wire)
    }
}

/// What a city with no transcription endpoint says when asked to
/// transcribe.
///
/// `E_TOOL_UNAVAILABLE` is the base-table code for a facility this
/// deployment does not have. `E_CONFIG_INVALID` would say the person
/// filled something in wrongly, and nothing here is filled in wrongly:
/// the facility is optional and was not taken.
fn unconfigured() -> AxError {
    AxError::failure(
        AxCode::ToolUnavailable,
        "transcribe a recording",
        "this city has no transcription endpoint attached",
    )
    .with_recovery(
        "attach an endpoint that serves audio/transcriptions and name the model that \
         transcribes, or type the message instead",
    )
}

/// The provider's own body never reaches this refusal: an audio face
/// echoes back what it heard, and what it heard is what a person said.
fn provider_refusal(url: &str, detail: &str) -> AxError {
    AxError::failure(
        AxCode::Provider,
        "transcribe a recording",
        format!("{url} {detail}"),
    )
    .with_recovery("record again, or type the message instead")
    .retriable()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::super::recording::AudioType;
    use super::*;
    use crate::endpoint::fakes::fake_provider;
    use crate::endpoint::redemption::resolver;
    use kernel::SecretRef;

    fn recording() -> Recording {
        Recording::new(b"RIFFfake-wave-bytes".to_vec(), AudioType::Wav).unwrap()
    }

    fn attached(base_url: &str) -> Transcriber {
        Transcriber::attach(
            TranscriberConfig {
                base_url: base_url.to_owned(),
                model: "whisper-1".to_owned(),
                auth: AuthSpec::for_dialect(
                    DialectKind::OpenAi,
                    SecretRef::parse("secret:openai/api").unwrap(),
                    None,
                ),
                timeout_ms: 5_000,
                proxying: kernel::Proxying::default(),
            },
            resolver(),
        )
        .unwrap()
    }

    #[test]
    fn a_loopback_provider_answers_with_the_words_that_were_spoken() {
        let body = serde_json::json!({ "text": "raise the lab building" }).to_string();
        let (url, server) = fake_provider(vec![(200, body)], false);
        let base = url.trim_end_matches("/messages").to_owned();
        let transcriber = attached(&base);
        assert!(transcriber.is_attached());
        let text = transcriber.transcribe(&recording()).unwrap();
        assert_eq!(text, "raise the lab building");
        let seen = server.join().unwrap();
        assert!(seen[0].starts_with("POST /v1/audio/transcriptions"));
        assert!(
            seen[0]
                .to_ascii_lowercase()
                .contains("authorization: bearer sk-test-"),
            "the credential travels the one path every other call uses: {}",
            seen[0]
        );
        assert!(seen[0].contains("multipart/form-data; boundary="));
        assert!(seen[0].contains("name=\"model\"\r\n\r\nwhisper-1"));
        assert!(seen[0].contains("RIFFfake-wave-bytes"));
    }

    #[test]
    fn a_city_with_no_transcription_endpoint_refuses_by_name() {
        let err = Transcriber::absent().transcribe(&recording()).unwrap_err();
        assert!(!Transcriber::absent().is_attached());
        assert_eq!(err.code(), &AxCode::ToolUnavailable);
        assert_eq!(err.action(), "transcribe a recording");
        assert!(err.subject().contains("no transcription endpoint"));
        assert!(
            err.recovery().contains("audio/transcriptions"),
            "a person who cannot act on the refusal has been told nothing: {}",
            err.recovery()
        );
    }

    #[test]
    fn a_status_the_provider_refused_with_never_carries_its_body_back() {
        let (url, server) = fake_provider(
            vec![(
                401,
                "{\"error\":\"the key sk-live-secret is bad\"}".to_owned(),
            )],
            false,
        );
        let base = url.trim_end_matches("/messages").to_owned();
        let err = attached(&base).transcribe(&recording()).unwrap_err();
        assert_eq!(err.code(), &AxCode::Provider);
        assert!(err.subject().contains("401"));
        assert!(!err.subject().contains("sk-live-secret"));
        let _ = server.join();
    }
}
