// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Proxying, Through};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Locality {

    Machine,

    Private,

    Public,
}

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

#[must_use]
pub fn through(rule: Proxying, base_url: &str) -> Through {
    let Some((host, _, tls)) = super::split(base_url) else {
        return Through::Direct;
    };
    match rule {
        Proxying::Never => Through::Disabled,
        Proxying::ExceptLocal => match locality(base_url) {

            Locality::Machine | Locality::Private => Through::LocalAddress,
            Locality::Public => named_by_environment(&host, tls),
        },
        Proxying::Always => named_by_environment(&host, tls),
    }
}

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

fn named_by_environment(host: &str, tls: bool) -> Through {
    match exclusions(|name| std::env::var(name)) {
        Exclusions::Listed(list) if excluded_by(host, &list) => return Through::Excluded,

        Exclusions::Listed(_) | Exclusions::Unset => {}

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

#[derive(Debug, PartialEq, Eq)]
enum Exclusions {

    Listed(String),

    Unset,

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

    #[test]
    fn a_call_to_this_machine_is_recognised_whatever_it_is_spelled_as() {
        assert!(is_local("http://127.0.0.1:11434/v1"));
        assert!(is_local("http://localhost:8791/enroll"));
        assert!(is_local("http://[::1]:8791/"));
        assert!(!is_local("https://api.openai.com/v1"));
        assert!(!is_local("https://127.0.0.1.nip.io/v1"));
    }

    #[test]
    fn a_host_on_no_proxy_reports_that_the_variables_do_not_apply() {
        assert!(excluded_by("api.openai.com", "localhost, .openai.com"));
        assert!(!excluded_by("api.openai.com", "localhost"));
        assert!(excluded_by("anything", "*"));
    }

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

    #[test]
    fn always_leaves_a_local_address_on_whatever_the_machine_names() {
        let reading = through(Proxying::Always, "http://127.0.0.1:11434/v1");
        assert_ne!(reading, Through::LocalAddress);
        assert!(matches!(
            reading,
            Through::Direct | Through::Environment(_) | Through::Excluded
        ));
    }

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

    #[test]
    fn a_url_this_cannot_split_reports_no_proxy_decision() {
        assert_eq!(
            through(Proxying::ExceptLocal, "api.openai.com"),
            Through::Direct
        );
    }
}
