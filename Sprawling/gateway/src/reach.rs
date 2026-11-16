// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::net::{TcpStream, ToSocketAddrs as _};
use std::time::Duration;

use kernel::{Answered, Connected, Named, Proxying, Reach, Through};

mod proxy;
#[cfg(test)]
pub(crate) mod resolve;
mod tls;

pub use proxy::{client_for, is_local, through};

const STAGE_TIMEOUT: Duration = Duration::from_secs(5);

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
        #[expect(
            clippy::wildcard_enum_match_arm,
            reason = "std::io::ErrorKind is an upstream open enum; two kinds are diagnosed and the rest are quoted"
        )]
        Err(err) => match err.kind() {
            std::io::ErrorKind::ConnectionRefused => Connected::Refused,
            std::io::ErrorKind::TimedOut => Connected::Silent,
            _ => Connected::Failed(err.to_string()),
        },
    };
    (named, connected)
}

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

    #[test]
    fn a_base_url_without_a_scheme_this_speaks_is_no_host() {
        assert_eq!(split("api.openai.com/v1"), None);
        assert_eq!(split("ftp://files.example.com"), None);
        assert_eq!(split("https://"), None);
    }

    #[test]
    fn a_refusal_is_classified_by_the_stage_it_names() {

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
