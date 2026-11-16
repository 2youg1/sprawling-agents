// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::net::{IpAddr, SocketAddr};

use kernel::{AxCode, AxError};

use super::Door;

const SAME_ORIGIN: &str = "same-origin";

const DEFAULT_HTTP_PORT: u16 = 80;

const DEFAULT_HTTPS_PORT: &str = ":443";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListenerOrigins {
    hosts: Hosts,
    url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Hosts {
    Named {
        names: Vec<String>,
        origins: Vec<String>,
    },

    AnyAddress {
        port: u16,
        loopback: Vec<String>,
    },
}

impl ListenerOrigins {
    #[must_use]
    pub fn of(local: SocketAddr) -> Self {
        let port = local.port();
        let loopback = || {
            ["127.0.0.1", "localhost", "[::1]"]
                .into_iter()
                .flat_map(|name| spelled(name, port))
                .collect::<Vec<_>>()
        };
        let ip = local.ip();
        let (hosts, url_host) = if ip.is_loopback() {
            let shown = match ip {
                IpAddr::V4(_) => "127.0.0.1".to_owned(),
                IpAddr::V6(_) => "[::1]".to_owned(),
            };
            (Hosts::named(loopback()), shown)
        } else if ip.is_unspecified() {
            (
                Hosts::AnyAddress {
                    port,
                    loopback: loopback(),
                },
                "127.0.0.1".to_owned(),
            )
        } else {
            let name = literal(ip);
            (Hosts::named(spelled(&name, port)), name)
        };
        Self {
            hosts,
            url: format!("http://{url_host}:{port}"),
        }
    }

    #[must_use]
    pub fn routed(local: SocketAddr, public: &str) -> Self {
        let mut listener = Self::of(local);
        if let (Some(name), Hosts::Named { names, origins }) =
            (public_name(public), &mut listener.hosts)
        {
            origins.push(format!("https://{name}"));
            names.push(name);
        }
        listener
    }

    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    #[must_use]
    pub fn admits_host(&self, host: &str) -> bool {
        match &self.hosts {
            Hosts::Named { names, .. } => names.iter().any(|name| name.eq_ignore_ascii_case(host)),
            Hosts::AnyAddress { port, loopback } => {
                loopback.iter().any(|name| name.eq_ignore_ascii_case(host))
                    || ip_literal_on(host, *port)
            }
        }
    }

    #[must_use]
    pub fn admits_origin(&self, origin: &str, host: &str) -> bool {
        match &self.hosts {
            Hosts::Named { origins, .. } => origins
                .iter()
                .any(|listed| listed.eq_ignore_ascii_case(origin)),
            Hosts::AnyAddress { .. } => origin
                .strip_prefix("http://")
                .is_some_and(|named| named.eq_ignore_ascii_case(host) && self.admits_host(named)),
        }
    }
}

impl Hosts {
    fn named(names: Vec<String>) -> Self {
        let origins = names.iter().map(|name| format!("http://{name}")).collect();
        Self::Named { names, origins }
    }
}

fn public_name(public: &str) -> Option<String> {
    let (scheme, rest) = public.split_once("://")?;
    let authority = rest.split(['/', '?']).next()?.to_ascii_lowercase();
    let name = authority
        .strip_suffix(DEFAULT_HTTPS_PORT)
        .map_or(authority.as_str(), |bare| bare)
        .to_owned();
    (scheme.eq_ignore_ascii_case("https") && !name.is_empty()).then_some(name)
}

fn spelled(name: &str, port: u16) -> Vec<String> {
    let mut names = vec![format!("{name}:{port}")];
    if port == DEFAULT_HTTP_PORT {
        names.push(name.to_owned());
    }
    names
}

fn literal(ip: IpAddr) -> String {
    match ip {
        IpAddr::V4(v4) => v4.to_string(),
        IpAddr::V6(v6) => format!("[{v6}]"),
    }
}

fn ip_literal_on(host: &str, port: u16) -> bool {
    let address = host.parse::<SocketAddr>().ok().or_else(|| {
        (port == DEFAULT_HTTP_PORT)
            .then(|| {
                host.trim_start_matches('[')
                    .trim_end_matches(']')
                    .parse::<IpAddr>()
                    .ok()
            })
            .flatten()
            .map(|ip| SocketAddr::new(ip, port))
    });
    address.is_some_and(|address| address.port() == port && !address.ip().is_unspecified())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageHeaders {
    lines: Vec<(&'static str, String)>,
}

impl PageHeaders {
    #[must_use]
    pub fn every_listener() -> Self {
        let policy = "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; \
                      style-src 'self' 'unsafe-inline'; connect-src 'self'; \
                      img-src 'self' data: blob:; worker-src 'self' blob:; object-src 'none'; \
                      base-uri 'none'; form-action 'none'; frame-ancestors 'none'";
        Self {
            lines: vec![
                ("content-security-policy", policy.to_owned()),
                ("x-frame-options", "DENY".to_owned()),
                ("x-content-type-options", "nosniff".to_owned()),
                ("referrer-policy", "no-referrer".to_owned()),
                ("cross-origin-opener-policy", "same-origin".to_owned()),
                ("cross-origin-resource-policy", "same-origin".to_owned()),
            ],
        }
    }

    #[must_use]
    pub fn lines(&self) -> &[(&'static str, String)] {
        &self.lines
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Presented<'a> {
    pub host: Option<&'a str>,
    pub origin: Option<&'a str>,
    pub sec_fetch_site: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrival {
    Page,

    Preflight,

    Socket,

    Door(Door),

    Pairing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Caller {
    Native,

    Browser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entry {
    Page,

    Caller(Caller),

    Refused(EntryRefusal),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryRefusal {
    ForeignHost,
    Preflight,
    ForeignOrigin,
    CrossSite,
    NativePairing,
}

impl EntryRefusal {
    #[must_use]
    pub fn error(self) -> AxError {
        let (subject, recovery) = match self {
            Self::ForeignHost => (
                "the request names a host this listener is not",
                "open the address the city printed when it started",
            ),
            Self::Preflight => (
                "this city answers no cross-origin request",
                "send the request from the city's own page, or from a program on this machine",
            ),
            Self::ForeignOrigin => (
                "the request comes from a page this city did not serve",
                "open the address the city printed when it started",
            ),
            Self::CrossSite => (
                "the browser says the request comes from another site",
                "open the address the city printed when it started",
            ),
            Self::NativePairing => (
                "pairing is for a browser page",
                "a program on this machine presents the city's key from its key file instead",
            ),
        };
        AxError::failure(AxCode::GateDenied, "enter the city's door", subject)
            .with_recovery(recovery)
    }
}

#[must_use]
pub fn decide_entry(
    origins: &ListenerOrigins,
    presented: Presented<'_>,
    arrival: Arrival,
) -> Entry {
    let Some(host) = presented.host.filter(|host| origins.admits_host(host)) else {
        return Entry::Refused(EntryRefusal::ForeignHost);
    };
    let knocking = match arrival {
        Arrival::Page => return Entry::Page,
        Arrival::Preflight => return Entry::Refused(EntryRefusal::Preflight),
        Arrival::Socket | Arrival::Door(_) | Arrival::Pairing => arrival,
    };
    let caller = match presented.origin {
        None => Caller::Native,
        Some(origin) if origins.admits_origin(origin, host) => Caller::Browser,
        Some(_) => return Entry::Refused(EntryRefusal::ForeignOrigin),
    };
    if presented
        .sec_fetch_site
        .is_some_and(|site| !site.eq_ignore_ascii_case(SAME_ORIGIN))
    {
        return Entry::Refused(EntryRefusal::CrossSite);
    }
    match (knocking, caller) {
        (Arrival::Pairing, Caller::Native) => Entry::Refused(EntryRefusal::NativePairing),
        (
            Arrival::Page
            | Arrival::Preflight
            | Arrival::Socket
            | Arrival::Door(_)
            | Arrival::Pairing,
            Caller::Native | Caller::Browser,
        ) => Entry::Caller(caller),
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    fn loopback() -> ListenerOrigins {
        ListenerOrigins::of("127.0.0.1:2501".parse().unwrap())
    }

    const ARRIVALS: [Arrival; 8] = [
        Arrival::Page,
        Arrival::Preflight,
        Arrival::Socket,
        Arrival::Door(Door::Transcribe),
        Arrival::Door(Door::Enroll),
        Arrival::Door(Door::Acp),
        Arrival::Door(Door::Drop),
        Arrival::Pairing,
    ];

    #[test]
    fn every_entry_property_holds_over_every_input() {
        let origins = loopback();
        let hosts = [
            (Some("127.0.0.1:2501"), true),
            (Some("localhost:2501"), true),
            (Some("evil.example:2501"), false),
            (Some("0.0.0.0:2501"), false),
            (None, false),
        ];
        let seen = [
            (None, "absent"),
            (Some("http://127.0.0.1:2501"), "listed"),
            (Some("http://localhost:2501"), "listed"),
            (Some("http://127.0.0.1:5173"), "foreign"),
            (Some("null"), "foreign"),
            (Some("https://127.0.0.1:2501"), "foreign"),
        ];
        let sites = [
            (None, false),
            (Some("same-origin"), false),
            (Some("same-site"), true),
            (Some("cross-site"), true),
            (Some("none"), true),
        ];
        for (host, listed) in hosts {
            for (origin, class) in seen {
                for (sec_fetch_site, other) in sites {
                    for arrival in ARRIVALS {
                        let presented = Presented {
                            host,
                            origin,
                            sec_fetch_site,
                        };
                        let entry = decide_entry(&origins, presented, arrival);
                        let case = format!("{presented:?} at {arrival:?}: {entry:?}");
                        if !listed {
                            assert_eq!(entry, Entry::Refused(EntryRefusal::ForeignHost), "{case}");
                            continue;
                        }
                        let entered = matches!(entry, Entry::Caller(_));
                        if arrival != Arrival::Page && class == "foreign" {
                            assert!(!entered, "{case}");
                        }
                        if arrival != Arrival::Page && other {
                            assert!(!entered, "{case}");
                        }
                        if arrival == Arrival::Preflight {
                            assert!(!entered, "{case}");
                        }
                        if arrival == Arrival::Pairing && entered {
                            assert_eq!(entry, Entry::Caller(Caller::Browser), "{case}");
                        }
                        if arrival == Arrival::Page {
                            assert_eq!(entry, Entry::Page, "{case}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn a_listener_on_every_interface_is_opened_at_loopback_and_admits_its_own_host_only() {
        let origins = ListenerOrigins::of("0.0.0.0:2501".parse().unwrap());
        assert_eq!(origins.url(), "http://127.0.0.1:2501");
        assert!(origins.admits_host("192.168.1.10:2501"));
        assert!(!origins.admits_host("0.0.0.0:2501"));
        assert!(!origins.admits_host("evil.example:2501"));
        assert!(!origins.admits_host("192.168.1.10:9999"));
        assert!(origins.admits_origin("http://192.168.1.10:2501", "192.168.1.10:2501"));
        assert!(!origins.admits_origin("http://10.0.0.5:2501", "192.168.1.10:2501"));
    }

    #[test]
    fn a_routed_listener_admits_its_loopback_names_and_the_public_name_only() {
        let origins = ListenerOrigins::routed(
            "127.0.0.1:41000".parse().unwrap(),
            "https://City.example.net:443/remote?x",
        );
        let at = |host, origin| {
            let presented = Presented {
                host: Some(host),
                origin,
                sec_fetch_site: Some("same-origin"),
            };
            decide_entry(&origins, presented, Arrival::Socket)
        };
        let public = Some("https://city.example.net");
        let browser = Entry::Caller(Caller::Browser);
        let seen = [
            at("city.example.net", public),
            at("127.0.0.1:41000", public),
            at("localhost:41000", Some("http://localhost:41000")),
            at("127.0.0.1:41000", None),
            at("evil.example", public),
            at("city.example.net:41000", public),
            at("city.example.net", Some("https://evil.example")),
            at("city.example.net", Some("http://city.example.net")),
        ];
        assert_eq!(
            seen,
            [
                browser,
                browser,
                browser,
                Entry::Caller(Caller::Native),
                Entry::Refused(EntryRefusal::ForeignHost),
                Entry::Refused(EntryRefusal::ForeignHost),
                Entry::Refused(EntryRefusal::ForeignOrigin),
                Entry::Refused(EntryRefusal::ForeignOrigin),
            ]
        );
    }

    #[test]
    fn the_default_port_is_admitted_without_being_written() {
        let origins = ListenerOrigins::of("127.0.0.1:80".parse().unwrap());
        assert!(origins.admits_host("localhost"));
        assert!(origins.admits_origin("http://localhost", "localhost"));
    }
}
