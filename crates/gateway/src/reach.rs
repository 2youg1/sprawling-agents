// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where a call to a provider stops, stage by stage.
//!
//! **A refusal that says `error sending request for url (...): operation
//! timed out` tells a person nothing they can act on.** Four things can
//! go wrong before a provider ever sees a request, and each has a
//! different next step: the name does not resolve (check the spelling,
//! or the proxy), the connection is refused or never answers (a
//! firewall, a wrong port, a proxy that is not running), the handshake
//! fails (a hostname that is not a legal DNS name, a certificate this
//! machine does not trust), or the provider answers with a status (401
//! is a key, 404 is a path). This module measures those stages and
//! reports which one stopped, so the page can say the next step beside
//! the field that holds the wrong value.
//!
//! **The stages before TLS are measured directly, and only when nothing
//! proxies the call.** A proxy does the city's resolving and connecting
//! for it, so resolving the provider's own name here would report on a
//! path the request never takes. When a proxy applies, the report says
//! so and starts at the request itself. Which calls a proxy applies to
//! is `proxy`'s answer, and this module asks it rather than deciding
//! again - a reading that disagreed with the call it describes is worse
//! than no reading.
//!
//! Time is a parameter. Nothing here samples a clock: the caller stamps
//! before and after, because the one sampling point is Main.
//!
//! The vocabulary is `kernel::reach`; what a socket can find out is
//! here.

use std::net::{TcpStream, ToSocketAddrs as _};
use std::time::Duration;

use kernel::{Answered, Connected, Named, Proxying, Reach, Through};

mod proxy;

pub use proxy::{client_for, is_local, through};

/// How long one stage may take. Short, because a person is watching a
/// settings page and a host that cannot answer in this time is one they
/// want to hear about rather than wait for.
const STAGE_TIMEOUT: Duration = Duration::from_secs(5);

/// The host and port a base URL points at, without pulling in a URL
/// parser for four fields: scheme, host, optional port, and whether the
/// scheme is the TLS one.
pub(crate) fn split(base_url: &str) -> Option<(String, u16, bool)> {
    let (scheme, rest) = base_url.split_once("://")?;
    let tls = match scheme {
        "https" => true,
        "http" => false,
        _ => return None,
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    let authority = authority.rsplit('@').next().unwrap_or(authority);
    let fallback = if tls { 443 } else { 80 };
    // An IPv6 literal carries colons of its own, so brackets decide
    // where the address ends and a port begins.
    let (host, port) = match authority.strip_prefix('[') {
        Some(bracketed) => {
            let (inside, rest) = bracketed.split_once(']')?;
            match rest.strip_prefix(':') {
                Some(port) => (inside, port.parse::<u16>().ok()?),
                None => (inside, fallback),
            }
        }
        None => match authority.rsplit_once(':') {
            Some((host, port)) if !host.contains(':') => (host, port.parse::<u16>().ok()?),
            _ => (authority, fallback),
        },
    };
    if host.is_empty() {
        return None;
    }
    Some((host.to_owned(), port, tls))
}

/// Resolve and connect, when the request will take that path itself.
fn transport(host: &str, port: u16, proxied: bool) -> (Named, Connected) {
    if proxied {
        return (Named::ProxiedAway, Connected::Skipped);
    }
    let addresses = match (host, port).to_socket_addrs() {
        Ok(found) => found.collect::<Vec<_>>(),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return (Named::NotFound, Connected::Skipped);
        }
        Err(err) => {
            // Windows answers an unknown host with `HostUnreachable` or
            // a raw error code rather than `NotFound`, so the text is
            // what distinguishes "no such host" from a resolver that is
            // itself unreachable.
            let text = err.to_string();
            return (Named::Refused(text), Connected::Skipped);
        }
    };
    let Some(first) = addresses.first() else {
        return (Named::NotFound, Connected::Skipped);
    };
    let named = Named::Resolved(u32::try_from(addresses.len()).unwrap_or(u32::MAX));
    let connected = match TcpStream::connect_timeout(first, STAGE_TIMEOUT) {
        Ok(_) => Connected::Open,
        Err(err) => match err.kind() {
            std::io::ErrorKind::ConnectionRefused => Connected::Refused,
            std::io::ErrorKind::TimedOut => Connected::Silent,
            _ => Connected::Failed(err.to_string()),
        },
    };
    (named, connected)
}

/// The whole error chain as one line. reqwest's own `Display` says only
/// that sending failed; which stage stopped is one link further down.
fn chain(err: &reqwest::Error) -> String {
    let mut detail = err.to_string();
    let mut source = std::error::Error::source(err);
    while let Some(link) = source {
        detail.push_str(": ");
        detail.push_str(&link.to_string());
        source = link.source();
    }
    detail
}

/// Which stage a flattened error chain names.
fn stopped_at(detail: String) -> Answered {
    let lowered = detail.to_ascii_lowercase();
    if lowered.contains("invalid dns name") || lowered.contains("invalid server name") {
        return Answered::NameNotUsable(detail);
    }
    if lowered.contains("certificate")
        || lowered.contains("handshake")
        || lowered.contains("tls")
        || lowered.contains("unknownissuer")
    {
        return Answered::HandshakeFailed(detail);
    }
    Answered::Unreachable(detail)
}

/// One staged reading, from a client that is already configured the way
/// the city's calls are.
///
/// Never fails: a report of where a call stops is the answer, including
/// when it stops at the first stage. `elapsed_ms` is the caller's, and a
/// base URL this cannot split reports as a name that does not resolve,
/// which is what a person typing `api.openai.com` without a scheme has
/// in fact produced. `rule` is the one the client was built with; a
/// reading taken under a different rule would describe another city's
/// call.
pub fn reach(
    client: &reqwest::blocking::Client,
    rule: Proxying,
    base_url: &str,
    elapsed_ms: u64,
) -> Reach {
    let Some((host, port, _)) = split(base_url) else {
        return Reach {
            host: base_url.to_owned(),
            named: Named::NotFound,
            connected: Connected::Skipped,
            answered: Answered::Unreachable(String::from("a base URL begins http:// or https://")),
            through: Through::Direct,
            elapsed_ms,
        };
    };
    let through = through(rule, base_url);
    let proxied = matches!(through, Through::Environment(_));
    let (named, connected) = transport(&host, port, proxied);
    let answered = match client
        .get(base_url)
        .timeout(STAGE_TIMEOUT)
        .header("accept", "application/json")
        .send()
    {
        Ok(response) => Answered::Status(response.status().as_u16()),
        Err(err) => stopped_at(chain(&err)),
    };
    Reach {
        host,
        named,
        connected,
        answered,
        through,
        elapsed_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_base_url_splits_into_the_host_and_port_a_socket_needs() {
        assert_eq!(
            split("https://api.openai.com/v1"),
            Some((String::from("api.openai.com"), 443, true))
        );
        assert_eq!(
            split("http://127.0.0.1:11434/v1/chat/completions"),
            Some((String::from("127.0.0.1"), 11434, false))
        );
        assert_eq!(
            split("https://[2606:4700::1111]:8443/v1"),
            Some((String::from("2606:4700::1111"), 8443, true))
        );
        assert_eq!(
            split("https://user:pass@gateway.internal/v1"),
            Some((String::from("gateway.internal"), 443, true))
        );
    }

    /// The two shapes a person actually types: no scheme at all, and a
    /// scheme this city does not speak.
    #[test]
    fn a_base_url_without_a_scheme_this_speaks_is_no_host() {
        assert_eq!(split("api.openai.com/v1"), None);
        assert_eq!(split("ftp://files.example.com"), None);
        assert_eq!(split("https://"), None);
    }

    #[test]
    fn a_refusal_is_classified_by_the_stage_it_names() {
        // The classifier reads the chain rather than reqwest's own kind,
        // because the kind says `is_connect` for both a refused socket
        // and a certificate this machine does not trust.
        assert!(matches!(
            stopped_at(String::from("invalid dns name: my_host")),
            Answered::NameNotUsable(_)
        ));
        assert!(matches!(
            stopped_at(String::from("invalid peer certificate: UnknownIssuer")),
            Answered::HandshakeFailed(_)
        ));
        assert!(matches!(
            stopped_at(String::from("operation timed out")),
            Answered::Unreachable(_)
        ));
    }
}
