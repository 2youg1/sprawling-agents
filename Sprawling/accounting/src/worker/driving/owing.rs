// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use kernel::{Address, AxCode, AxError, NodeId};

#[derive(Clone)]
pub(in crate::worker) enum Owed {
    Asked,

    Row { addr: Address, node: NodeId },

    Unasked(Unasked),

    Child { parent: Address },
}

#[derive(Clone)]
pub(in crate::worker) enum Unasked {
    Schedule,

    Arrival(kernel::TaintSource),

    Knock,

    Unblocked,
}

impl Unasked {
    pub(in crate::worker) fn because(&self) -> &'static str {
        match self {
            Unasked::Schedule => "the schedule said it was due",
            Unasked::Arrival(_) => "something arrived from outside",
            Unasked::Knock => "a neighbour signalled this resident",
            Unasked::Unblocked => "the approval it was waiting on was answered",
        }
    }

    pub(in crate::worker) fn taint(&self) -> kernel::TaintSet {
        match self {
            Unasked::Arrival(source) => kernel::TaintSet::of(source.clone()),
            Unasked::Schedule | Unasked::Knock | Unasked::Unblocked => kernel::TaintSet::empty(),
        }
    }
}

pub(in crate::worker) struct Owing {
    owed: Owed,
    reply: Arc<wire::Reply>,
    relays: Relays,
}

#[derive(Clone, Default)]
struct Relays {
    chain: KnockChain,
    successions: u32,
}

#[derive(Clone, Default)]
pub(in crate::worker) struct KnockChain {
    hops: u32,
    woken: Arc<AtomicU32>,
}

const CONVERSATION_HOPS_MAX: u32 = 16;

const SUCCESSION_HOPS_MAX: u32 = 64;

const CONVERSATION_RUNS_MAX: u32 = 64;

impl Relays {
    fn after_knock(&self) -> Result<Relays, AxError> {
        let hops = self
            .chain
            .hops
            .checked_add(1)
            .filter(|next| *next <= CONVERSATION_HOPS_MAX)
            .ok_or_else(|| {
                conversation_refused(format!(
                    "a conversation already {} knocks deep",
                    self.chain.hops
                ))
            })?;

        self.chain
            .woken
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |woken| {
                woken
                    .checked_add(1)
                    .filter(|next| *next <= CONVERSATION_RUNS_MAX)
            })
            .map_err(|woken| {
                conversation_refused(format!(
                    "a conversation that has already woken {woken} runs"
                ))
            })?;
        Ok(Relays {
            chain: KnockChain {
                hops,
                woken: Arc::clone(&self.chain.woken),
            },
            successions: self.successions,
        })
    }

    fn after_succession(&self) -> Result<Relays, AxError> {
        let successions = self
            .successions
            .checked_add(1)
            .filter(|next| *next <= SUCCESSION_HOPS_MAX)
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::LoopSuspected,
                    "hand this work to a successor",
                    format!(
                        "a run already {} successions into one piece of work",
                        self.successions
                    ),
                )
                .with_recovery(
                    "let this run's ending stand, or dispatch the work again; the city will not \
                     hand one piece of work over for ever",
                )
            })?;
        Ok(Relays {
            chain: self.chain.clone(),
            successions,
        })
    }
}

fn conversation_refused(subject: String) -> AxError {
    AxError::failure(
        AxCode::LoopSuspected,
        "wake the resident a signal was sent to",
        subject,
    )
    .with_recovery(
        "read the signal already in the room's inbox, or dispatch the work to that resident by \
         hand; the city will not open another run in this chain",
    )
}

#[cfg(test)]
impl KnockChain {
    pub(in crate::worker) fn deep(hops: u32) -> KnockChain {
        KnockChain {
            hops,
            woken: Arc::default(),
        }
    }
}

impl Owing {
    pub(in crate::worker) fn asked(reply: wire::Reply) -> Owing {
        Owing {
            owed: Owed::Asked,
            reply: Arc::new(reply),
            relays: Relays::default(),
        }
    }

    pub(in crate::worker) fn unasked(because: Unasked) -> Owing {
        Owing {
            owed: Owed::Unasked(because),
            reply: Arc::new(wire::Reply::nowhere()),
            relays: Relays::default(),
        }
    }

    pub(in crate::worker) fn knocked(chain: KnockChain) -> Result<Owing, AxError> {
        Ok(Owing {
            owed: Owed::Unasked(Unasked::Knock),
            reply: Arc::new(wire::Reply::nowhere()),
            relays: Relays {
                chain,
                successions: 0,
            }
            .after_knock()?,
        })
    }

    pub(in crate::worker) fn row(addr: Address, node: NodeId) -> Owing {
        Owing {
            owed: Owed::Row { addr, node },
            reply: Arc::new(wire::Reply::nowhere()),
            relays: Relays::default(),
        }
    }

    pub(in crate::worker) fn child(&self, parent: Address) -> Owing {
        Owing {
            owed: Owed::Child { parent },
            reply: Arc::clone(&self.reply),
            relays: self.relays.clone(),
        }
    }

    pub(in crate::worker) fn owed(&self) -> &Owed {
        &self.owed
    }

    pub(in crate::worker) fn reply(&self) -> Arc<wire::Reply> {
        Arc::clone(&self.reply)
    }

    pub(in crate::worker) fn knock_chain(&self) -> &KnockChain {
        &self.relays.chain
    }

    pub(in crate::worker) fn after_succession(&self) -> Result<Owing, AxError> {
        Ok(Owing {
            owed: self.owed.clone(),
            reply: Arc::clone(&self.reply),
            relays: self.relays.after_succession()?,
        })
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::*;

    #[test]
    fn a_chain_of_successions_stops_at_its_ceiling() {
        let mut owing = Owing::asked(wire::Reply::nowhere());
        for _ in 0..SUCCESSION_HOPS_MAX {
            owing = match owing.after_succession() {
                Ok(next) => next,
                Err(refusal) => panic!("a succession below the ceiling was refused: {refusal}"),
            };
        }
        let refusal = owing
            .after_succession()
            .err()
            .expect("a succession at the ceiling must be refused");
        assert_eq!(refusal.code(), &AxCode::LoopSuspected);
        assert!(!refusal.recovery().is_empty());
    }

    #[test]
    fn a_knock_chain_stops_at_its_ceiling() {
        let owed = match Owing::knocked(KnockChain::deep(CONVERSATION_HOPS_MAX - 1)) {
            Ok(owed) => owed,
            Err(refusal) => panic!("a knock below the ceiling was refused: {refusal}"),
        };
        assert_eq!(owed.knock_chain().hops, CONVERSATION_HOPS_MAX);
        let refusal = Owing::knocked(KnockChain::deep(CONVERSATION_HOPS_MAX))
            .err()
            .expect("a knock at the ceiling must be refused");
        assert_eq!(refusal.code(), &AxCode::LoopSuspected);
        assert!(!refusal.recovery().is_empty());
    }

    #[test]
    fn a_delegated_child_inherits_the_chain_it_came_from() {
        let parent = match Owing::knocked(KnockChain::deep(3)) {
            Ok(parent) => parent,
            Err(refusal) => panic!("a knock below the ceiling was refused: {refusal}"),
        };
        let child = parent.child(Address::parse("market/hana").unwrap());
        assert_eq!(child.knock_chain().hops, parent.knock_chain().hops);
    }

    #[test]
    fn a_conversation_that_fans_out_stops_at_its_width() {
        let spoken = KnockChain::default();
        for _ in 0..CONVERSATION_RUNS_MAX {
            if let Err(refusal) = Owing::knocked(spoken.clone()) {
                panic!("a knock below the width was refused: {refusal}");
            }
        }
        let refusal = Owing::knocked(spoken)
            .err()
            .expect("a knock past the width must be refused");
        assert_eq!(refusal.code(), &AxCode::LoopSuspected);
    }
}
