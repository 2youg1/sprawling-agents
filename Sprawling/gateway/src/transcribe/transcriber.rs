// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError, DialectKind};
use serde_json::Value;

use crate::endpoint::{AuthSpec, Endpoint, EndpointConfig, Redemption, SecretResolver};
use crate::router::join;

use super::recording::Recording;
use super::wire::{form_body, transcription_of, transcription_path};

pub struct TranscriberConfig {
    pub base_url: String,

    pub model: String,
    pub auth: AuthSpec,
    pub timeout_ms: u64,

    pub proxying: kernel::Proxying,
}

pub struct Transcriber {
    attached: Option<Endpoint>,
}

impl Transcriber {
    #[must_use]
    pub fn absent() -> Transcriber {
        Transcriber { attached: None }
    }

    pub fn attach(
        config: TranscriberConfig,
        secrets: SecretResolver,
    ) -> Result<Transcriber, AxError> {
        let endpoint = Endpoint::new(
            EndpointConfig {
                base_url: join(&config.base_url, transcription_path()),

                dialect: DialectKind::OpenAi,
                model: config.model,
                auth: config.auth,
                extra_headers: Vec::new(),
                overrides: Vec::new(),
                timeout_ms: config.timeout_ms,
                stream_idle_timeout_ms: None,
                pricing: None,
                proxying: config.proxying,
            },
            Redemption::without_images(secrets),
        )?;
        Ok(Transcriber {
            attached: Some(endpoint),
        })
    }

    #[must_use]
    pub fn is_attached(&self) -> bool {
        self.attached.is_some()
    }

    pub fn transcribe(&self, recording: &Recording) -> Result<String, AxError> {
        let endpoint = self.attached.as_ref().ok_or_else(unconfigured)?;
        let body = form_body(endpoint.model(), recording);

        let wire: Value = endpoint
            .post_bytes(&body.content_type, body.bytes)
            .map_err(|err| err.rewrite_recovery("record again, or type the message instead"))?;
        transcription_of(&wire)
    }
}

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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
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
