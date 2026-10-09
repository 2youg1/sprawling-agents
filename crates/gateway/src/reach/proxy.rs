// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which calls go through this machine's proxy, and what a person may
//! settle about that.
//!
//! **Every HTTP client this city builds is built here**, after the
//! process's TLS backend is installed, and clippy refuses a client built
//! anywhere else (`crates/gateway/spec/Reach.lean` §8-15). A rule applied
//! at five construction sites is five rules that agree until one of them
//! is edited; worse, the staged reading in the parent module would then
//! describe a path the request does not take, which is the one thing a
//! person reading that report cannot check.
//!
//! The default takes an address on this machine or on its private network
//! off the proxy, because a proxy that intercepts loopback answers 502 for
//! a local inference server, for an MCP server a person started
//! themselves, and for the city's own enrolment door, and a proxy handed
//! a call to a model server on the local network reads the prompt in
//! plain text on a path the call never needed (gateway D35). That is a
//! rule about the common machine
//! rather than a fact about every machine, so it is a setting with a
//! default and not a constant: an organisation that audits every request
//! through a relay on loopback sets `always`, and a machine whose
//! virtual network adapter already carries every packet sets `never`
//! rather than letting a stale proxy variable break a call.

use kernel::{Proxying, Through};

/// Where an address is, as far as a call to it is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Locality {
    /// The machine the city runs on: a loopback address, `localhost`, or
    /// a name under `.localhost`.
    Machine,
    /// The private network this machine is on: an RFC 1918 or link-local
    /// IPv4 address, a unique-local or link-local IPv6 address, or a name
    /// under `.local` (mDNS) or `.home.arpa`.
    Private,
    /// Anywhere else, and a URL this cannot split.
    Public,
}

/// Where this base URL points.
///
/// One authority city-wide (gateway D35): the proxy decision reads it,
/// and so does [`is_local`], which the local adapter, the settings page's
/// "local" mark and the confidential building's rule read, so none of
/// them can mean something different by "local". A name is judged by
/// its spelling and never resolved: a lookup would make the answer
/// depend on the network and send every endpoint's name to a resolver.
#[must_use]
pub(crate) fn locality(base_url: &str) -> Locality {
    let Some((host, _, _)) = super::split(base_url) else {
        return Locality::Public;
    };
    match host.parse::<std::net::IpAddr>() {
        Ok(address) => locality_of(address.to_canonical()),
        Err(_) => locality_of_name(&host.to_ascii_lowercase()),
    }
}

/// Whether this base URL points at the machine the city runs on.
#[must_use]
pub fn is_local(base_url: &str) -> bool {
    locality(base_url) == Locality::Machine
}

fn locality_of(address: std::net::IpAddr) -> Locality {
    use std::net::IpAddr;
    let private = match address {
        _ if address.is_loopback() => return Locality::Machine,
        IpAddr::V4(v4) => v4.is_private() || v4.is_link_local(),
        IpAddr::V6(v6) => v6.is_unique_local() || v6.is_unicast_link_local(),
    };
    if private {
        Locality::Private
    } else {
        Locality::Public
    }
}

fn locality_of_name(host: &str) -> Locality {
    let under = |domain: &str| host == domain || host.ends_with(&format!(".{domain}"));
    if under("localhost") {
        Locality::Machine
    } else if under("local") || under("home.arpa") {
        Locality::Private
    } else {
        Locality::Public
    }
}

/// What the city will send a request to this URL through.
///
/// The environment is what a report can state as fact. A system proxy
/// setting is read by the HTTP client itself and is not visible here, so
/// a reading of `direct` says only that nothing this side can see names
/// a proxy.
#[must_use]
pub fn through(rule: Proxying, base_url: &str) -> Through {
    let Some((host, _, tls)) = super::split(base_url) else {
        return Through::Direct;
    };
    match rule {
        Proxying::Never => Through::Disabled,
        Proxying::ExceptLocal => match locality(base_url) {
            // The private network is kept off the proxy with this machine
            // (gateway D35); `Through` has one arm for both.
            Locality::Machine | Locality::Private => Through::LocalAddress,
            Locality::Public => named_by_environment(&host, tls),
        },
        Proxying::Always => named_by_environment(&host, tls),
    }
}

/// An HTTP client that reaches this URL the way the person settled it,
/// and names this city as the client.
///
/// The caller adds its own deadline and whatever else it needs; what it
/// may not do is decide the proxy again. The name is here, once, for
/// every outbound call, because servers refuse a request that will not
/// say what sent it: OpenCode Go asks each client to identify itself by
/// its own name rather than an HTTP library's, and a hosted MCP server
/// behind a content delivery network answers 403 to a request with no
/// user agent at all.
///
/// The builder comes after the process's one TLS backend is installed
/// (`reach::tls`), so a client built from it has a backend to find;
/// this is the only construction site clippy admits (`crates/gateway/spec/Reach.lean`
/// §8-15).
pub fn client_for(rule: Proxying, base_url: &str) -> reqwest::blocking::ClientBuilder {
    super::tls::install_provider();
    #[expect(
        clippy::disallowed_methods,
        reason = "the one place a client is built; the TLS backend is installed on the line above"
    )]
    let builder = reqwest::blocking::Client::builder()
        .user_agent(concat!("sprawling/", env!("CARGO_PKG_VERSION")));
    match through(rule, base_url) {
        Through::LocalAddress | Through::Disabled => builder.no_proxy(),
        Through::Direct | Through::Environment(_) | Through::Excluded => builder,
    }
}

/// Which proxy variable, if any, this host's requests will read.
fn named_by_environment(host: &str, tls: bool) -> Through {
    match exclusions(|name| std::env::var(name)) {
        Exclusions::Listed(list) if excluded_by(host, &list) => return Through::Excluded,
        // A list that does not cover this host leaves the variables in force.
        Exclusions::Listed(_) | Exclusions::Unset => {}
        // The client reads a `NO_PROXY` that is not Unicode as unset, so
        // it excludes nothing and the variables apply; the reading states
        // the path the request takes rather than one it does not
        // (gateway D23).
        Exclusions::Unreadable => {}
    }
    let ordered: [&str; 6] = if tls {
        [
            "HTTPS_PROXY",
            "https_proxy",
            "ALL_PROXY",
            "all_proxy",
            "HTTP_PROXY",
            "http_proxy",
        ]
    } else {
        [
            "HTTP_PROXY",
            "http_proxy",
            "ALL_PROXY",
            "all_proxy",
            "HTTPS_PROXY",
            "https_proxy",
        ]
    };
    for name in ordered {
        if std::env::var(name).is_ok_and(|value| !value.trim().is_empty()) {
            return Through::Environment(name.to_owned());
        }
    }
    Through::Direct
}

/// The `NO_PROXY` list the HTTP client reads for every request.
///
/// The client (hyper-util's proxy matcher, under reqwest) takes the first
/// of `NO_PROXY` and `no_proxy` that holds Unicode and treats the rest as
/// absent; this reads the same two names in the same order, so the
/// reading cannot name a list the request does not use (gateway D23).
#[derive(Debug, PartialEq, Eq)]
enum Exclusions {
    /// A spelling held this list.
    Listed(String),
    /// Neither spelling is set.
    Unset,
    /// A spelling is set to a value that is not Unicode (invalid UTF-8
    /// on macOS and Linux, an unpaired surrogate on Windows) and no
    /// spelling after it holds one. The client excludes nothing.
    Unreadable,
}

fn exclusions(read: impl Fn(&str) -> Result<String, std::env::VarError>) -> Exclusions {
    ["NO_PROXY", "no_proxy"]
        .into_iter()
        .try_fold(Exclusions::Unset, |seen, name| match read(name) {
            Ok(list) => Err(Exclusions::Listed(list)),
            Err(std::env::VarError::NotPresent) => Ok(seen),
            Err(std::env::VarError::NotUnicode(_)) => Ok(Exclusions::Unreadable),
        })
        .unwrap_or_else(|listed| listed)
}

/// Whether a `NO_PROXY` list covers this host, by the client's rules
/// (gateway D23): an address is covered by an equal address or by a
/// network that holds it; a name is covered by `*`, by an entry equal to
/// it, or by an entry it ends in after a dot, a leading dot on the entry
/// meaning the same, ASCII case ignored. `example.com` therefore covers
/// `api.example.com` and not `badexample.com`.
fn excluded_by(host: &str, list: &str) -> bool {
    let mut entries = list
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty());
    match host.parse::<std::net::IpAddr>() {
        Ok(address) => entries.any(|entry| network_covers(entry, address)),
        Err(_) => entries.any(|entry| domain_covers(entry, host)),
    }
}

fn domain_covers(entry: &str, host: &str) -> bool {
    let domain = entry.strip_prefix('.').unwrap_or(entry);
    entry == "*"
        || host.eq_ignore_ascii_case(domain)
        || domain
            .len()
            .checked_add(1)
            .and_then(|tail| host.len().checked_sub(tail))
            .and_then(|cut| host.get(cut..))
            .and_then(|tail| tail.strip_prefix('.'))
            .is_some_and(|parent| parent.eq_ignore_ascii_case(domain))
}

/// Whether an address entry (`10.0.0.1`) or a network entry
/// (`10.0.0.0/8`) holds this address. An entry of the other family, or
/// one that does not parse, holds nothing.
fn network_covers(entry: &str, address: std::net::IpAddr) -> bool {
    use std::net::IpAddr;
    let (network, prefix) = match entry.split_once('/') {
        Some((network, prefix)) => (network, Some(prefix)),
        None => (entry, None),
    };
    match (network.parse::<IpAddr>(), address) {
        (Ok(IpAddr::V4(network)), IpAddr::V4(address)) => same_prefix(
            u128::from(u32::from(network)),
            u128::from(u32::from(address)),
            32,
            prefix,
        ),
        (Ok(IpAddr::V6(network)), IpAddr::V6(address)) => {
            same_prefix(u128::from(network), u128::from(address), 128, prefix)
        }
        (Ok(IpAddr::V4(_)), IpAddr::V6(_)) | (Ok(IpAddr::V6(_)), IpAddr::V4(_)) | (Err(_), _) => {
            false
        }
    }
}

/// Whether two addresses `width` bits wide agree on their first
/// `prefix` bits; no prefix means the whole address.
fn same_prefix(network: u128, address: u128, width: u32, prefix: Option<&str>) -> bool {
    let kept = match prefix.map(str::parse::<u32>) {
        None => width,
        Some(Ok(bits)) if bits <= width => bits,
        Some(Ok(_) | Err(_)) => return false,
    };
    width
        .checked_sub(kept)
        .is_some_and(|dropped| network.checked_shr(dropped) == address.checked_shr(dropped))
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

    /// Found by running it: with the system proxy compiled in, a machine
    /// whose owner runs a proxy sent loopback through it and the local
    /// server answered 502 - through somebody else's gateway, for a
    /// request that never had to leave the machine.
    #[test]
    fn a_call_to_this_machine_is_recognised_whatever_it_is_spelled_as() {
        assert!(is_local("http://127.0.0.1:11434/v1"));
        assert!(is_local("http://localhost:8791/enroll"));
        assert!(is_local("http://[::1]:8791/"));
        assert!(!is_local("https://api.openai.com/v1"));
        assert!(!is_local("https://127.0.0.1.nip.io/v1"));
    }

    /// `NO_PROXY` is read as a fact about this host, so a city inside a
    /// corporate network does not report a proxy for a provider the
    /// proxy is told to skip. Environment reading is process-wide, so
    /// this asserts the pure half.
    #[test]
    fn a_host_on_no_proxy_reports_that_the_variables_do_not_apply() {
        assert!(excluded_by("api.openai.com", "localhost, .openai.com"));
        assert!(!excluded_by("api.openai.com", "localhost"));
        assert!(excluded_by("anything", "*"));
    }

    /// Found in review: a suffix test let `example.com` exclude
    /// `badexample.com`, so the reading said "excluded" for a request the
    /// client sent through the proxy. The cases follow the client's own
    /// (hyper-util's `NoProxy` tests), so the two do not drift silently.
    #[test]
    fn an_entry_covers_its_domain_and_subdomains_and_never_a_longer_name() {
        let list = "example.com, .corp.test, 10.0.0.0/8, 192.168.1.7, ::1, fd00::/8";
        let asked = [
            "example.com",
            "api.example.com",
            "API.Example.COM",
            "badexample.com",
            "corp.test",
            "x.corp.test",
            "xcorp.test",
            "10.200.3.4",
            "11.0.0.1",
            "192.168.1.7",
            "192.168.1.8",
            "::1",
            "fd12::1",
            "fe80::1",
        ];
        let covered: Vec<bool> = asked.iter().map(|host| excluded_by(host, list)).collect();
        assert_eq!(
            covered,
            vec![
                true, true, true, false, true, true, false, true, false, true, false, true, true,
                false,
            ]
        );
    }

    /// A `NO_PROXY` that is not Unicode is read the way the client reads
    /// it: as absent, so the lower-case spelling answers when it is set and
    /// nothing is excluded when it is not (gateway D23). The reader is
    /// injected because setting a process variable is a fact about the
    /// whole test process.
    #[test]
    fn a_no_proxy_that_is_not_unicode_excludes_nothing_as_the_client_reads_it() {
        use std::env::VarError;
        let not_unicode = || VarError::NotUnicode(std::ffi::OsString::from("x"));
        let read = |upper: Result<String, VarError>, lower: Result<String, VarError>| {
            exclusions(|name| match name {
                "NO_PROXY" => upper.clone(),
                _ => lower.clone(),
            })
        };
        assert_eq!(
            [
                read(Err(not_unicode()), Err(VarError::NotPresent)),
                read(Err(not_unicode()), Ok("example.com".to_owned())),
                read(Ok("a.test".to_owned()), Ok("b.test".to_owned())),
                read(Err(VarError::NotPresent), Err(VarError::NotPresent)),
                read(Err(VarError::NotPresent), Err(not_unicode())),
            ],
            [
                Exclusions::Unreadable,
                Exclusions::Listed("example.com".to_owned()),
                Exclusions::Listed("a.test".to_owned()),
                Exclusions::Unset,
                Exclusions::Unreadable,
            ]
        );
    }

    /// The two settings that do not consult the environment answer the
    /// same way on every machine, which is what lets them be tested at
    /// all: environment reading is process-wide and a test that set a
    /// variable would be a test about the whole process.
    #[test]
    fn the_settled_readings_do_not_depend_on_this_machine() {
        assert_eq!(
            through(Proxying::ExceptLocal, "http://127.0.0.1:11434/v1"),
            Through::LocalAddress
        );
        assert_eq!(
            through(Proxying::Never, "https://api.openai.com/v1"),
            Through::Disabled
        );
        assert_eq!(
            through(Proxying::Never, "http://127.0.0.1:11434/v1"),
            Through::Disabled
        );
    }

    /// `always` is the whole point of the setting: the same loopback URL
    /// that `except_local` takes off the proxy is left on it, so what
    /// the environment says is what the reading says.
    #[test]
    fn always_leaves_a_local_address_on_whatever_the_machine_names() {
        let reading = through(Proxying::Always, "http://127.0.0.1:11434/v1");
        assert_ne!(reading, Through::LocalAddress);
        assert!(matches!(
            reading,
            Through::Direct | Through::Environment(_) | Through::Excluded
        ));
    }

    /// Handing out a builder installs the process's one crypto provider,
    /// so the client built from it finds a TLS backend rather than
    /// panicking at `build()` (`crates/gateway/spec/Reach.lean` §8-15).
    #[test]
    fn a_builder_handed_out_brings_the_one_crypto_provider_with_it() {
        let _builder = client_for(Proxying::ExceptLocal, "https://example.invalid");

        let installed = rustls::crypto::CryptoProvider::get_default();
        assert!(
            installed.is_some(),
            "client_for handed out a builder with no process-wide crypto provider installed"
        );
        let suites = |provider: &rustls::crypto::CryptoProvider| {
            provider
                .cipher_suites
                .iter()
                .map(|suite| suite.suite())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            suites(installed.unwrap()),
            suites(&rustls::crypto::aws_lc_rs::default_provider())
        );
    }

    /// A URL with no scheme has no host to judge, and the report says
    /// nothing about a proxy rather than inventing a reading.
    #[test]
    fn a_url_this_cannot_split_reports_no_proxy_decision() {
        assert_eq!(
            through(Proxying::ExceptLocal, "api.openai.com"),
            Through::Direct
        );
    }
}
