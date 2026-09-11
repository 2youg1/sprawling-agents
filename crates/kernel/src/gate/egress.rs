// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::error::{AxCode, AxError, GateRefusal};
use crate::secret::SecretSpan;

/// address resolution; kernel never resolves names.
///
/// Non-exhaustive as the specification has always said it is: the kinds
/// of destination a city can distinguish grow.
#[non_exhaustive]
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
            refusal: Box::new(AxError::refusal(
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
            )),
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
        refusal: Box::new(AxError::refusal(
            AxCode::GateDenied,
            "reach a host outside this building's egress list",
            host.clone(),
            GateRefusal::new(
                "a building reaches only the domains it names",
                format!("{host} is not one of them"),
                alternative,
            ),
        )),
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
    #[test]
    fn an_allowlist_matches_on_label_boundaries_not_on_text() {
        let list = EgressAllowlist::new(vec![".Example.com".to_owned(), "docs.rs".to_owned()]);
        assert!(list.admits("example.com"));
        assert!(list.admits("api.example.com"));
        assert!(list.admits("EXAMPLE.COM"), "hosts are case-insensitive");
        assert!(
            !list.admits("notexample.com"),
            "a suffix test would allow this"
        );
        assert!(!list.admits("example.com.evil.test"));
    }

    #[test]
    fn a_host_outside_the_list_is_refused_in_three_parts_that_name_the_way_out() {
        let list = EgressAllowlist::new(vec!["example.com".to_owned()]);
        let target = EgressTarget::Public {
            host: "pastebin.test".to_owned(),
        };
        let EgressOutcome::Deny { refusal } = egress_target(&list, &target) else {
            panic!("an unlisted host is refused");
        };
        assert!(refusal.to_string().contains("pastebin.test"));
        let gate = refusal.gate().unwrap();
        assert!(gate.alternative().contains("example.com"));
    }

    #[test]
    fn an_empty_list_reaches_nothing_public_and_says_what_to_do_instead() {
        let list = EgressAllowlist::default();
        let EgressOutcome::Deny { refusal } = egress_target(
            &list,
            &EgressTarget::Public {
                host: "example.com".to_owned(),
            },
        ) else {
            panic!("a building with no list reaches nothing public");
        };
        let gate = refusal.gate().unwrap();
        assert!(gate.alternative().contains("building that has one"));
    }

    #[test]
    fn local_and_private_targets_are_not_egress_at_all() {
        let list = EgressAllowlist::default();
        assert!(matches!(
            egress_target(&list, &EgressTarget::Loopback),
            EgressOutcome::Allow { .. }
        ));
        assert!(
            matches!(
                egress_target(&list, &EgressTarget::Private),
                EgressOutcome::Allow { .. }
            ),
            "a confidential building must still reach its own inference server"
        );
    }

    #[test]
    fn the_egress_door_denies_spans_and_flags_the_first_public_hop() {
        let span = SecretSpan {
            start: 10,
            len: 40,
            provider: Some("anthropic"),
        };
        let outcome = egress(
            std::slice::from_ref(&span),
            &EgressTarget::Public {
                host: "api.example.com".into(),
            },
            false,
        );
        match outcome {
            EgressOutcome::Deny { refusal } => {
                assert_eq!(refusal.code(), &AxCode::SecretEgress);
                assert!(refusal.subject().contains("10+40"));
                assert!(!refusal.subject().contains("sk-ant"), "never echo bytes");
                assert!(refusal.gate().unwrap().alternative().contains("rotation"));
            }
            EgressOutcome::Allow { .. } => panic!("spans must deny"),
        }
        match egress(
            &[],
            &EgressTarget::Public {
                host: "x.dev".into(),
            },
            false,
        ) {
            EgressOutcome::Allow {
                first_public_egress,
            } => assert!(first_public_egress),
            EgressOutcome::Deny { .. } => panic!("clean egress allows"),
        }
        match egress(
            &[],
            &EgressTarget::Public {
                host: "x.dev".into(),
            },
            true,
        ) {
            EgressOutcome::Allow {
                first_public_egress,
            } => assert!(!first_public_egress),
            EgressOutcome::Deny { .. } => panic!(),
        }
        match egress(&[], &EgressTarget::Loopback, false) {
            EgressOutcome::Allow {
                first_public_egress,
            } => assert!(!first_public_egress, "localhost is not the internet"),
            EgressOutcome::Deny { .. } => panic!(),
        }
    }
}
