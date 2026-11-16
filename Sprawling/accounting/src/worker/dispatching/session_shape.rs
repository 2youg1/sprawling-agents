// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxError, Effort};
use runtime::turn::CallShape;

use super::super::{Assignment, RunWorker};

#[cfg(test)]
mod tests;

impl RunWorker {
    pub(super) fn choose_shape(
        &mut self,
        at: &Assignment,
        model: &gateway::ModelEntry,
        provider: &str,
    ) -> Result<kernel::FrozenGeneration, AxError> {
        let endpoint = self
            .credentials
            .book
            .endpoints()
            .find(|endpoint| endpoint.name == provider);
        if let Some(endpoint) = endpoint {
            gateway::thinking_offer(endpoint, &model.id).admit(at.effort, &model.id)?;
        }
        let generation = match endpoint {
            Some(endpoint) => gateway::freeze_generation(endpoint, model)?,
            None => kernel::FrozenGeneration {
                max_tokens: model.max_output_tokens,
                ..kernel::FrozenGeneration::default()
            },
        };
        let own = city::own_layer(&self.city_root, &at.addr)?;
        let Some(frozen_model) = own.model() else {
            let mut shape = city::SessionShape::new(&model.id, at.effort, Some(generation.clone()));
            shape.endpoint = Some(provider.to_owned());
            city::write_session(&self.city_root, &at.addr, &shape)?;
            return Ok(generation);
        };

        let settled = city::settled_effort(&self.city_root, &at.addr)?.map(|(effort, _)| effort);
        let mut frozen = self.frozen_shape(frozen_model, own.endpoint(), settled, own.generation());
        let moved = CallShape {
            model: model.id.clone(),
            effort: at.effort.or(settled),
            generation,
            context_tokens: model.context_tokens,
        };

        if own.generation().is_none() {
            frozen.generation.overrides = moved.generation.overrides.clone();
        }

        moved.verified_against(&frozen)?;
        Ok(frozen.generation)
    }

    fn frozen_shape(
        &self,
        model: &str,
        endpoint: Option<&str>,
        effort: Option<Effort>,
        generation: Option<&kernel::FrozenGeneration>,
    ) -> CallShape {
        let held = self
            .credentials
            .book
            .choices()
            .find(|(_, provider, entry)| {
                entry.id == model && endpoint.is_none_or(|held| held == *provider)
            })
            .map(|(_, _, entry)| entry);
        CallShape {
            model: model.to_owned(),
            effort,
            generation: generation
                .cloned()
                .unwrap_or_else(|| kernel::FrozenGeneration {
                    max_tokens: held.and_then(|entry| entry.max_output_tokens),
                    ..kernel::FrozenGeneration::default()
                }),
            context_tokens: held.map_or(0, |entry| entry.context_tokens),
        }
    }
}
