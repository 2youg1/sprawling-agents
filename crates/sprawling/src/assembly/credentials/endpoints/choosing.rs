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

use crate::assembly::RunWorker;
use crate::assembly::credentials::{Ceilings, Chosen, registered_as};

impl RunWorker {
    /// Points one tag at one model.
    ///
    /// **A figure the person left empty is not a figure they erased.**
    /// The settings page sends the whole row on every pick, so re-picking
    /// an already registered model used to overwrite its ceiling with
    /// nothing, and the next call on the Anthropic wire was refused for a
    /// field it could no longer write (sprawling-SPEC.md 8-71). An empty
    /// box now keeps what this same model was registered with, read back
    /// by `registered_as`.
    pub(in crate::assembly) fn select_model(
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
        let registered = registered_as(&self.book, tag, &endpoint, &model);
        // What the person stated outranks what they stated before, which
        // outranks the catalogue, and what none of the three states
        // stays unstated. **The old reading of an unknown model was
        // zero**, which the OpenAI wire wrote out as `max_tokens: 0` and
        // a provider answered with no content at all; the run then froze
        // as work that finished. A ceiling this city cannot name is now
        // carried as one it cannot name.
        let context_tokens = context_tokens.or_else(|| {
            registered
                .as_ref()
                .and_then(|row| kernel::Window::new(row.context_tokens))
                .or_else(|| {
                    priced
                        .as_ref()
                        .and_then(|row| kernel::Window::new(row.context_tokens))
                })
        });
        // The ladder answers, and says which rung answered. The person's
        // figure outranks the one they entered before, which outranks
        // the pinned catalogue and the preset table, and the policy
        // default is the last rung rather than a number invented at the
        // call site (gateway-SPEC.md 8-17). Because the ladder always
        // answers, a model no catalogue knows can still be called on the
        // Anthropic wire, which is what B-01 was.
        let resolved = gateway::OutputCeiling::resolve(
            gateway::Stated {
                person: max_output_tokens,
                upstream: registered.as_ref().and_then(|row| row.max_output_tokens),
            },
            priced.as_ref().and_then(|row| row.max_output_tokens),
            &known.base_url,
            &model,
        );
        let max_output_tokens = resolved.map(gateway::OutputCeiling::tokens);
        let ceiling_from = resolved.map(gateway::OutputCeiling::source);
        let entry = gateway::ModelEntry {
            id: model,
            // The catalogue row carries a plain figure; a window nobody
            // stated is zero there and `None` on the wire, and this is
            // the one place the two spellings meet.
            context_tokens: context_tokens.map_or(0, kernel::Window::get),
            max_output_tokens,
            // What a model accepts is the catalogue's fact, not a
            // person's: a form cannot make a text-only model see.
            input: priced.as_ref().map(|row| row.input).unwrap_or_default(),
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
        let payload = gateway::selected_payload(tag, &endpoint, &entry, ceiling_from)?;
        self.record(EventKind::ModelSelected, payload)
    }
}
