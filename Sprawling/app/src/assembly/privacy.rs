// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The privacy page on a served city's listener
//! (`crates/sprawling/spec/Privacy/Service.lean`): `Query::Privacy` is
//! answered from the host and `Command::PrivacyOperation` carried out on
//! it, ahead of the views and the run worker, which hold nothing of the
//! host.

use std::sync::Arc;

use accounting::views::{Published, answer_outside_the_lock};
use accounting::worker::CommandDesk;
use kernel::AxError;

use crate::outside::asking::{Commands, Front};
use crate::privacy::service::Service;
use crate::privacy::system::System;

/// The privacy page of the machine this process runs on, or why there is
/// none: a person whose home cannot be found has no privacy history.
pub(super) type Page = Result<Arc<Service<System>>, AxError>;

pub(super) fn page() -> Page {
    Service::of_this_machine().map(Arc::new)
}

/// What the listener and the console answer with: the privacy page from
/// the host, every other query from the views.
pub(super) fn answering(views: Arc<Published>, page: Page) -> crate::console::Answering {
    Arc::new(move |query: wire::Query| {
        let (as_of, from_views) = answer_outside_the_lock(&views, &query);
        if matches!(query, wire::Query::Privacy) {
            let answer = page
                .as_ref()
                .map(|page| wire::Answer::Privacy(Box::new(page.answer())))
                .map_err(Clone::clone);
            (as_of, answer)
        } else {
            (as_of, from_views)
        }
    })
}

/// What the listener hands each Command to: a privacy operation to the
/// page, the remote door's verbs to `front`, everything else to `desk`.
pub(super) fn commands(front: &Front, desk: Arc<CommandDesk>, page: Page) -> Commands {
    front.commands(move |command: wire::WireCommand, reply: wire::Reply| {
        if let wire::Command::PrivacyOperation(wire::PrivacyRequest { action, idem }) = command {
            page.as_ref().map_err(Clone::clone)?.carry_out(action, idem)
        } else {
            desk.post(command.into(), reply);
            Ok(())
        }
    })
}
