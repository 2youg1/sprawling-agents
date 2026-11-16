// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::Who;
use kernel::event::record::RemoteSessionStarted;
use kernel::{AxCode, AxError, EventKind};
use remote_access::door::{Authority, SessionId};
use remote_access::handshake::{self, CityWaiting, Finish, Hello, Reply, Session};
use remote_access::keys::VerifyingKey;

use super::{Doorway, drawn, nonce};

impl Doorway {

    pub(crate) fn session_reply(&self, hello: &Hello) -> Result<(Reply, CityWaiting), AxError> {
        let nonce = nonce(&self.senses)?;
        let kept = self.kept()?;
        kept.require_open("open a remote session")?;
        if kept.door.device(hello.device()).is_none() {
            return Err(unpaired());
        }
        handshake::city_reply(hello, kept.city()?, nonce)
    }

    pub(crate) fn admit(
        &self,
        waiting: CityWaiting,
        finish: &Finish,
    ) -> Result<(SessionId, Session), AxError> {
        let device = waiting.device();
        let id = SessionId::from_entropy(drawn(&self.senses)?);
        let now = (self.senses.clock)()?;
        let mut kept = self.kept()?;
        let key = kept
            .door
            .device(device)
            .map(|known| VerifyingKey::from_bytes(known.key.as_bytes()))
            .ok_or_else(unpaired)??;
        let session = waiting.accept(finish, &key)?;
        let expires = kept.require_open("open a remote session")?;
        kept.door.admit(id, device, expires)?;
        let line = RemoteSessionStarted {
            device: device.text(),
            expires,
        };
        kept.record(now, Who::Person, EventKind::RemoteSessionStarted, &line)?;
        Ok((id, session))
    }

    pub(crate) fn authority(&self, session: SessionId) -> Result<Option<Authority>, AxError> {
        let now = (self.senses.clock)()?;
        Ok(self.kept()?.door.authority(session, now))
    }
}

fn unpaired() -> AxError {
    AxError::failure(
        AxCode::GateDenied,
        "open a remote session",
        "an unpaired device",
    )
    .with_recovery("pair this device again from the city's console with `/remote pair <name>`")
}
