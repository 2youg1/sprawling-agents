// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::net::SocketAddr;
use std::sync::Mutex;

use browser::OwnListeners;

#[derive(Clone, PartialEq, Eq)]
struct Entrance {
    address: SocketAddr,
    origin: Option<String>,
}

static LISTENING: Mutex<Vec<Entrance>> = Mutex::new(Vec::new());

#[must_use = "the address is taken back when this is dropped"]
pub(crate) struct Served(Entrance);

pub(crate) fn serve(address: SocketAddr) -> Served {
    register(Entrance {
        address,
        origin: None,
    })
}

pub(crate) fn serve_public(address: SocketAddr, origin: &reqwest::Url) -> Served {
    register(Entrance {
        address,
        origin: Some(origin.origin().ascii_serialization()),
    })
}

fn register(entrance: Entrance) -> Served {
    LISTENING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(entrance.clone());
    Served(entrance)
}

impl Drop for Served {
    fn drop(&mut self) {
        let mut listening = LISTENING
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(index) = listening.iter().position(|at| *at == self.0) {
            listening.swap_remove(index);
        }
    }
}

pub(super) fn own() -> OwnListeners {
    let listening = LISTENING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    OwnListeners::new(listening.iter().map(|door| door.address))
        .with_origins(listening.iter().filter_map(|door| door.origin.clone()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn the_public_door_guards_its_host_and_bound_port_until_closed() {
        let bound = "127.0.0.1:45432".parse().unwrap();
        let origin = reqwest::Url::parse("https://city.example").unwrap();
        let door = serve_public(bound, &origin);
        let listeners = own();
        assert!(listeners.holds("https://city.example:443/#/city"));
        assert!(listeners.holds("https://localhost:45432/"));
        assert!(!listeners.holds("https://work.example/"));
        assert!(!listeners.holds("https://localhost:45433/"));

        drop(door);
        assert!(!own().holds("https://city.example:443/#/city"));
        assert!(!own().holds("https://localhost:45432/"));
    }
}
