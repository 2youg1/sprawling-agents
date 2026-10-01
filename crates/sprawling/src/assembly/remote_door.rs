// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the remote door of a served city is made of (sprawling-SPEC.md
//! 8-139): the device table's path, the writer's relay for its five
//! lines, this machine's clock and random source, and where the city's
//! own listener answers on this machine.
//!
//! Assembled here because the clock is sampled in `bin::assembly` only,
//! and because the relay exists only once the writer thread runs. No
//! route is chosen yet: nothing reads a `[remote]` table, so `/remote
//! open` refuses and says so (sprawling-SPEC.md 8-139).

use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use kernel::{AxCode, AxError};

use crate::outside::console::Remote;
use crate::outside::keeper::{Doorway, Keeping, Senses};
use crate::outside::listener::Reaching;

/// The parts a remote door is kept with, gathered while the city opens.
pub(super) struct Outdoors {
    city_root: PathBuf,
    relay: accounting::worker::Relay,
    /// The address the city's own listener was bound to.
    city: SocketAddr,
    /// The pairing token that listener asks for, if it asks.
    token: Option<String>,
}

impl Outdoors {
    pub(super) fn new(
        city_root: &Path,
        relay: accounting::worker::Relay,
        city: SocketAddr,
        token: Option<String>,
    ) -> Outdoors {
        Outdoors {
            city_root: city_root.to_path_buf(),
            relay,
            city,
            token,
        }
    }

    /// The door this serve keeps, closed, with the devices paired before.
    ///
    /// # Errors
    /// An unreadable device table; a random source that refuses.
    pub(super) fn keep(self) -> Result<Remote, AxError> {
        let Outdoors {
            city_root,
            relay,
            city,
            token,
        } = self;
        let doorway = Doorway::keep(Keeping {
            devices: kernel::layout::CityLayout::new(&city_root).devices(),
            ledger: Box::new(relay),
            route: None,
            senses: Senses {
                clock: Arc::new(|| accounting::Clock::now(&super::SystemClock)),
                entropy: Arc::new(|bytes: &mut [u8]| {
                    getrandom::fill(bytes).map_err(|refused| {
                        AxError::failure(
                            AxCode::ConfigInvalid,
                            "draw randomness for the remote door",
                            refused.to_string(),
                        )
                        .with_recovery("this machine's random source refused; restart the city")
                    })
                }),
            },
        })?;
        // A city bound to every interface is reached on this machine at
        // loopback, where its relay connects from.
        let city = if city.ip().is_unspecified() {
            SocketAddr::from((Ipv4Addr::LOCALHOST, city.port()))
        } else {
            city
        };
        let runtime = tokio::runtime::Handle::try_current().map_err(|outside| {
            AxError::failure(
                AxCode::ConfigInvalid,
                "keep the remote door",
                outside.to_string(),
            )
            .with_recovery("report this: the door is kept from inside the serving runtime")
        })?;
        Ok(Remote {
            doorway,
            reaching: Reaching {
                runtime,
                city,
                token,
            },
        })
    }
}
