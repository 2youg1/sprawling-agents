// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, AxCode, AxError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reflex {
    Discard,

    Notify,

    Light,

    Full,
}

impl Reflex {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Reflex::Discard => "discard",
            Reflex::Notify => "notify",
            Reflex::Light => "light",
            Reflex::Full => "full",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arrival {
    pub source: String,
    pub subject: String,

    pub tainted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub matches: String,
    pub landing: Address,
    pub reflex: Reflex,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Landing {
    pub addr: Address,
    pub reflex: Reflex,

    pub because: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Triage {
    rules: Vec<Rule>,
    fallback: Address,
}

impl Triage {
    pub fn new(rules: Vec<Rule>, fallback: Address) -> Result<Triage, AxError> {
        for rule in &rules {
            if rule.matches.trim().is_empty() {
                return Err(AxError::failure(
                    AxCode::ConfigInvalid,
                    "build a triage table",
                    "a rule with nothing to match".to_owned(),
                )
                .with_recovery(
                    "give the rule something to match, or change the fallback address, which is \
                     what catches everything else",
                ));
            }
        }
        Ok(Triage { rules, fallback })
    }

    #[must_use]
    pub fn decide(&self, arrival: &Arrival) -> Landing {
        let haystack = format!("{} {}", arrival.source, arrival.subject).to_lowercase();
        for rule in &self.rules {
            if !haystack.contains(&rule.matches.trim().to_lowercase()) {
                continue;
            }

            let reflex = if arrival.tainted && matches!(rule.reflex, Reflex::Light | Reflex::Full) {
                Reflex::Notify
            } else {
                rule.reflex
            };
            let because = if reflex == rule.reflex {
                format!("rule `{}`", rule.matches)
            } else {
                format!(
                    "rule `{}`, held at notify because it came from outside",
                    rule.matches
                )
            };
            return Landing {
                addr: rule.landing.clone(),
                reflex,
                because,
            };
        }
        Landing {
            addr: self.fallback.clone(),
            reflex: Reflex::Notify,
            because: "no rule matched, so the User reads it".to_owned(),
        }
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

    fn table() -> Triage {
        Triage::new(
            vec![
                Rule {
                    matches: "build failed".to_owned(),
                    landing: Address::parse("lab/ci").unwrap(),
                    reflex: Reflex::Full,
                },
                Rule {
                    matches: "newsletter".to_owned(),
                    landing: Address::parse("lab").unwrap(),
                    reflex: Reflex::Discard,
                },
            ],
            Address::parse("hall").unwrap(),
        )
        .unwrap()
    }

    fn arrival(source: &str, subject: &str, tainted: bool) -> Arrival {
        Arrival {
            source: source.to_owned(),
            subject: subject.to_owned(),
            tainted,
        }
    }

    #[test]
    fn the_same_arrival_twice_lands_the_same_way() {
        let table = table();
        let one = arrival("ci@lab", "build failed on main", false);
        assert_eq!(table.decide(&one), table.decide(&one));
        let landing = table.decide(&one);
        assert_eq!(landing.addr.as_str(), "lab/ci");
        assert_eq!(landing.reflex, Reflex::Full);
    }

    #[test]
    fn nothing_matching_is_a_person_reading_it_rather_than_an_error() {
        let landing = table().decide(&arrival("someone", "a question", false));
        assert_eq!(landing.addr.as_str(), "hall");
        assert_eq!(landing.reflex, Reflex::Notify);
        assert!(landing.because.contains("no rule matched"));
    }

    #[test]
    fn outside_material_never_starts_work_by_itself() {
        let landing = table().decide(&arrival("ci@lab", "build failed on main", true));
        assert_eq!(
            landing.reflex,
            Reflex::Notify,
            "a rule may route tainted material; it may not act on it"
        );
        assert_eq!(landing.addr.as_str(), "lab/ci", "the address still holds");
        assert!(landing.because.contains("outside"));
    }

    #[test]
    fn a_rule_that_matches_everything_is_refused_at_the_table() {
        let refused = Triage::new(
            vec![Rule {
                matches: "  ".to_owned(),
                landing: Address::parse("lab").unwrap(),
                reflex: Reflex::Full,
            }],
            Address::parse("hall").unwrap(),
        );
        assert!(refused.is_err());
    }

    #[test]
    fn the_table_reads_top_to_bottom() {
        let table = Triage::new(
            vec![
                Rule {
                    matches: "build".to_owned(),
                    landing: Address::parse("lab/first").unwrap(),
                    reflex: Reflex::Light,
                },
                Rule {
                    matches: "build failed".to_owned(),
                    landing: Address::parse("lab/second").unwrap(),
                    reflex: Reflex::Full,
                },
            ],
            Address::parse("hall").unwrap(),
        )
        .unwrap();
        assert_eq!(
            table
                .decide(&arrival("ci", "build failed", false))
                .addr
                .as_str(),
            "lab/first",
            "the person wrote the table in an order, and that order is the answer"
        );
    }
}
