// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Choosing which model one tag points at, and the ceiling that call
//! is made with.
//!
//! Separate from attaching because the two fail for different reasons:
//! an attach fails when a machine cannot be reached, and a choice fails
//! when the endpoint this city already reached does not serve what was
//! asked for. The ceiling ladder runs here and nowhere else, so a run
//! cannot be called with a figure invented at the call site.

use kernel::event::EventKind;
use kernel::{AxCode, AxError};

use crate::worker::RunWorker;
use crate::worker::credentials::{Ceilings, Chosen, registered_as};

impl RunWorker {
    /// Points one tag at one model.
    ///
    /// **A figure the person left empty is not a figure they erased.**
    /// The settings page sends the whole row on every pick, so re-picking
    /// an already registered model with an empty box must not overwrite
    /// its ceiling with nothing: the next call on the Anthropic wire
    /// would be refused for a field it could not write (sprawling-SPEC.md
    /// 8-71). An empty box keeps what this same model was registered
    /// with, read back by `registered_as`.
    pub(in crate::worker) fn select_model(
        &mut self,
        chosen: Chosen,
        ceilings: Ceilings,
    ) -> Result<(), AxError> {
        let Chosen {
            endpoint,
            model,
            tag,
        } = chosen;
        let Ceilings {
            context_tokens,
            max_output_tokens,
        } = ceilings;
        let known = self
            .credentials
            .book
            .endpoints()
            .find(|candidate| candidate.name == endpoint)
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::ConfigInvalid,
                    "choose a model",
                    format!("{endpoint} is not attached"),
                )
                .with_recovery("attach the endpoint first, then choose one of the models it lists")
            })?;
        if !known.models.iter().any(|row| row.id == model) {
            return Err(AxError::failure(
                AxCode::ConfigInvalid,
                "choose a model",
                format!("{endpoint} does not serve {model}"),
            )
            .with_nearby(
                known
                    .models
                    .iter()
                    .map(|row| row.id.clone())
                    .collect::<Vec<String>>(),
            )
            .with_recovery("choose one of the models the endpoint listed"));
        }
        let priced = gateway::MarketSnapshot::builtin()?.lookup(&model).cloned();
        let registered = registered_as(&self.credentials.book, tag, &endpoint, &model);
        // What the person stated outranks what they stated before, which
        // outranks the catalogue, which outranks the vendor's documented
        // window, and what none of the four states stays unstated. **An
        // unknown window is not zero**: a window this city cannot name
        // is carried as one it cannot name, and the context reminder
        // stays quiet rather than counting against a guess.
        let context_tokens = context_tokens.or_else(|| {
            registered
                .as_ref()
                .and_then(|row| kernel::Window::new(row.context_tokens))
                .or_else(|| {
                    priced
                        .as_ref()
                        .and_then(|row| kernel::Window::new(row.context_tokens))
                })
                .or_else(|| gateway::window_for(&known.base_url, &model))
        });
        // The ladder answers, and says which rung answered. The person's
        // figure outranks the one they entered before; on the messages
        // face, which needs a figure in every request, the pinned
        // catalogue, the preset table and the policy default follow, and
        // on the chat and responses faces nobody's statement leaves the
        // figure to the provider (`crates/gateway/Spec.lean` §8-17).
        let resolved = gateway::OutputCeiling::resolve(
            gateway::Stated {
                person: max_output_tokens,
                upstream: registered.as_ref().and_then(|row| row.max_output_tokens),
            },
            priced.as_ref().and_then(|row| row.max_output_tokens),
            gateway::Target {
                base_url: &known.base_url,
                id: &model,
                wire: known.dialect,
            },
        );
        let max_output_tokens = resolved.and_then(gateway::OutputCeiling::tokens);
        // What a model accepts is the vendor's fact, read off the
        // catalogue or the preset table; a form cannot make a text-only
        // model see (`crates/gateway/Spec.lean` §8-37).
        let input = gateway::accepted_input(
            priced.as_ref().map(|row| row.input),
            &known.base_url,
            &model,
        );
        let entry = gateway::ModelEntry {
            id: model,
            // The catalogue row carries a plain figure; a window nobody
            // stated is zero there and `None` on the wire, and this is
            // the one place the two spellings meet.
            context_tokens: context_tokens.map_or(0, kernel::Window::get),
            max_output_tokens,
            input,
            // Prices come from the pinned catalog when it knows the
            // model and are zero when it does not: an unpriced call is
            // reported as unpriced rather than as free-looking guesswork.
            input_price: priced
                .as_ref()
                .map(|row| row.input_price)
                .unwrap_or_default(),
            output_price: priced
                .as_ref()
                .map(|row| row.output_price)
                .unwrap_or_default(),
            cache_read_price: priced
                .as_ref()
                .map(|row| row.cache_read_price)
                .unwrap_or_default(),
            cache_write_price: priced.map(|row| row.cache_write_price).unwrap_or_default(),
        };
        let payload = gateway::selected_payload(tag, &endpoint, &entry, resolved)?;
        self.record(EventKind::ModelSelected, payload)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use kernel::event::EventKind;
    use kernel::{Payload, RunId};

    use crate::worker::RunWorker;

    /// A model the pinned catalogue does not know, whose vendor's page says
    /// it reads pictures, is registered as reading them, so the endpoint
    /// shows it the picture ocr hands it instead of refusing
    /// (`crates/gateway/Spec.lean` §8-37).
    #[test]
    fn a_preset_model_that_reads_pictures_is_registered_as_reading_them() {
        let dir = tempfile::tempdir().unwrap();
        crate::worker::fixture::init_city(dir.path()).unwrap();
        let mut worker = RunWorker::new(
            dir.path(),
            runtime::diagnostics::Diagnostics::off(),
            crate::worker::fixture::hands(),
        )
        .unwrap();
        let attached = kernel::event::record::EndpointAttached {
            name: "vendor".to_owned(),
            base_url: "https://api.anthropic.com/v1".to_owned(),
            dialect: kernel::DialectKind::Anthropic,
            auth: None,
            auth_header: None,
            models: vec!["claude-sonnet-4-5".to_owned()],
            connection_kind: None,
            probed: false,
            tuning: None,
        };
        worker
            .record(EventKind::EndpointAttached, Payload::of(&attached).unwrap())
            .unwrap();
        worker
            .handle(wire::Command::SelectModel {
                endpoint: wire::ProviderName::parse("vendor").unwrap(),
                model: "claude-sonnet-4-5".to_owned(),
                tag: kernel::ModelTag::Ocr,
                context_tokens: None,
                max_output_tokens: None,
                idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"select"),
            })
            .unwrap();
        let registered: Vec<(kernel::ModelTag, gateway::InputKinds)> = worker
            .credentials
            .book
            .choices()
            .map(|(tag, _, entry)| (tag, entry.input))
            .collect();
        assert_eq!(
            registered,
            vec![(kernel::ModelTag::Ocr, gateway::InputKinds::TextImage)]
        );
    }
}
