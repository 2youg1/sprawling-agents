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
//! asked for. The ladder for the window and the ceiling is
//! `gateway::Figures::resolve`, called here with the facts this city
//! holds, so a run cannot be called with a figure invented at the call
//! site and a query about a model cannot answer with a different one.

use kernel::event::EventKind;
use kernel::{AxCode, AxError};

use crate::worker::RunWorker;
use crate::worker::credentials::{Chosen, Stated, registered_as};

impl RunWorker {
    /// Points one tag at one model.
    ///
    /// **A figure the person left empty is not a figure they erased.**
    /// The settings page sends the whole row on every pick, so re-picking
    /// an already registered model with an empty box must not overwrite
    /// its ceiling with nothing: the next call on the Anthropic wire
    /// would be refused for a field it could not write (`crates/sprawling/Spec.lean`
    /// §8-71). An empty box leaves the figure to the ladder in
    /// `gateway::Figures::resolve`, which this method hands the facts it
    /// holds: what the person sent, the provider's own row for this id in
    /// the book, and what this same model was registered with, read back
    /// by `registered_as`.
    pub(in crate::worker) fn select_model(
        &mut self,
        chosen: Chosen,
        stated: Stated,
    ) -> Result<(), AxError> {
        let Chosen {
            endpoint,
            model,
            tag,
        } = chosen;
        let Stated {
            context_tokens,
            max_output_tokens,
            input: stated_input,
        } = stated;
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
        let Some(listed) = known.models.iter().find(|row| row.id == model) else {
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
        };
        let catalogue = gateway::MarketSnapshot::builtin()?;
        let priced = catalogue.lookup(&model);
        let registered = registered_as(&self.credentials.book, tag, &endpoint, &model);
        let figures = gateway::Figures::resolve(
            gateway::Statements {
                window: context_tokens,
                ceiling: max_output_tokens,
                listed: Some(listed),
                registered: registered.as_ref(),
            },
            priced,
            gateway::Target {
                base_url: &known.base_url,
                id: &model,
                wire: known.dialect,
            },
        );
        // What a model accepts is what the person said, else the
        // vendor's fact read off the catalogue or the preset table
        // (`crates/gateway/Spec.lean` §8-37, gateway D16).
        let input = gateway::accepted_input(
            stated_input,
            priced.map(|row| row.input),
            &known.base_url,
            &model,
        );
        // Prices come from the pinned catalog when it knows the model
        // and are zero when it does not: an unpriced call is reported as
        // unpriced rather than as free-looking guesswork.
        let price =
            |of: fn(&gateway::ModelEntry) -> kernel::UsdMicros| priced.map(of).unwrap_or_default();
        let entry = gateway::ModelEntry {
            id: model,
            // The catalogue row carries a plain figure; a window nobody
            // stated is zero there and `None` on the wire, and this is
            // the one place the two spellings meet.
            context_tokens: figures.window.map_or(0, |window| window.tokens.get()),
            window_from: figures.window.map(|window| window.from),
            max_output_tokens: figures.ceiling.and_then(gateway::OutputCeiling::tokens),
            ceiling_from: figures.ceiling.and_then(gateway::OutputCeiling::source),
            input,
            input_price: price(|row| row.input_price),
            output_price: price(|row| row.output_price),
            cache_read_price: price(|row| row.cache_read_price),
            cache_write_price: price(|row| row.cache_write_price),
        };
        let payload = gateway::selected_payload(tag, &endpoint, &entry)?;
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
            facts_blob: None,
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
                input: None,
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

    /// The person's statement is the first rung: a model the pinned
    /// catalogue lists as reading text, on a server no preset row speaks
    /// for, is registered as reading pictures when the person says so
    /// (`crates/gateway/Spec.lean` §8-37, gateway D16).
    #[test]
    fn a_model_the_person_says_reads_pictures_is_registered_as_reading_them() {
        let dir = tempfile::tempdir().unwrap();
        crate::worker::fixture::init_city(dir.path()).unwrap();
        let mut worker = RunWorker::new(
            dir.path(),
            runtime::diagnostics::Diagnostics::off(),
            crate::worker::fixture::hands(),
        )
        .unwrap();
        let attached = kernel::event::record::EndpointAttached {
            name: "here".to_owned(),
            base_url: "http://127.0.0.1:8000/v1".to_owned(),
            dialect: kernel::DialectKind::OpenAi,
            auth: None,
            auth_header: None,
            models: vec!["local".to_owned()],
            connection_kind: None,
            probed: false,
            tuning: None,
            facts_blob: None,
        };
        worker
            .record(EventKind::EndpointAttached, Payload::of(&attached).unwrap())
            .unwrap();
        assert_eq!(
            gateway::MarketSnapshot::builtin()
                .unwrap()
                .lookup("local")
                .map(|row| row.input),
            Some(gateway::InputKinds::Text),
            "the catalogue says text"
        );
        worker
            .handle(wire::Command::SelectModel {
                endpoint: wire::ProviderName::parse("here").unwrap(),
                model: "local".to_owned(),
                tag: kernel::ModelTag::Ocr,
                context_tokens: None,
                max_output_tokens: None,
                input: Some(kernel::InputKinds::TextImage),
                idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"stated"),
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
