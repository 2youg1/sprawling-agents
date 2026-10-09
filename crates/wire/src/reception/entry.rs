// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Whether one request may knock at this city's door at all, before any
//! credential is read (`crates/wire/spec/Reception/Entry.lean` §8-94,
//! wire D54).
//!
//! The names and origins a listener answers to are computed once, from
//! the address it holds, into [`ListenerOrigins`]; the Host check, the
//! Origin check, the URL a person is given and the residents' browser
//! guard all read that one value.

use std::net::{IpAddr, SocketAddr};

use kernel::{AxCode, AxError};

use super::Door;

/// The one value `Sec-Fetch-Site` may carry on a request this city
/// answers. A page on another port of this machine sends `same-site`.
const SAME_ORIGIN: &str = "same-origin";

/// The port a browser leaves out of `Host` and `Origin`.
const DEFAULT_HTTP_PORT: u16 = 80;

/// The port a browser leaves out of an `https` address.
const DEFAULT_HTTPS_PORT: &str = ":443";

/// The names and origins one listener answers to, computed once from
/// the address it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListenerOrigins {
    hosts: Hosts,
    url: String,
}

/// Which `Host` values a listener admits.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Hosts {
    /// The listener's own names, each with its port, and the origins of
    /// the pages it serves.
    Named {
        names: Vec<String>,
        origins: Vec<String>,
    },
    /// A listener on every interface: any IP literal with this port, the
    /// unspecified address itself excepted, and the loopback names.
    AnyAddress { port: u16, loopback: Vec<String> },
}

impl ListenerOrigins {
    /// The names a listener bound to `local` answers to.
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

    /// The names the remote listener bound to `local` answers to: its
    /// loopback names, and the host of `public`, the `https://` address
    /// its route gave (wire D56).
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

    /// The address a person on this machine opens: `http://127.0.0.1:<port>`
    /// for a loopback or every-interface listener. One spelling, because a
    /// browser keeps its device key per origin and a second spelling of
    /// the same listener is a second origin with no key in it.
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Whether `host`, as a request's `Host` header spells it, names this
    /// listener.
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

    /// Whether `origin` is one of this listener's own, for a request whose
    /// admitted `Host` is `host`.
    ///
    /// A named listener admits any of its origins; a listener on every
    /// interface admits only the origin of the host the request was sent
    /// to, because "any IP literal" would let a page on another machine of
    /// the network in.
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
    /// A listener answering to `names`, whose pages are served over
    /// `http` under the same names.
    fn named(names: Vec<String>) -> Self {
        let origins = names.iter().map(|name| format!("http://{name}")).collect();
        Self::Named { names, origins }
    }
}

/// The host of a route's `https://` address as a browser writes it in
/// `Host`: lowercase, without `:443`. `None` when the address is not
/// `https` or has no host (wire D56).
fn public_name(public: &str) -> Option<String> {
    let (scheme, rest) = public.split_once("://")?;
    let authority = rest.split(['/', '?']).next()?.to_ascii_lowercase();
    let name = authority
        .strip_suffix(DEFAULT_HTTPS_PORT)
        .map_or(authority.as_str(), |bare| bare)
        .to_owned();
    (scheme.eq_ignore_ascii_case("https") && !name.is_empty()).then_some(name)
}

/// The name `host:port`, and the bare name too when the port is the one a
/// browser leaves out.
fn spelled(name: &str, port: u16) -> Vec<String> {
    let mut names = vec![format!("{name}:{port}")];
    if port == DEFAULT_HTTP_PORT {
        names.push(name.to_owned());
    }
    names
}

/// An address as a `Host` header writes it: IPv6 in brackets.
fn literal(ip: IpAddr) -> String {
    match ip {
        IpAddr::V4(v4) => v4.to_string(),
        IpAddr::V6(v6) => format!("[{v6}]"),
    }
}

/// Whether `host` is an IP literal other than the unspecified address,
/// carrying `port` (or no port, when `port` is the default one).
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

/// The response headers a listener answers with, as names and values;
/// the shell turns them into HTTP headers once per listener.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageHeaders {
    lines: Vec<(&'static str, String)>,
}

impl PageHeaders {
    /// The headers every listener answers with: the city's own port and
    /// the remote listener serve the same page under the same headers
    /// (wire D20). `connect-src 'self'` admits the page's socket on its
    /// own host and port, `ws:` from an `http:` page and `wss:` from an
    /// `https:` one, in every engine that has the WebCrypto Ed25519 a
    /// paired browser needs.
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

    /// Every header, in the order it is written.
    #[must_use]
    pub fn lines(&self) -> &[(&'static str, String)] {
        &self.lines
    }
}

/// The three headers of a request that say who is knocking. One value,
/// because the entry decision reads all three together.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Presented<'a> {
    pub host: Option<&'a str>,
    pub origin: Option<&'a str>,
    pub sec_fetch_site: Option<&'a str>,
}

/// Which route a request arrived at, as the route table names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrival {
    /// A GET of the client bundle.
    Page,
    /// An `OPTIONS` request at any route.
    Preflight,
    /// The socket upgrade at `/ws`.
    Socket,
    /// One of the HTTP doors that act.
    Door(Door),
    /// `/pair`, `/session/challenge` or `/session`.
    Pairing,
}

/// Who is knocking, once the entry decision has let them knock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Caller {
    /// A program on this machine: it sends no `Origin`.
    Native,
    /// A page served by this listener.
    Browser,
}

/// The verdict of [`decide_entry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entry {
    /// The client bundle, served with this listener's headers and without
    /// a credential.
    Page,
    /// A caller who may now present a credential.
    Caller(Caller),
    /// Refused with 403 before anything else happens.
    Refused(EntryRefusal),
}

/// Why a request was refused at the entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryRefusal {
    ForeignHost,
    Preflight,
    ForeignOrigin,
    CrossSite,
    NativePairing,
}

impl EntryRefusal {
    /// The refusal a person or a program reads.
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

/// Decides whether one request may knock at all, in the order of
/// `crates/wire/spec/Reception/Entry.lean` §8-94: Host, page, preflight,
/// Origin, `Sec-Fetch-Site`, then the pairing routes for browsers only.
/// Pure; the credential is judged after this, by [`super::Keys`].
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
        ListenerOrigins::of("127.0.0.1:8787".parse().unwrap())
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

    /// The five properties of `crates/wire/spec/Reception/Entry.lean`,
    /// checked cell by cell over every input the model admits: each host,
    /// origin and fetch-site class a request can present, at every route.
    #[test]
    fn every_entry_property_holds_over_every_input() {
        let origins = loopback();
        let hosts = [
            (Some("127.0.0.1:8787"), true),
            (Some("localhost:8787"), true),
            (Some("evil.example:8787"), false),
            (Some("0.0.0.0:8787"), false),
            (None, false),
        ];
        let seen = [
            (None, "absent"),
            (Some("http://127.0.0.1:8787"), "listed"),
            (Some("http://localhost:8787"), "listed"),
            (Some("http://127.0.0.1:5173"), "foreign"),
            (Some("null"), "foreign"),
            (Some("https://127.0.0.1:8787"), "foreign"),
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
        let origins = ListenerOrigins::of("0.0.0.0:8787".parse().unwrap());
        assert_eq!(origins.url(), "http://127.0.0.1:8787");
        assert!(origins.admits_host("192.168.1.10:8787"));
        assert!(!origins.admits_host("0.0.0.0:8787"));
        assert!(!origins.admits_host("evil.example:8787"));
        assert!(!origins.admits_host("192.168.1.10:9999"));
        assert!(origins.admits_origin("http://192.168.1.10:8787", "192.168.1.10:8787"));
        assert!(!origins.admits_origin("http://10.0.0.5:8787", "192.168.1.10:8787"));
    }

    /// Wire D56: a route may pass the browser's `Host` on or rewrite it
    /// to the loopback port, and the page on the route sends the public
    /// `https` origin; anything else is another site.
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
