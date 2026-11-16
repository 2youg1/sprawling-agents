// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::{Arc, Mutex};

use kernel::{AxError, EventKind, Payload};

pub(in crate::worker) struct Credentials {

    pub(in crate::worker) book: gateway::EndpointBook,

    pub(in crate::worker) vault: Arc<Mutex<gateway::Custodian>>,
}

impl Credentials {

    pub(in crate::worker) fn opened(
        book: gateway::EndpointBook,
        vault: gateway::Custodian,
    ) -> Credentials {
        Credentials {
            book,
            vault: Arc::new(Mutex::new(vault)),
        }
    }

    pub(in crate::worker) fn absorb(
        &mut self,
        kind: EventKind,
        run: kernel::RunId,
        addr: Option<&kernel::Address>,
        data: &Payload,
    ) -> Result<(), AxError> {
        self.book.absorb(kind, run, addr, data)
    }
}
