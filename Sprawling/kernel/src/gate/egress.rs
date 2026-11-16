// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::error::{AxCode, AxError, GateRefusal};
use crate::secret::SecretSpan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EgressTarget {
    Loopback,
    Private,
    Public {
        host: String,
    },

    Connector {
        label: crate::tool::ServerLabel,
    },
}

#[derive(Debug)]
pub enum EgressOutcome {
    Allow {

        first_public_egress: bool,
    },
    Deny {
        refusal: Box<AxError>,
    },
}

pub fn egress(
    spans: &[SecretSpan],
    target: &EgressTarget,
    prior_public_egress: bool,
) -> EgressOutcome {
    if !spans.is_empty() {
        let offsets: Vec<String> = spans
            .iter()
            .map(|s| format!("{}+{}", s.start, s.len))
            .collect();
        return EgressOutcome::Deny {
            refusal: Box::new(
                AxError::refusal(
                    AxCode::SecretEgress,
                    "send bytes out",
                    format!(
                        "{} secret-shaped span(s) at {}",
                        spans.len(),
                        offsets.join(", ")
                    ),
                    GateRefusal::new(
                        "credentials leave only as secret: references (C13)",
                        format!("the payload carries {} secret-shaped span(s)", spans.len()),
                        "replace each span with its secret:<realm>/<name> reference; \
                         if a real credential already left, rotation is the only remedy",
                    ),
                )
                .with_recovery(
                    "cut the bytes at the offsets above out of the payload and send the \
                     secret:<realm>/<name> reference in their place; `sprawling status --secrets` \
                     lists the references this machine holds",
                ),
            ),
        };
    }

    let leaves_the_machine = matches!(
        target,
        EgressTarget::Public { .. } | EgressTarget::Connector { .. }
    );
    let first_public_egress = leaves_the_machine && !prior_public_egress;
    EgressOutcome::Allow {
        first_public_egress,
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EgressAllowlist {
    suffixes: Vec<String>,
}

impl EgressAllowlist {

    #[must_use]
    pub fn new(entries: Vec<String>) -> EgressAllowlist {
        let mut suffixes: Vec<String> = entries
            .into_iter()
            .map(|entry| entry.trim().trim_start_matches('.').to_ascii_lowercase())
            .filter(|entry| !entry.is_empty())
            .collect();
        suffixes.sort();
        suffixes.dedup();
        EgressAllowlist { suffixes }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.suffixes.is_empty()
    }

    pub fn entries(&self) -> impl Iterator<Item = &str> {
        self.suffixes.iter().map(String::as_str)
    }

    #[must_use]
    pub fn admits(&self, host: &str) -> bool {
        let host = host.trim_end_matches('.').to_ascii_lowercase();
        self.suffixes.iter().any(|allowed| {
            host == *allowed
                || host
                    .strip_suffix(allowed.as_str())
                    .is_some_and(|head| head.ends_with('.'))
        })
    }
}

pub fn host_of(url: &str) -> Result<Option<String>, AxError> {
    let parsed = match url::Url::parse(url.trim()) {
        Ok(parsed) => parsed,
        Err(url::ParseError::RelativeUrlWithoutBase) => return Ok(None),
        Err(_) => return Err(malformed_host()),
    };
    Ok(parsed
        .host_str()
        .filter(|host| !host.is_empty())
        .map(|host| {
            host.trim_start_matches('[')
                .trim_end_matches(']')
                .to_ascii_lowercase()
        }))
}

fn malformed_host() -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "read a url",
        "the url's host is not a host",
    )
    .with_recovery("pass an absolute url, for example `https://example.com/page`")
}

#[must_use]
pub fn target_of(host: &str) -> EgressTarget {
    let bare = host
        .trim()
        .trim_end_matches('.')
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_ascii_lowercase();
    if bare == "localhost" || bare == "::1" {
        return EgressTarget::Loopback;
    }
    let octets = ipv4_octets(&bare);
    if let Some([first, second, ..]) = octets {
        if first == 127 {
            return EgressTarget::Loopback;
        }
        let private = first == 10
            || (first == 192 && second == 168)
            || (first == 172 && (16..=31).contains(&second))
            || (first == 169 && second == 254);
        if private {
            return EgressTarget::Private;
        }
    }
    if let Some(leading) = ipv6_leading(&bare) {

        if (leading & 0xfe00) == 0xfc00 || (leading & 0xffc0) == 0xfe80 {
            return EgressTarget::Private;
        }
    }
    EgressTarget::Public {
        host: bare.to_owned(),
    }
}

fn ipv4_octets(host: &str) -> Option<[u8; 4]> {
    let mut octets = [0u8; 4];
    let mut at = 0usize;
    for part in host.split('.') {
        let octet: u8 = part.parse().ok()?;
        let slot = octets.get_mut(at)?;
        *slot = octet;
        at = at.saturating_add(1);
    }
    if at == 4 { Some(octets) } else { None }
}

fn ipv6_leading(host: &str) -> Option<u16> {
    if !host.contains(':') {
        return None;
    }
    let first = host.split(':').next()?;
    u16::from_str_radix(first, 16).ok()
}

#[must_use]
pub fn egress_target(list: &EgressAllowlist, target: &EgressTarget) -> EgressOutcome {

    if let EgressTarget::Connector { .. } = target {
        return EgressOutcome::Allow {
            first_public_egress: true,
        };
    }
    let EgressTarget::Public { host } = target else {
        return EgressOutcome::Allow {
            first_public_egress: false,
        };
    };
    if list.admits(host) {
        return EgressOutcome::Allow {
            first_public_egress: true,
        };
    }
    let known: Vec<&str> = list.entries().collect();
    let alternative = if known.is_empty() {
        "this building reaches nothing public; add a domain to its egress list, \
         or do the work in a building that has one"
            .to_owned()
    } else {
        format!("reachable domains here: {}", known.join(", "))
    };
    EgressOutcome::Deny {
        refusal: Box::new(
            AxError::refusal(
                AxCode::GateDenied,
                "reach a host outside this building's egress list",
                host.clone(),
                GateRefusal::new(
                    "a building reaches only the domains it names",
                    format!("{host} is not one of them"),
                    alternative,
                ),
            )
            .with_recovery(format!(
                "the egress list lives in the building's `.sprawling/RULES.toml`, \
                 which the User edits; ask the User to add {host} to it"
            )),
        ),
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
mod tests;
