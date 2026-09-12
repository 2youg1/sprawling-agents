// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Opening a city for service: what the vault turned out to be, and the
//! local server the environment names when nothing is registered yet.
//!
//! **The environment path is a convenience, not a second authority.**
//! It writes the same two records the settings page writes, through the
//! same two methods, so the book stays the only statement of what this
//! city can call and no reader has to ask which of two ways an endpoint
//! arrived.

use kernel::{AxError, EventKind, Payload};

use super::super::RunWorker;
use super::{Ceilings, Chosen, Credential, ENVIRONMENT_ENDPOINT, Entered, local_model_facts};

impl RunWorker {
    fn seed_from_environment(&mut self, base_url: &str, model: &str) -> Result<(), AxError> {
        let facts = local_model_facts(model)?;
        self.attach_endpoint(
            Entered {
                name: ENVIRONMENT_ENDPOINT.to_owned(),
                base_url: base_url.to_owned(),
                dialect: kernel::DialectKind::OpenAi,
                credential: Credential::Absent,
                tuning: gateway::EndpointTuning::default(),
            },
            &[],
        )?;
        self.select_model(
            Chosen {
                endpoint: ENVIRONMENT_ENDPOINT.to_owned(),
                model: model.to_owned(),
                tag: kernel::ModelTag::Main,
            },
            Ceilings {
                context_tokens: facts.context_tokens,
                max_output_tokens: facts.max_output_tokens,
            },
        )
    }

    /// Records what the vault turned out to be, and registers the local
    /// server named in the environment when nothing is registered yet.
    ///
    /// The environment path is a convenience, not a second authority:
    /// it writes the same two records the settings page writes, so the
    /// book stays the only statement of what this city can call. A
    /// failure here is reported and not fatal — a city stays readable
    /// without a provider.
    pub(crate) fn open_for_service(&mut self, vault_notice: Option<Payload>) {
        if let Some(notice) = vault_notice
            && let Err(err) = self.record(EventKind::ProviderDegraded, notice)
        {
            self.note(
                runtime::diagnostics::Level::Refuse,
                "bin::assembly",
                &format!("{err}; {}", err.recovery()),
            );
        }
        if !self.book.is_empty() {
            return;
        }
        let (Ok(base_url), Ok(model)) = (
            std::env::var("SPRAWLING_MODEL_URL"),
            std::env::var("SPRAWLING_MODEL"),
        ) else {
            return;
        };
        if let Err(err) = self.seed_from_environment(&base_url, &model) {
            self.note(
                runtime::diagnostics::Level::Refuse,
                "bin::assembly",
                &format!(
                    "the model named in the environment is not attached: {err}; {}",
                    err.recovery()
                ),
            );
        }
    }
}
