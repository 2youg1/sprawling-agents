// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Attached or absent, one picture shown to the chosen model, and the
//! text read back (gateway-SPEC.md section 8-34).
//!
//! The attached state holds the adapter [`adapter_for`](crate::adapter_for)
//! builds for the chosen model, so a picture travels the path every other
//! picture in this city travels: named by its locator in the request,
//! fetched by that locator through the adapter's picture face, and
//! written on the wire by the dialect of the face the endpoint speaks.

use std::borrow::Cow;
use std::sync::{Arc, Mutex, MutexGuard};

use kernel::model::content_from_message;
use kernel::{
    AxCode, AxError, B3Hash, BuildingPolicy, Ceiling, ChatMessage, ChatRequest, ContentBlock,
    ImageRef, Model, ModelRequest, Payload, Role, SystemBlock,
};

use super::picture::Picture;

/// The one system block every picture is read under.
const INSTRUCTION: &str = "\
You read pictures for a coding agent. Write out all the text the picture \
shows, in reading order, keeping a line break wherever the picture starts \
a new line. Answer with that text and nothing else; when the picture shows \
no text, answer with nothing.";

/// What the user message beside the picture asks.
const ASK: &str = "Write out the text in this picture.";

/// The picture the call in flight shows, where the adapter's picture
/// face reads it. Empty between calls.
pub(super) type Shown = Arc<Mutex<Option<Picture>>>;

/// The city's facility for reading the text in a picture, which may not
/// exist.
///
/// `absent` is the ordinary state of a city nobody chose a model for
/// `ocr` in, and every call on it is answered by a refusal rather than
/// by silence.
pub struct Recogniser {
    attached: Option<Seeing>,
}

/// The chosen model, and what every request to it carries.
pub(super) struct Seeing {
    pub(super) model: Box<dyn Model + Send>,
    pub(super) model_id: String,
    /// The ceiling the choice was registered with.
    pub(super) max_tokens: Option<Ceiling>,
    pub(super) shown: Shown,
}

impl Recogniser {
    /// A city with no model chosen for `ocr`. Every `recognise` on it
    /// refuses by name.
    #[must_use]
    pub fn absent() -> Recogniser {
        Recogniser { attached: None }
    }

    pub(super) fn attached(seeing: Seeing) -> Recogniser {
        Recogniser {
            attached: Some(seeing),
        }
    }

    /// Whether this city can read a picture at all.
    #[must_use]
    pub fn is_attached(&self) -> bool {
        self.attached.is_some()
    }

    /// One picture, one call to the chosen model, the text it read.
    ///
    /// An empty answer is a real answer: a picture with no text in it
    /// reads as nothing.
    ///
    /// # Errors
    /// `E_TOOL_UNAVAILABLE` when no model is chosen; whatever the
    /// adapter refuses, a model registered as reading text only among
    /// it; `E_STORAGE_FATAL` when a call that died left the picture
    /// locked.
    pub fn recognise(
        &mut self,
        picture: Picture,
        policy: &BuildingPolicy,
    ) -> Result<String, AxError> {
        let seeing = self.attached.as_mut().ok_or_else(unconfigured)?;
        let request = seeing.request(picture.seen().clone(), policy);
        *held(&seeing.shown)? = Some(picture);
        let returned = seeing.model.call(&request);
        *held(&seeing.shown)? = None;
        returned.map(|_| String::new())
    }
}

impl Seeing {
    /// The request that shows `seen`: the instruction, then one user
    /// message holding the picture and what is asked of it. No tools: a
    /// reader asked for text has nothing to call.
    fn request(&self, seen: ImageRef, policy: &BuildingPolicy) -> ModelRequest<'static> {
        ModelRequest {
            policy: policy.clone(),
            // Not a run: there is no frozen prefix, so the four segment
            // slots carry the hash of the one block that stands where a
            // prefix would, as the adviser's do. Nothing writes this
            // request to a ledger; the text it brings back is what a
            // tool result records.
            segments: [B3Hash::digest(INSTRUCTION.as_bytes()); 4],
            chat: ChatRequest {
                model: self.model_id.clone(),
                max_tokens: self.max_tokens,
                system: vec![SystemBlock {
                    text: INSTRUCTION.to_owned(),
                    cache: false,
                }],
                messages: Cow::Owned(vec![ChatMessage {
                    role: Role::User,
                    content: vec![ContentBlock::Text {
                        text: format!("{ASK} {}", seen.locator),
                    }],
                }]),
                tools: Cow::Borrowed(&[]),
                breakpoint: kernel::MessageBreakpoint::Unmarked,
                effort: None,
            },
        }
    }
}

/// The text blocks of an answer, in order, one per line.
///
/// # Errors
/// Propagates an answer whose content does not read.
fn text_of(message: &Payload) -> Result<String, AxError> {
    let read: Vec<String> = content_from_message(message)?
        .into_iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } => Some(text),
            ContentBlock::Thinking { .. }
            | ContentBlock::RedactedThinking { .. }
            | ContentBlock::ToolUse { .. }
            | ContentBlock::Image(_)
            | ContentBlock::ToolResult { .. } => None,
        })
        .collect();
    Ok(read.join("\n"))
}

/// The picture slot, or the refusal a slot left locked by a call that
/// died is answered with.
pub(super) fn held(shown: &Shown) -> Result<MutexGuard<'_, Option<Picture>>, AxError> {
    shown.lock().map_err(|_| {
        AxError::failure(
            AxCode::StorageFatal,
            "show a picture",
            "the picture slot was left locked by a call that died",
        )
        .with_recovery(
            "end this run and resume it: a facility a thread died holding is not trusted \
             again inside this process",
        )
    })
}

/// What a city with no model chosen for `ocr` says when asked to read a
/// picture.
///
/// `E_TOOL_UNAVAILABLE` for the reason `transcribe` gives: the facility
/// is optional and this deployment does not have it, which is not
/// something a person filled in wrongly.
fn unconfigured() -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "read the text in a picture",
        "this city has no OCR model chosen",
    )
    .with_recovery(
        "attach an endpoint whose model reads pictures and choose it for `ocr`, or read the \
         window through its accessibility tree instead",
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
    reason = "test code"
)]
mod tests {
    use base64::Engine as _;
    use kernel::{Locator, UsdMicros};

    use super::*;
    use crate::endpoint::AuthSpec;
    use crate::endpoint::fakes::fake_provider;
    use crate::endpoint::redemption::resolver;
    use crate::market::{InputKinds, ModelEntry};
    use crate::provider::registry::ConnectionKind;
    use crate::router::{AttachedEndpoint, Chosen, EndpointTuning};

    /// What the stand-in endpoint read in every picture.
    const READ: &str = "KILN 1280";

    fn picture() -> Picture {
        let bytes = b"the pixels of a screenshot".to_vec();
        Picture::new(
            ImageRef {
                locator: Locator::cas(B3Hash::digest(&bytes)),
                media_type: kernel::ImageType::Png,
                width: 4,
                height: 2,
            },
            bytes,
        )
        .unwrap()
    }

    fn seeing_model() -> ModelEntry {
        ModelEntry {
            id: "eyes-1".to_owned(),
            context_tokens: 32_768,
            input: InputKinds::TextImage,
            max_output_tokens: Ceiling::new(1_024),
            input_price: UsdMicros::new(0),
            output_price: UsdMicros::new(0),
            cache_read_price: UsdMicros::new(0),
            cache_write_price: UsdMicros::new(0),
        }
    }

    /// The answer each face gives when it read [`READ`].
    fn answer(kind: ConnectionKind) -> String {
        match kind {
            ConnectionKind::OpenAiCompat => serde_json::json!({
                "choices": [{ "message": { "role": "assistant", "content": READ },
                              "finish_reason": "stop" }],
                "usage": { "prompt_tokens": 1, "completion_tokens": 1 },
            }),
            ConnectionKind::Responses => serde_json::json!({
                "status": "completed",
                "output": [{ "type": "message", "role": "assistant",
                             "content": [{ "type": "output_text", "text": READ }] }],
                "usage": { "input_tokens": 1, "output_tokens": 1 },
            }),
            ConnectionKind::AnthropicNative => serde_json::json!({
                "content": [{ "type": "text", "text": READ }],
                "stop_reason": "end_turn",
                "usage": { "input_tokens": 1, "output_tokens": 1 },
            }),
        }
        .to_string()
    }

    /// What one face was sent and what came back from it.
    #[derive(Debug, PartialEq)]
    struct Heard {
        path: String,
        model_named: bool,
        picture_carried: bool,
        read: String,
    }

    /// One picture shown through a stand-in that speaks `kind`.
    fn shown_on(kind: ConnectionKind) -> Heard {
        let (url, server) = fake_provider(vec![(200, answer(kind))], false);
        let endpoint = AttachedEndpoint {
            name: "eyes".to_owned(),
            base_url: url.trim_end_matches("/messages").to_owned(),
            dialect: kind.wire(),
            connection_kind: kind,
            auth: AuthSpec::None,
            models: Vec::new(),
            probed: false,
            tuning: EndpointTuning::default(),
        };
        let entry = seeing_model();
        let transport = crate::endpoint::Transport::default();
        let chosen = Chosen {
            endpoint: &endpoint,
            entry: &entry,
            transport: &transport,
        };
        let mut recogniser = recogniser_for_test(&chosen);
        let read = recogniser
            .recognise(picture(), &BuildingPolicy::default())
            .unwrap();
        let request = server.join().unwrap().remove(0);
        let encoded =
            base64::engine::general_purpose::STANDARD.encode(b"the pixels of a screenshot");
        Heard {
            path: request.split_whitespace().nth(1).unwrap().to_owned(),
            model_named: request.contains("\"eyes-1\""),
            picture_carried: request.contains(&encoded),
            read,
        }
    }

    fn recogniser_for_test(chosen: &Chosen<'_>) -> Recogniser {
        super::super::recogniser_for(chosen, resolver(), Vec::new()).unwrap()
    }

    /// The picture is written in the shape of the face the chosen
    /// endpoint speaks, to that face's path, for the model the person
    /// chose, and what the model read comes back.
    #[test]
    fn a_picture_is_shown_on_the_face_the_chosen_endpoint_speaks() {
        let kinds = [
            ConnectionKind::OpenAiCompat,
            ConnectionKind::Responses,
            ConnectionKind::AnthropicNative,
        ];
        let heard: Vec<Heard> = kinds.into_iter().map(shown_on).collect();
        let owed: Vec<Heard> = ["/v1/chat/completions", "/v1/responses", "/v1/messages"]
            .into_iter()
            .map(|path| Heard {
                path: path.to_owned(),
                model_named: true,
                picture_carried: true,
                read: READ.to_owned(),
            })
            .collect();
        assert_eq!(heard, owed);
    }

    #[test]
    fn a_city_with_no_ocr_model_refuses_by_name() {
        let mut absent = Recogniser::absent();
        assert!(!absent.is_attached());
        let err = absent
            .recognise(picture(), &BuildingPolicy::default())
            .unwrap_err();
        assert_eq!(
            (err.code(), err.action()),
            (&AxCode::ToolUnavailable, "read the text in a picture")
        );
        assert!(err.recovery().contains("`ocr`"), "{}", err.recovery());
    }
}
