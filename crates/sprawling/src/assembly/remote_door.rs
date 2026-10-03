// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the remote door of a served city is made of (`crates/sprawling/spec/Outside/Conduit.lean`
//! §8-139): the device table's path, the writer's relay for its five
//! lines, this machine's clock and random source, where the city's own
//! listener answers on this machine and the page it serves, and the
//! route the city's `[remote]` table names (`crates/sprawling/spec/Assembly.lean`
//! §8-151).
//!
//! Assembled here because the clock is sampled in `bin::assembly` only,
//! and because the relay exists only once the writer thread runs. The
//! city key arrives already named (`CityKey::of`, by the genesis line and
//! in the city's one vault), because the vault is handed out only there.
//!
//! [`chosen`] is the one place a route is built: the door asks it at
//! each `/remote open`, so a person who edits the table opens the door
//! on the new route without restarting the city.

use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use city::{HostPermanence, RemoteRoute};
use kernel::{AxCode, AxError, TimeoutMs};
use remote_access::route::cloudflare::{NamedTunnel, Tunnel, TunnelName};
use remote_access::route::command::{CommandRoute, RouteCommand};
use remote_access::route::{Permanence, PublicUrl, Route};

use crate::outside::console::Remote;
use crate::outside::keeper::{CityKey, Doorway, Keeping, Senses};
use crate::outside::listener::Reaching;

/// The program a Cloudflare route runs when the table names none: the
/// one on `PATH`.
const CLOUDFLARED: &str = "cloudflared";

/// The longest one `/remote open` waits for its route to be reachable.
/// The console's thread waits, and the door's lock is held, for as long
/// as this; `cloudflared` reaches its edge in one to a few seconds
/// (`crates/sprawling/spec/Assembly.lean` §8-151).
const ROUTE_PATIENCE: TimeoutMs = TimeoutMs(30_000);

/// The parts a remote door is kept with, gathered while the city opens.
pub(super) struct Outdoors {
    city_root: PathBuf,
    relay: accounting::worker::Relay,
    port: CityPort,
    /// Where the city's signing key lives, or why it has no name yet.
    key: Result<CityKey, AxError>,
}

/// The city's own listener, as the remote relay on this machine reaches
/// it and as the remote listener repeats its page.
pub(super) struct CityPort {
    /// The address that listener holds.
    pub(super) at: SocketAddr,
    /// The pairing token it asks for, if it asks.
    pub(super) token: Option<String>,
    /// The client bundle it serves, which the remote listener serves too.
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

    /// The door this serve keeps, closed, with the devices paired before.
    ///
    /// # Errors
    /// A ledger with no genesis line to name the city key by; an
    /// unreadable device table; a vault or a random source that refuses.
    pub(super) fn keep(self) -> Result<Remote, AxError> {
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
            choose: Box::new(move || chosen(&city_root)),
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
                runtime,
                city,
                token,
                page,
            },
        })
    }
}

/// The route the city's `[remote]` table names, built and closed.
///
/// # Errors
/// The city's own layer names no route or cannot be read; a tunnel name
/// or an address `remote_access` refuses.
fn chosen(city_root: &Path) -> Result<Box<dyn Route + Send>, AxError> {
    Ok(match city::remote_route(city_root)? {
        RemoteRoute::Cloudflare {
            tunnel,
            url,
            command,
        } => Box::new(NamedTunnel::new(
            Tunnel {
                program: PathBuf::from(command.as_deref().unwrap_or(CLOUDFLARED)),
                name: TunnelName::parse(&tunnel)?,
                url: PublicUrl::parse(&url)?,
            },
            ROUTE_PATIENCE,
        )),
        RemoteRoute::Command {
            command,
            args,
            permanence,
        } => Box::new(CommandRoute::new(
            RouteCommand {
                program: PathBuf::from(command),
                args,
            },
            match permanence {
                HostPermanence::Fixed => Permanence::Fixed,
                HostPermanence::PerStart => Permanence::PerStart,
            },
            ROUTE_PATIENCE,
        )),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::*;

    fn city_layer(city_root: &Path, text: &str) {
        let file = kernel::layout::CityLayout::new(city_root).city_config();
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    }

    /// What opening the route the city's file names comes to, as the
    /// three parts a person reads.
    fn opening(city_root: &Path) -> (AxCode, String, String) {
        let refused = chosen(city_root)
            .and_then(|mut route| route.open(SocketAddr::from((Ipv4Addr::LOCALHOST, 9))))
            .map(drop)
            .unwrap_err();
        (
            *refused.code(),
            refused.subject().to_owned(),
            refused.recovery().to_owned(),
        )
    }

    #[test]
    fn a_city_with_no_remote_table_is_refused_naming_the_table() {
        let dir = tempfile::tempdir().unwrap();
        let (code, subject, recovery) = opening(dir.path());
        let named = [
            "route = \"cloudflare\"",
            "route = \"command\"",
            "`tunnel`",
            "`url`",
            "`command`",
            "`permanence`",
        ]
        .iter()
        .all(|key| recovery.contains(key));
        assert!(
            code == AxCode::ConfigInvalid && subject.contains("[remote]") && named,
            "{code:?}: {subject}: {recovery}"
        );
    }

    /// Each arm builds its own route: the program a route could not
    /// start is the one the table named, and the recovery is that
    /// route's own; an address that is not `https://` is refused before
    /// anything starts.
    #[test]
    fn the_route_the_table_names_is_the_one_that_opens() {
        let dir = tempfile::tempdir().unwrap();
        city_layer(
            dir.path(),
            "[remote]\nroute = \"command\"\ncommand = \"sprawling-absent-route\"\n\
             permanence = \"fixed\"\n",
        );
        let command = opening(dir.path());
        city_layer(
            dir.path(),
            "[remote]\nroute = \"cloudflare\"\ntunnel = \"my-city\"\n\
             url = \"https://city.example.org\"\ncommand = \"sprawling-absent-cloudflared\"\n",
        );
        let cloudflare = opening(dir.path());
        city_layer(
            dir.path(),
            "[remote]\nroute = \"cloudflare\"\ntunnel = \"my-city\"\n\
             url = \"http://city.example.org\"\n",
        );
        let plain = opening(dir.path());
        assert_eq!(
            [
                (
                    command.0,
                    command.1.contains("sprawling-absent-route"),
                    command.2.contains("route command")
                ),
                (
                    cloudflare.0,
                    cloudflare.1.contains("sprawling-absent-cloudflared"),
                    cloudflare.2.contains("install cloudflared")
                ),
                (plain.0, plain.1.contains("not https"), true),
            ],
            [
                (AxCode::ToolUnavailable, true, true),
                (AxCode::ToolUnavailable, true, true),
                (AxCode::ConfigInvalid, true, true),
            ],
            "{command:?}\n{cloudflare:?}\n{plain:?}"
        );
    }
}
