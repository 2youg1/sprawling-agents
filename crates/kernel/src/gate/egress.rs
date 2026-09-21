// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::error::{AxCode, AxError, GateRefusal};
use crate::secret::SecretSpan;

/// address resolution; kernel never resolves names.
///
/// Closed: the kinds of destination a city can distinguish still grow,
/// and every module that decides about a destination has to be shown
/// the new kind by the compiler on the day it is added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EgressTarget {
    Loopback,
    Private,
    Public {
        host: String,
    },
    /// An external server this building configured. The city names the
    /// connector rather than a host because a connector's own
    /// destination is not visible from here: claiming a host would be
    /// inventing one, and claiming loopback would understate where the
    /// bytes can end up.
    Connector {
        label: crate::tool::ServerLabel,
    },
}

/// Deliberately exhaustive.
#[derive(Debug)]
pub enum EgressOutcome {
    Allow {
        /// True exactly once per run: the pipeline hangs the net notice
        /// on this flag (4.3). Loopback and private targets never set it.
        first_public_egress: bool,
    },
    Deny {
        refusal: Box<AxError>,
    },
}

/// The Egress door: secret shapes never leave (C13), and the first
/// public egress of a run is named. The subject counts spans and offsets
/// — the matched bytes never appear (the error message is itself an
/// egress surface).
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
    // A connector counts as leaving, for the same reason a public host
    // does: the run has reached a service outside itself, and the one
    // notice a run gets about that must not depend on whether the city
    // could name the far end.
    let leaves_the_machine = matches!(
        target,
        EgressTarget::Public { .. } | EgressTarget::Connector { .. }
    );
    let first_public_egress = leaves_the_machine && !prior_public_egress;
    EgressOutcome::Allow {
        first_public_egress,
    }
}

/// Which hosts a run may reach. A suffix list, because a building names
/// domains rather than addresses, and an empty list is the honest way to
/// say "nothing public" — a confidential building holds exactly this.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EgressAllowlist {
    suffixes: Vec<String>,
}

impl EgressAllowlist {
    /// Builds the list. Entries are lowercased and stripped of a leading
    /// dot, so `.example.com` and `example.com` are the same rule rather
    /// than two rules with different behaviour.
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

    /// True when `host` is the domain itself or one of its subdomains.
    /// Matching is on label boundaries: `notexample.com` does not match
    /// `example.com`, which a plain suffix test would let through.
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

/// The host a URL names, when it names one.
///
/// One authority for the whole workspace: the browser tool reading a page
/// address and a building reading the address a person declared for
/// their browser are the same question, and two parsers would answer it
/// differently the first time either was corrected. No URL parser: only
/// the authority component is asked for, and `about:blank` or a `file:`
/// path names no host.
///
/// # Errors
/// Refuses an authority this build cannot read, which would leave an
/// egress unjudged.
pub fn host_of(url: &str) -> Result<Option<String>, AxError> {
    let Some((_scheme, rest)) = url.split_once("://") else {
        return Ok(None);
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let after_user = authority
        .rsplit_once('@')
        .map_or(authority, |(_userinfo, host)| host);
    if after_user.is_empty() {
        return Ok(None);
    }
    if let Some(bracketed) = after_user.strip_prefix('[') {
        let (inside, _tail) = bracketed.split_once(']').ok_or_else(malformed_host)?;
        if inside.is_empty() {
            return Err(malformed_host());
        }
        return Ok(Some(inside.to_ascii_lowercase()));
    }
    let host = if let Some((candidate, port)) = after_user.rsplit_once(':') {
        if !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) {
            candidate
        } else {
            after_user
        }
    } else {
        after_user
    };
    if host.is_empty() || host.contains(':') {
        return Err(malformed_host());
    }
    Ok(Some(host.to_ascii_lowercase()))
}

fn malformed_host() -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "read a url",
        "the url's host is not a host",
    )
    .with_recovery("pass an absolute url, for example `https://example.com/page`")
}

/// Classifies a host name or address for the egress doors.
///
/// The one authority for what this city counts as loopback and as the
/// private network around it: the target kind decides whether the first
/// public egress notice is due and whether an attachment to a person's
/// browser is on this machine at all. The host arrives already resolved
/// to a name or an address; this function never resolves anything, and
/// a host it cannot read as an address is public, which is the side
/// that keeps the notice rather than the side that drops it.
///
/// Brackets around an IPv6 literal are stripped, so `[::1]` and `::1`
/// are one address rather than two rules.
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
        // fc00::/7 unique-local and fe80::/10 link-local: the mask keeps
        // the prefix bits and compares the group that carries them.
        if (leading & 0xfe00) == 0xfc00 || (leading & 0xffc0) == 0xfe80 {
            return EgressTarget::Private;
        }
    }
    EgressTarget::Public {
        host: bare.to_owned(),
    }
}

/// Four octets when the string is a dotted IPv4 address, `None`
/// otherwise. Anything dotted but out of range is a name, which lands
/// on the public side.
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

/// The first sixteen-bit group of an IPv6 literal, when this is one.
fn ipv6_leading(host: &str) -> Option<u16> {
    if !host.contains(':') {
        return None;
    }
    let first = host.split(':').next()?;
    u16::from_str_radix(first, 16).ok()
}

/// The destination half of the egress question: may this run reach this
/// host at all? The payload half stays with [`egress`] — "may these bytes
/// leave" and "may this host be reached" are two questions, and a caller
/// that asks only one of them should not look as if it asked both.
///
/// Loopback and private targets are not on the list: a local model or a
/// machine on the same network is not egress, and putting them behind a
/// domain list would mean a confidential building could not reach its own
/// inference server.
#[must_use]
pub fn egress_target(list: &EgressAllowlist, target: &EgressTarget) -> EgressOutcome {
    // A connector is admitted by the building's own server table, frozen
    // at run start; this list answers a different question, about hosts,
    // and a connector label is not a host.
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
                "the egress list lives in the building's `.sprawling/BUILDING.md`; \
                 propose adding {host} to it through the `rules` tool"
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
