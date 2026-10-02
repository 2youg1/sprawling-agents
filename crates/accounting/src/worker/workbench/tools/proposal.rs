// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `proposal`: a run offers a change to one stretch of a document as the
//! city holds it, for the person to decide, and takes back a card it
//! offered while the card is still open (`crates/accounting/spec/Worker/Workbench/Tools.lean` §8-30,
//! `crates/documents/Spec.lean` D35, D36).
//!
//! The card is a line on the history and nothing else: the run never
//! writes the document, and what the person accepts lands through their
//! decision (`crates/accounting/spec/Worker/Commanding/Saving.lean` §8-22). The line goes through the lane's
//! relay, so the governance fold every decision is judged against is
//! shown it the way it is shown every other line a lane writes.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use documents::Offer;
use kernel::event::record::ProposalWithdrawn;
use kernel::layout::CityLayout;
use kernel::{
    Address, AxCode, AxError, B3Hash, CostTier, Effect, EventDraft, EventKind, Ledger, Payload,
    RenderIntent, RunId, Temporal, Tool, ToolCall, ToolMeta, ToolName, ToolOutcome, Writes,
};
use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::worker::Relay;
use crate::worker::workbench::{Laying, Site, held};

mod quoting;

/// What every refusal of this tool names as the action that failed.
const ACTION: &str = "proposal";

/// What a call that is not one of the two actions is told to do instead.
const ACTIONS: &str = "offer with `path`, `old` and `new`; withdraw with `proposal`, the id an \
                       offer answered with";

/// The face a `proposal` tool files its lines through, and the clock
/// that stamps them: what laying out a bench takes from the worker for
/// this tool.
pub(in crate::worker) struct Proposing {
    relay: Relay,
    clock: Arc<dyn crate::Clock + Send + Sync>,
}

impl crate::worker::RunWorker {
    /// What a `proposal` tool laid out for a run of this city takes from
    /// the worker: a relay onto its ledger, and its clock.
    pub(in crate::worker) fn proposing(&self) -> Proposing {
        Proposing {
            relay: self.relay(),
            clock: Arc::clone(&self.clock),
        }
    }
}

/// Who a line of this tool is filed under, and what stamps it.
struct Filing {
    run: RunId,
    who: String,
    room: Address,
    clock: Arc<dyn crate::Clock + Send + Sync>,
}

/// What became of a card this run offered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Held {
    Open,
    Withdrawn,
}

/// The ledger this tool writes and the cards it wrote there, under one
/// lock: a card enters the book only once its line is on the history.
struct Desk<L> {
    ledger: L,
    cards: BTreeMap<B3Hash, Held>,
}

/// The tool: what the run may read, who its lines are filed under, and
/// the cards it offered.
pub(super) struct ProposalTool<L> {
    reader: runtime::BoundReader,
    filing: Filing,
    desk: Mutex<Desk<L>>,
    meta: ToolMeta,
}

/// The arguments of `offer`, besides the action.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Offering {
    path: String,
    old: String,
    new: String,
}

/// The arguments of `withdraw`, besides the action.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Withdrawing {
    proposal: B3Hash,
}

impl Laying {
    /// The proposal tool for a run in `room`, reading documents as the
    /// city holds them through the read bound `read` asks. It is the
    /// last built-in on every building's bench, after `playback`: a
    /// run's suggestion about a document, which the person decides.
    ///
    /// # Errors
    /// Propagates [`ProposalTool::new`]'s refusal.
    pub(super) fn proposal_tool(
        &self,
        site: &Site,
        room: &Address,
        bound: &runtime::ReadBound,
    ) -> Result<ProposalTool<Relay>, AxError> {
        ProposalTool::new(
            runtime::BoundReader::new(
                &self.city_root,
                Arc::clone(bound),
                &CityLayout::new(&self.city_root).cas(),
            ),
            Filing {
                run: site.run_id,
                who: site.who.clone(),
                room: room.clone(),
                clock: Arc::clone(&self.proposing.clock),
            },
            self.proposing.relay.clone(),
        )
    }
}

impl<L: Ledger> ProposalTool<L> {
    /// # Errors
    /// Refuses a name or a parameter schema that does not build, which
    /// the literals below cannot produce.
    fn new(
        reader: runtime::BoundReader,
        filing: Filing,
        ledger: L,
    ) -> Result<ProposalTool<L>, AxError> {
        let params = json!({
            "type": "object",
            "properties": {
                "action": {"type": "string", "enum": ["offer", "withdraw"]},
                "path": {
                    "type": "string",
                    "description": "offer: the document, relative to the city root",
                },
                "old": {
                    "type": "string",
                    "description": "offer: the exact text to change, as the city's copy of the \
                                    document holds it; it must appear there exactly once",
                },
                "new": {"type": "string", "description": "offer: the text you suggest in its place"},
                "proposal": {
                    "type": "string",
                    "description": "withdraw: the id your offer answered with",
                },
            },
            "required": ["action"],
        });
        let Value::Object(params) = params else {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                ACTION,
                "the parameter schema is not an object",
            )
            .with_recovery("report this against accounting::worker::workbench::tools::proposal"));
        };
        let room = filing.room.clone();
        Ok(ProposalTool {
            reader,
            filing,
            desk: Mutex::new(Desk {
                ledger,
                cards: BTreeMap::new(),
            }),
            meta: ToolMeta {
                name: ToolName::parse(ACTION)?,
                disclosure: "Suggest a change to a document for the person to decide. `offer` \
                             quotes `old` exactly as the city's copy of the document holds it \
                             - not your own tree's copy - and suggests `new`; the person sees a \
                             card and accepts or rejects it sentence by sentence, and nothing in \
                             the document changes until they do. `withdraw` takes back a card \
                             you offered while it is still open."
                    .to_owned(),
                params: Payload::new(params)?,
                // A card is a line on the history, so an offer and the
                // withdrawal after it run in order, never ahead.
                effect: Effect::Write { domain: room },
                // One read of one document.
                cost_tier: CostTier::Light,
                timeout: None,
                render: RenderIntent::Generic,
                temporal: Temporal::Timeless,
            },
        })
    }

    /// Writes the `proposal_offered` line for the change `asked` quotes,
    /// unless this run already offered the same card.
    fn offer(&self, asked: &Offering) -> Result<Map<String, Value>, AxError> {
        let offered = quoting::offered(&self.reader, asked)?;
        let id = Offer::of(self.filing.run, &offered)?.id();
        let mut desk = held(&self.desk, "take the proposal desk")?;
        match desk.cards.get(&id).copied() {
            // The same card offered again is the same card (documents
            // D13): the history already holds it.
            Some(Held::Open) => {}
            Some(Held::Withdrawn) => {
                return Err(refused(
                    format!("proposal {id} was withdrawn, and a card is handled once"),
                    "offer a different change, or leave this one withdrawn",
                ));
            }
            None => {
                desk.file(
                    &self.filing,
                    EventKind::ProposalOffered,
                    Payload::of(&offered)?,
                )?;
                desk.cards.insert(id, Held::Open);
            }
        }
        let mut out = Map::new();
        out.insert("proposal".to_owned(), id.to_string().into());
        out.insert("doc".to_owned(), offered.doc.as_str().into());
        out.insert("baseline".to_owned(), offered.baseline.to_string().into());
        out.insert("start".to_owned(), offered.start.into());
        out.insert("end".to_owned(), offered.end.into());
        Ok(out)
    }

    /// Writes the `proposal_withdrawn` line for a card this run offered
    /// and has not withdrawn.
    fn withdraw(&self, asked: &Withdrawing) -> Result<Map<String, Value>, AxError> {
        let id = asked.proposal;
        let mut desk = held(&self.desk, "take the proposal desk")?;
        match desk.cards.get(&id).copied() {
            Some(Held::Open) => {
                desk.file(
                    &self.filing,
                    EventKind::ProposalWithdrawn,
                    Payload::of(&ProposalWithdrawn { proposal: id })?,
                )?;
                desk.cards.insert(id, Held::Withdrawn);
            }
            Some(Held::Withdrawn) => {
                return Err(refused(
                    format!("proposal {id} was withdrawn already"),
                    "nothing to do: the card is closed",
                ));
            }
            None => {
                return Err(refused(
                    format!("proposal {id} is not a card this run offered"),
                    "withdraw only a card your own offer answered with; the person decides \
                     every other card",
                ));
            }
        }
        let mut out = Map::new();
        out.insert("proposal".to_owned(), id.to_string().into());
        out.insert("withdrawn".to_owned(), true.into());
        Ok(out)
    }
}

impl<L: Ledger> Desk<L> {
    /// Appends one line, filed under `filing`.
    fn file(&mut self, filing: &Filing, kind: EventKind, data: Payload) -> Result<(), AxError> {
        self.ledger.append(EventDraft {
            run: filing.run,
            t: filing.clock.now()?,
            who: filing.who.clone(),
            addr: Some(filing.room.clone()),
            kind,
            data,
            ig: false,
        })?;
        Ok(())
    }
}

impl<L: Ledger + Send> Tool for ProposalTool<L> {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    fn invoke(&self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                ACTION,
                format!("call routed to the wrong tool: {}", call.name.as_str()),
            )
            .with_recovery(format!(
                "call `{}`, the name this tool answers to",
                self.meta.name.as_str()
            )));
        }
        let mut args = call.args.as_map().clone();
        let result = match args.remove("action") {
            Some(Value::String(action)) if action == "offer" => {
                self.offer(&arguments::<Offering>(args)?)?
            }
            Some(Value::String(action)) if action == "withdraw" => {
                self.withdraw(&arguments::<Withdrawing>(args)?)?
            }
            Some(_) | None => {
                return Err(refused(
                    "`action` is neither `offer` nor `withdraw`".to_owned(),
                    ACTIONS,
                ));
            }
        };
        Ok(ToolOutcome {
            result: Payload::new(result)?,
            attachments: Vec::new(),
        })
    }

    /// A card is a line on the history; nothing in the tree moves.
    fn writes(&self, _call: &ToolCall) -> Writes {
        Writes::Nothing
    }
}

/// One action's arguments, every field known.
fn arguments<T: for<'de> Deserialize<'de>>(args: Map<String, Value>) -> Result<T, AxError> {
    serde_json::from_value(Value::Object(args)).map_err(|err| {
        AxError::failure(AxCode::InvalidArgs, ACTION, err.to_string()).with_recovery(ACTIONS)
    })
}

fn refused(subject: String, recovery: &str) -> AxError {
    AxError::failure(AxCode::InvalidArgs, ACTION, subject).with_recovery(recovery)
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
