// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the city's endpoints offer a console session
//! (`crates/sprawling/spec/Console.lean` §8-11): the arguments Tab may
//! put after `/model` and `/effort`, and what plain lines ask, as the
//! composer's settings row writes it. Each reads the attached endpoints
//! once, when asked, so an endpoint attached or removed shows at the next
//! Tab or the next line.

use super::super::terminal::Inside;
use super::Session;

impl Session {
    /// What Tab may put after `/model` or `/effort`, from one read of
    /// the attached endpoints: every model id served, or the thinking
    /// levels the current (Endpoint, model) offers. Any other verb, or a
    /// city that cannot answer, offers nothing.
    pub(crate) fn arguments(&self, inside: &Inside, verb: wire::Slash) -> Vec<String> {
        let book = || match (inside.answering)(wire::Query::EndpointView) {
            (_, Ok(wire::Answer::Endpoints(book))) => Some(book),
            (_, Ok(_) | Err(_)) => None,
        };
        match verb {
            wire::Slash::Model => {
                let Some(book) = book() else {
                    return Vec::new();
                };
                let mut ids: Vec<String> = book
                    .endpoints
                    .iter()
                    .flat_map(|endpoint| endpoint.models.iter().map(|model| model.id.clone()))
                    .collect();
                ids.sort();
                ids.dedup();
                ids
            }
            wire::Slash::Effort => book()
                .as_ref()
                .and_then(|book| self.current_offer(book))
                .map_or_else(Vec::new, |model| {
                    model
                        .thinking
                        .levels
                        .iter()
                        .map(|level| level.as_str().to_owned())
                        .collect()
                }),
            wire::Slash::Help
            | wire::Slash::Room
            | wire::Slash::New
            | wire::Slash::Stop
            | wire::Slash::Halt
            | wire::Slash::Release
            | wire::Slash::Approve
            | wire::Slash::Deny
            | wire::Slash::Web
            | wire::Slash::Quit
            | wire::Slash::Serving
            | wire::Slash::Remote
            | wire::Slash::Acp
            | wire::Slash::Wire => Vec::new(),
        }
    }

    /// What plain lines ask, as the composer's settings row writes it:
    /// the model `/model` chose or `main` names, and the level `/effort` chose.
    pub(crate) fn offer(&self, inside: &Inside) -> String {
        let main = || match (inside.answering)(wire::Query::EndpointView) {
            (_, Ok(wire::Answer::Endpoints(book))) => book
                .chosen
                .into_iter()
                .find(|chosen| chosen.tag == kernel::ModelTag::Main)
                .map(|chosen| chosen.model),
            (_, Ok(_) | Err(_)) => None,
        };
        let effort = self.effort.map(|level| level.as_str().to_owned());
        let said: Vec<String> = self
            .model
            .clone()
            .or_else(main)
            .into_iter()
            .chain(effort)
            .collect();
        said.join(" · ")
    }

    /// The model plain lines ask, on the endpoint that serves it: the
    /// one `/model` chose, looked for first where `main` points, else the
    /// one `main` names.
    fn current_offer<'a>(
        &self,
        book: &'a wire::EndpointsAnswer,
    ) -> Option<&'a wire::ModelFactsSummary> {
        let main = book
            .chosen
            .iter()
            .find(|chosen| chosen.tag == kernel::ModelTag::Main);
        let wanted = self
            .model
            .as_deref()
            .or(main.map(|chosen| chosen.model.as_str()))?;
        let served = |endpoint: &'a wire::EndpointSummary| {
            endpoint.models.iter().find(|model| model.id == wanted)
        };
        book.endpoints
            .iter()
            .filter(|endpoint| main.is_some_and(|chosen| chosen.endpoint == endpoint.name))
            .find_map(served)
            .or_else(|| book.endpoints.iter().find_map(served))
    }
}
