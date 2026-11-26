// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use kernel::{AxCode, AxError};

use crate::outside::console::Remote;
use crate::outside::keeper::{CityKey, Doorway, Keeping, Senses};
use crate::outside::listener::Reaching;

pub(super) struct Outdoors {
    city_root: PathBuf,
    relay: accounting::worker::Relay,
    port: CityPort,

    key: Result<CityKey, AxError>,
}

pub(super) struct CityPort {
    pub(super) at: SocketAddr,

    pub(super) token: String,

    pub(super) page: Arc<wire::ClientAssets>,
}

impl Outdoors {
    pub(super) fn new(
        city_root: &Path,
        relay: accounting::worker::Relay,
        port: CityPort,
        key: Result<CityKey, AxError>,
    ) -> Outdoors {
        Outdoors {
            city_root: city_root.to_path_buf(),
            relay,
            port,
            key,
        }
    }

    pub(super) fn keep_aside(self, then: impl FnOnce(Result<Remote, AxError>) + Send + 'static) {
        let runtime = tokio::runtime::Handle::try_current();
        std::thread::spawn(move || {
            let _inside_the_runtime = runtime.as_ref().map(tokio::runtime::Handle::enter);
            then(self.keep());
        });
    }

    fn keep(self) -> Result<Remote, AxError> {
        let Outdoors {
            city_root,
            relay,
            port: CityPort { at, token, page },
            key,
        } = self;
        let doorway = Doorway::keep(Keeping {
            key: key?,
            devices: kernel::layout::CityLayout::new(&city_root).devices(),
            ledger: Box::new(relay),
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

        let city = if at.ip().is_unspecified() {
            SocketAddr::from((Ipv4Addr::LOCALHOST, at.port()))
        } else {
            at
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
                city_root,
                runtime,
                city,
                token,
                page,
            },
        })
    }
}
