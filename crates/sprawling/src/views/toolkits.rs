// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which outside applications the broker offers, asked at the moment
//! somebody wants to know.
//!
//! Beside `mcp_health` rather than beside the folded answers, and for
//! the same reason: whether an account is connected is a fact about
//! now, and a remembered one would still say "connected" an hour after
//! the person revoked it. **This is the second read that costs
//! seconds**, and a page asks it when somebody opens the page or comes
//! back from the consent page they were sent to, never on a timer.
//!
//! Where the key lives, which proxy rule reaches the broker and who
//! this city is to it are all `assembly::toolkits`'s to answer, because
//! the command behind the button on this page has to agree with them.

use super::holding::Views;
use crate::assembly::broker_for;

impl Views {
    /// The shelf, or why there is no shelf to show.
    pub(super) fn toolkits_answer(&self) -> channels::ToolkitsAnswer {
        let held = match broker_for(self.vault.as_ref(), self.city.as_ref()) {
            Ok(Some(held)) => held,
            // No key enrolled is the first step rather than a failure.
            // Drawing it as an error would tell somebody opening this
            // page for the first time that something broke.
            Ok(None) => return channels::ToolkitsAnswer::Unenrolled,
            Err(refusal) => {
                return channels::ToolkitsAnswer::Refused {
                    refusal: Box::new(refusal),
                };
            }
        };
        let (broker, user) = held;
        match broker.shelf(&user) {
            Ok(shelf) => channels::ToolkitsAnswer::Shelf {
                toolkits: shelf.into_iter().filter_map(line_of).collect(),
            },
            Err(refusal) => channels::ToolkitsAnswer::Refused {
                refusal: Box::new(refusal),
            },
        }
    }
}

/// One row, in the vocabulary a page draws rather than the broker's.
///
/// A slug this crate will not carry drops the row instead of drawing it
/// under a placeholder: the only thing a person can do with such a row
/// is press it, and pressing it could not name what to connect.
fn line_of(toolkit: gateway::Toolkit) -> Option<channels::ToolkitLine> {
    let standing = match toolkit.standing {
        gateway::Connection::Absent => channels::Standing::Absent,
        gateway::Connection::Awaiting { consent_url } => {
            channels::Standing::Awaiting { consent_url }
        }
        gateway::Connection::Connected { alias } => channels::Standing::Connected { alias },
        gateway::Connection::Refused { refusal } => channels::Standing::Refused {
            refusal: Box::new(refusal),
        },
    };
    Some(channels::ToolkitLine {
        slug: channels::ToolkitSlug::parse(&toolkit.slug).ok()?,
        name: toolkit.name,
        auth: toolkit.auth,
        standing,
    })
}
