// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use city::{HostPermanence, RemoteRoute};
use kernel::{AxCode, AxError, TimeoutMs};
use remote_access::route::caddy::{Caddy, H3};
use remote_access::route::command::{CommandRoute, RouteCommand};
use remote_access::route::{Opened, Permanence, PublicUrl, Route};

use crate::outside::console::Remote;
use crate::outside::keeper::{CityKey, Doorway, Keeping, Senses};
use crate::outside::listener::Reaching;

const ROUTE_PATIENCE: TimeoutMs = TimeoutMs(30_000);

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

fn chosen(city_root: &Path) -> Result<Box<dyn Route + Send>, AxError> {
    let config = kernel::layout::CityLayout::new(city_root).city_config();
    Ok(match city::remote_route(city_root)? {
        RemoteRoute::H3 {
            command,
            url,
            listen,
            certificate,
            private_key,
        } => {
            let storage = config.with_file_name("remote-gateway");
            Box::new(Caddy::new(
                H3 {
                    program: PathBuf::from(command),
                    url: PublicUrl::parse(&url)?,
                    listen,
                    certificate: PathBuf::from(certificate),
                    private_key: PathBuf::from(private_key),
                    storage,
                },
                ROUTE_PATIENCE,
            )?)
        }
        RemoteRoute::Cloudflare { .. } => {
            return Err(public_gateway_required(
                &config,
                "route = \"cloudflare\"",
                "the configured route terminates at Cloudflare",
            ));
        }
        RemoteRoute::Command {
            command,
            args,
            permanence,
        } => Box::new(LoopbackCommand {
            route: CommandRoute::new(
                RouteCommand {
                    program: PathBuf::from(command),
                    args,
                },
                match permanence {
                    HostPermanence::Fixed => Permanence::Fixed,
                    HostPermanence::PerStart => Permanence::PerStart,
                },
                ROUTE_PATIENCE,
            ),
            config,
        }),
    })
}

struct LoopbackCommand {
    route: CommandRoute,
    config: PathBuf,
}

impl Route for LoopbackCommand {
    fn open(&mut self, local: SocketAddr) -> Result<Opened, AxError> {
        let opened = self.route.open(local)?;
        let url = reqwest::Url::parse(opened.url.as_str());
        let on_this_machine = url.as_ref().is_ok_and(|url| {
            url.host_str().is_some_and(|host| {
                host.eq_ignore_ascii_case("localhost")
                    || host
                        .trim_matches(['[', ']'])
                        .parse::<IpAddr>()
                        .is_ok_and(|ip| ip.is_loopback())
            })
        });
        if on_this_machine {
            return Ok(opened);
        }
        self.route.close()?;
        let reason = match url {
            Ok(_) => "the command returned an origin outside loopback".to_owned(),
            Err(source) => format!("the command returned an invalid origin: {source}"),
        };
        Err(public_gateway_required(
            &self.config,
            opened.url.as_str(),
            &reason,
        ))
    }

    fn close(&mut self) -> Result<(), AxError> {
        self.route.close()
    }
}

fn public_gateway_required(config: &Path, value: &str, reason: &str) -> AxError {
    AxError::failure(
        AxCode::ConfigInvalid,
        "open the remote door",
        format!("{}: [remote] value {value}: {reason}", config.display()),
    )
    .with_recovery(
        "in that [remote] table use route = \"h3\" for public access and set command (absolute Caddy path), url (HTTPS origin), listen (IP and UDP port), certificate (absolute PEM path), and private_key (absolute PEM path); see docs/operate/operating.md#choosing-the-route",
    )
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
            "route = \"h3\"",
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
        assert_eq!(command.0, AxCode::ToolUnavailable);
        assert!(command.1.contains("sprawling-absent-route"), "{command:?}");
        assert_eq!(cloudflare.0, AxCode::ConfigInvalid);
        assert!(cloudflare.1.contains("Cloudflare"), "{cloudflare:?}");
        assert!(cloudflare.2.contains("route = \"h3\""), "{cloudflare:?}");
    }
}
