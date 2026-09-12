// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which calls go through this machine's proxy, and what a person may
//! settle about that.
//!
//! **Every HTTP client this city builds is built here.** A rule applied
//! at five construction sites is five rules that agree until one of them
//! is edited; worse, the staged reading in the parent module would then
//! describe a path the request does not take, which is the one thing a
//! person reading that report cannot check.
//!
//! The default takes an address on this machine off the proxy, because a
//! proxy that intercepts loopback answers 502 for a local inference
//! server, for an MCP server a person started themselves, and for the
//! city's own enrolment door. That is a rule about the common machine
//! rather than a fact about every machine, so it is a setting with a
//! default and not a constant: an organisation that audits every request
//! through a relay on loopback sets `always`, and a machine whose
//! virtual network adapter already carries every packet sets `never`
//! rather than letting a stale proxy variable break a call.

use kernel::{Proxying, Through};

/// Whether this base URL points at the machine the city runs on.
///
/// One authority city-wide: the local adapter refuses a URL that is not
/// local by this test, the proxy decision is made by this test, and the
/// settings page reports "local" from this test, so the three can never
/// mean three different things.
#[must_use]
pub fn is_local(base_url: &str) -> bool {
    let Some((host, _, _)) = super::split(base_url) else {
        return false;
    };
    match host.parse::<std::net::IpAddr>() {
        Ok(address) => address.is_loopback(),
        Err(_) => host == "localhost" || host.ends_with(".localhost"),
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
        Proxying::ExceptLocal if is_local(base_url) => Through::LocalAddress,
        Proxying::ExceptLocal | Proxying::Always => named_by_environment(&host, tls),
    }
}

/// An HTTP client that reaches this URL the way the person settled it.
///
/// The caller adds its own deadline and whatever else it needs; what it
/// may not do is decide the proxy again.
pub fn client_for(rule: Proxying, base_url: &str) -> reqwest::blocking::ClientBuilder {
    let builder = reqwest::blocking::Client::builder();
    match through(rule, base_url) {
        Through::LocalAddress | Through::Disabled => builder.no_proxy(),
        Through::Direct | Through::Environment(_) | Through::Excluded => builder,
    }
}

/// Which proxy variable, if any, this host's requests will read.
fn named_by_environment(host: &str, tls: bool) -> Through {
    let excluded = std::env::var("NO_PROXY")
        .or_else(|_| std::env::var("no_proxy"))
        .unwrap_or_default();
    if excluded_by(host, &excluded) {
        return Through::Excluded;
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

/// Whether a `NO_PROXY` list covers this host.
fn excluded_by(host: &str, list: &str) -> bool {
    list.split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .any(|entry| entry == "*" || host == entry || host.ends_with(entry))
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
