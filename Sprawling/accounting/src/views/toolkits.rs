// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::prepared::LiveAsk;
use crate::toolkit_broker::broker_for;

impl LiveAsk {
    pub(super) fn toolkits_answer(&self) -> wire::ToolkitsAnswer {
        let held = match broker_for(self.vault.as_ref(), self.city.as_ref()) {
            Ok(Some(held)) => held,

            Ok(None) => return wire::ToolkitsAnswer::Unenrolled,
            Err(refusal) => {
                return wire::ToolkitsAnswer::Refused {
                    refusal: Box::new(refusal),
                };
            }
        };
        let (broker, user) = held;
        match broker.shelf(&user) {
            Ok(shelf) => wire::ToolkitsAnswer::Shelf {
                toolkits: shelf.into_iter().filter_map(line_of).collect(),
            },
            Err(refusal) => wire::ToolkitsAnswer::Refused {
                refusal: Box::new(refusal),
            },
        }
    }
}

fn line_of(toolkit: agent_protocols::Toolkit) -> Option<wire::ToolkitLine> {
    let standing = match toolkit.standing {
        agent_protocols::Connection::Absent => wire::Standing::Absent,
        agent_protocols::Connection::Awaiting { consent_url } => {
            wire::Standing::Awaiting { consent_url }
        }
        agent_protocols::Connection::Connected { alias } => wire::Standing::Connected { alias },
        agent_protocols::Connection::Refused { refusal } => wire::Standing::Refused {
            refusal: Box::new(refusal),
        },
    };
    Some(wire::ToolkitLine {
        slug: wire::ToolkitSlug::parse(&toolkit.slug).ok()?,
        name: toolkit.name,
        auth: toolkit.auth,
        standing,
    })
}
