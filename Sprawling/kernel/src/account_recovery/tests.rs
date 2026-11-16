// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use proptest::prelude::*;

use super::*;
use crate::error::AxCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecoveryEvent {
    Busy,
    Unusable,
    Refused,
    Lost,
    Repair,
}

fn failure(event: RecoveryEvent) -> Option<AxError> {
    let draft = AxError::failure(AxCode::Provider, "call model", "account");
    match event {
        RecoveryEvent::Busy => Some(draft.retriable().with_recovery("r")),
        RecoveryEvent::Unusable => Some(draft.account_unusable().with_recovery("r")),
        RecoveryEvent::Refused => Some(draft.with_recovery("r")),
        RecoveryEvent::Lost => Some(draft.effect_unknown().with_recovery("r")),
        RecoveryEvent::Repair => None,
    }
}

fn label(name: &str) -> ServerLabel {
    ServerLabel::parse(name).unwrap()
}

fn several(names: &[&str], retries: AccountRetries) -> Roster {
    Roster::Several {
        accounts: names.iter().map(|name| label(name)).collect(),
        retries,
    }
}

fn decisions(mut round: AccountRound, events: &[RecoveryEvent]) -> Vec<(RecoveryEvent, Option<AccountStep>)> {
    let mut out = Vec::new();
    for &event in events {
        match failure(event) {
            None if round.admits_repair() => {
                out.push((event, Some(AccountStep::Resend)));
                round = round.repaired();
            }
            None => out.push((event, None)),
            Some(err) => {
                let step = round.next(&err);
                out.push((event, Some(step.clone())));
                if let AccountStep::Stop { .. } = step {
                    break;
                }
                round = round.apply(&step);
            }
        }
    }
    out
}

type Decided = Option<(Option<ServerLabel>, Vec<(RecoveryEvent, Option<AccountStep>)>)>;

struct Trace<'a> {
    roster: Roster,
    missing: &'a [&'a str],
    cap: Retries,
    held: Option<&'a str>,
    events: &'a [RecoveryEvent],
}

fn vector(
    Trace {
        roster,
        missing,
        cap,
        held,
        events,
    }: Trace<'_>,
) -> Decided {
    let usable = match &roster {
        Roster::Single => BTreeSet::new(),
        Roster::Several { accounts, .. } => accounts
            .iter()
            .filter(|account| !missing.contains(&account.as_str()))
            .cloned()
            .collect(),
    };
    let round = AccountRound::start(roster, usable, cap, held.map(label)).ok()?;
    Some((round.current().cloned(), decisions(round, events)))
}

fn resend() -> Option<AccountStep> {
    Some(AccountStep::Resend)
}

fn switch(to: &str) -> Option<AccountStep> {
    Some(AccountStep::Switch { to: label(to) })
}

fn stop(why: RoundEnd) -> Option<AccountStep> {
    Some(AccountStep::Stop { why })
}

#[test]
fn the_lean_trace_vectors_replay_one_by_one() {
    use RecoveryEvent::{Busy, Lost, Refused, Repair, Unusable};
    let two = || several(&["a", "b"], AccountRetries::Two);
    let a = Some(label("a"));
    let b = Some(label("b"));
    let found = [
        vector(Trace {
            roster: two(),
            missing: &[],
            cap: Retries::UntilHalted,
            held: None,
            events: &[Busy, Busy, Busy, Busy],
        }),
        vector(Trace {
            roster: two(),
            missing: &[],
            cap: Retries::UntilHalted,
            held: None,
            events: &[Unusable, Unusable],
        }),
        vector(Trace {
            roster: two(),
            missing: &[],
            cap: Retries::UntilHalted,
            held: None,
            events: &[Refused],
        }),
        vector(Trace {
            roster: two(),
            missing: &[],
            cap: Retries::UntilHalted,
            held: None,
            events: &[Lost, Lost, Lost],
        }),
        vector(Trace {
            roster: two(),
            missing: &[],
            cap: Retries::AtMost(1),
            held: None,
            events: &[Busy, Busy],
        }),
        vector(Trace {
            roster: several(&["a", "b", "c"], AccountRetries::One),
            missing: &["b"],
            cap: Retries::UntilHalted,
            held: Some("c"),
            events: &[Busy, Busy, Unusable],
        }),
        vector(Trace {
            roster: several(&["a", "b"], AccountRetries::One),
            missing: &[],
            cap: Retries::UntilHalted,
            held: Some("b"),
            events: &[Repair, Busy, Repair],
        }),
        vector(Trace {
            roster: two(),
            missing: &["a", "b"],
            cap: Retries::UntilHalted,
            held: None,
            events: &[],
        }),
        vector(Trace {
            roster: Roster::Single,
            missing: &[],
            cap: Retries::AtMost(2),
            held: None,
            events: &[Busy, Lost, Busy],
        }),
        vector(Trace {
            roster: Roster::Single,
            missing: &[],
            cap: Retries::UntilHalted,
            held: None,
            events: &[Repair, Unusable],
        }),
        vector(Trace {
            roster: two(),
            missing: &[],
            cap: Retries::UntilHalted,
            held: Some("b"),
            events: &[Busy],
        }),
    ];
    let lean: [Decided; 11] = [
        Some((
            a.clone(),
            vec![
                (Busy, resend()),
                (Busy, resend()),
                (Busy, switch("b")),
                (Busy, resend()),
            ],
        )),
        Some((
            a.clone(),
            vec![
                (Unusable, switch("b")),
                (Unusable, stop(RoundEnd::Exhausted)),
            ],
        )),
        Some((a.clone(), vec![(Refused, stop(RoundEnd::Refused))])),
        Some((
            a.clone(),
            vec![
                (Lost, resend()),
                (Lost, resend()),
                (Lost, stop(RoundEnd::Unknown)),
            ],
        )),
        Some((a, vec![(Busy, resend()), (Busy, stop(RoundEnd::Cap))])),
        Some((
            Some(label("c")),
            vec![
                (Busy, resend()),
                (Busy, switch("a")),
                (Unusable, stop(RoundEnd::Exhausted)),
            ],
        )),
        Some((
            b.clone(),
            vec![(Repair, resend()), (Busy, switch("a")), (Repair, resend())],
        )),
        None,
        Some((
            None,
            vec![
                (Busy, resend()),
                (Lost, resend()),
                (Busy, stop(RoundEnd::Cap)),
            ],
        )),
        Some((
            None,
            vec![(Repair, resend()), (Unusable, stop(RoundEnd::Refused))],
        )),
        Some((b, vec![(Busy, resend())])),
    ];
    assert_eq!(found, lean);
}

fn switched(step: &AccountStep) -> bool {
    matches!(step, AccountStep::Switch { .. })
}

fn event() -> impl Strategy<Value = RecoveryEvent> {
    prop_oneof![
        Just(RecoveryEvent::Busy),
        Just(RecoveryEvent::Unusable),
        Just(RecoveryEvent::Refused),
        Just(RecoveryEvent::Lost),
        Just(RecoveryEvent::Repair),
    ]
}

fn cap() -> impl Strategy<Value = Retries> {
    prop_oneof![
        Just(Retries::UntilHalted),
        (0u32..8).prop_map(Retries::AtMost)
    ]
}

const NAMES: [&str; 6] = ["a", "b", "c", "d", "e", "z"];

proptest! {

    #[test]
    fn every_round_keeps_the_lean_properties(
        listed in 2usize..=5,
        missing in proptest::collection::vec(any::<bool>(), 5),
        two in any::<bool>(),
        cap in cap(),
        held in proptest::option::of(0usize..6),
        events in proptest::collection::vec(event(), 0..40),
    ) {
        let accounts: Vec<ServerLabel> = NAMES[..listed].iter().map(|name| label(name)).collect();
        let usable: BTreeSet<ServerLabel> = accounts
            .iter()
            .zip(&missing)
            .filter(|(_, gone)| !**gone)
            .map(|(account, _)| account.clone())
            .collect();
        let retries = if two { AccountRetries::Two } else { AccountRetries::One };
        let budget = if two { 3 } else { 2 };
        let held = held.map(|at| label(NAMES[at]));
        let opened = AccountRound::start(
            Roster::Several { accounts: accounts.clone(), retries },
            usable.clone(),
            cap,
            held.clone(),
        );
        if usable.is_empty() {
            prop_assert_eq!(opened, Err(RoundEnd::Exhausted));
            return Ok(());
        }
        let mut round = opened.unwrap();
        let first = held
            .filter(|account| usable.contains(account))
            .or_else(|| accounts.iter().find(|account| usable.contains(*account)).cloned());
        prop_assert_eq!(round.current().cloned(), first.clone());
        let mut on = first.unwrap();
        let mut visited = vec![on.clone()];
        let mut sends = BTreeMap::from([(on.clone(), 1u32)]);
        let mut total = 1u32;
        for event in events {
            let Some(err) = failure(event) else {
                if round.admits_repair() {
                    *sends.get_mut(&on).unwrap() += 1;
                    total += 1;
                    round = round.repaired();
                }
                continue;
            };
            let step = round.next(&err);
            match event {
                RecoveryEvent::Refused => {
                    prop_assert_eq!(&step, &AccountStep::Stop { why: RoundEnd::Refused });
                }
                RecoveryEvent::Unusable => prop_assert_ne!(&step, &AccountStep::Resend),
                RecoveryEvent::Lost => prop_assert_ne!(switched(&step), true),
                RecoveryEvent::Busy | RecoveryEvent::Repair => {}
            }
            match &step {
                AccountStep::Resend => {
                    *sends.get_mut(&on).unwrap() += 1;
                }
                AccountStep::Switch { to } => {
                    prop_assert!(
                        usable.contains(to) && !visited.contains(to),
                        "{to} was tried already or is not redeemable"
                    );
                    visited.push(to.clone());
                    sends.insert(to.clone(), 1);
                    on = to.clone();
                }
                AccountStep::Stop { .. } => break,
            }
            total += 1;
            round = round.apply(&step);
            prop_assert_eq!(round.current(), Some(&on));
        }
        prop_assert!(sends.values().all(|&sent| sent <= budget), "{sends:?} over {budget}");
        prop_assert!(total <= u32::try_from(listed).unwrap() * budget);
        if let Retries::AtMost(retries) = cap {
            prop_assert!(total <= retries + 1, "{total} sends under AtMost({retries})");
        }
    }

    #[test]
    fn a_single_account_round_reads_retry_alone(
        cap in cap(),
        events in proptest::collection::vec(event(), 0..40),
    ) {
        let mut round = AccountRound::start(Roster::Single, BTreeSet::new(), cap, None).unwrap();
        prop_assert_eq!(round.current(), None);
        let mut total = 1u32;
        for event in events {
            let Some(err) = failure(event) else {
                prop_assert!(round.admits_repair());
                round = round.repaired();
                continue;
            };
            let step = round.next(&err);
            match event {
                RecoveryEvent::Refused | RecoveryEvent::Unusable => {
                    prop_assert_eq!(&step, &AccountStep::Stop { why: RoundEnd::Refused });
                }
                RecoveryEvent::Busy | RecoveryEvent::Lost | RecoveryEvent::Repair => {
                    prop_assert_ne!(switched(&step), true);
                }
            }
            if let AccountStep::Stop { .. } = step {
                break;
            }
            total += 1;
            round = round.apply(&step);
        }
        if let Retries::AtMost(retries) = cap {
            prop_assert!(total <= retries + 1);
        }
    }
}
