// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::terminal::Inside;
use super::Session;

impl Session {
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
}
