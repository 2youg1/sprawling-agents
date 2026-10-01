// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `proposal`: a run offers a change to one stretch of a document as the
//! city holds it, for the person to decide, and takes back a card it
//! offered while the card is still open (accounting-SPEC.md 8-30,
//! `crates/documents/Spec.lean` D35, D36).
//!
//! The card is a line on the history and nothing else: the run never
//! writes the document, and what the person accepts lands through their
//! decision (accounting-SPEC.md 8-22). The line goes through the lane's
//! relay, so the governance fold every decision is judged against is
//! shown it the way it is shown every other line a lane writes.

use std::collections::BTreeMap;
use std::io::Read as _;
use std::sync::{Arc, Mutex};

use documents::{Encoding, Offer, Reading};
use kernel::event::record::{ProposalOffered, ProposalWithdrawn};
use kernel::layout::CityLayout;
use kernel::{
    Address, AxCode, AxError, B3Hash, CostTier, Effect, EventDraft, EventKind, Ledger, Payload,
    RenderIntent, RunId, Temporal, Tool, ToolCall, ToolMeta, ToolName, ToolOutcome, Writes,
};
use runtime::tools::Named;
use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::worker::Relay;
use crate::worker::workbench::{Laying, Site, held};

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
        Err(refused(
            format!("offering {} is not built", asked.path),
            ACTIONS,
        ))
    }

    /// The `proposal_offered` line `asked` makes of the document as the
    /// city holds it now (documents D35).
    fn quoted(&self, asked: &Offering) -> Result<ProposalOffered, AxError> {
        if asked.old.is_empty() {
            return Err(refused(
                "`old` is empty".to_owned(),
                "quote the text the change replaces; to add text, quote the sentence it goes \
                 beside and repeat that sentence in `new`",
            ));
        }
        if asked.old == asked.new {
            return Err(refused(
                "`new` is the same as `old`".to_owned(),
                "suggest text that differs from what the document says",
            ));
        }
        if kernel::Locator::parse(&asked.path).is_ok() {
            return Err(refused(
                format!("`{}` is a locator", asked.path),
                "name the document by its path: a proposal is about the city's copy as it stands",
            ));
        }
        let mut opened = self.reader.open(&asked.path, ACTION)?;
        let Named::File(doc) = opened.named().clone() else {
            return Err(refused(
                format!("`{}` is a stored block, not a document", asked.path),
                "name the document by its path",
            ));
        };
        let mut bytes = Vec::new();
        opened.read_to_end(&mut bytes).map_err(|err| {
            AxError::failure(
                AxCode::StorageFatal,
                ACTION,
                format!("{}: {err}", asked.path),
            )
            .with_recovery("a person has to make the file readable")
        })?;
        let Reading::Text(encoding) = Reading::of(&bytes) else {
            return Err(refused(
                format!("`{}` does not read as text", asked.path),
                "propose changes only to a text document",
            ));
        };
        let (start, end) = stretch(&encoding.decode(&bytes)?, &asked.old, encoding)?;
        Ok(ProposalOffered {
            doc,
            baseline: B3Hash::digest(&bytes),
            start,
            end,
            before: asked.old.clone(),
            after: asked.new.clone(),
        })
    }

    /// Writes the `proposal_withdrawn` line for a card this run offered
    /// and has not withdrawn.
    fn withdraw(&self, asked: &Withdrawing) -> Result<Map<String, Value>, AxError> {
        Err(refused(
            format!("withdrawing {} is not built", asked.proposal),
            ACTIONS,
        ))
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

/// Where `old` lies in `text`, the whole of a version read in
/// `encoding`, as a byte interval of that version: in either UTF-8 the
/// text's bytes are the version's (documents D5), in UTF-16 each code
/// unit is two bytes.
fn stretch(text: &str, old: &str, encoding: Encoding) -> Result<(u64, u64), AxError> {
    let mut places = text.match_indices(old).map(|(at, _)| at);
    let (Some(at), None) = (places.next(), places.next()) else {
        return Err(match text.matches(old).count() {
            0 => refused(
                "`old` is not in the document as the city holds it".to_owned(),
                "read the document and quote it exactly; a run under review quotes the city's \
                 copy, not its own tree's",
            ),
            times => refused(
                format!("`old` appears {times} times in the document"),
                "quote more of the text around the change, so that it appears once",
            ),
        });
    };
    let width = |piece: &str| match encoding {
        Encoding::Utf8 | Encoding::Utf8Bom => piece.len(),
        Encoding::Utf16Le | Encoding::Utf16Be => piece.encode_utf16().count().saturating_mul(2),
    };
    let before = text.get(..at).ok_or_else(|| {
        refused(
            "`old` does not begin at a character".to_owned(),
            "quote whole characters",
        )
    })?;
    let start = width(before);
    let end = start.checked_add(width(old)).ok_or_else(|| {
        refused(
            "the stretch ends past what a document can hold".to_owned(),
            "quote a shorter stretch",
        )
    })?;
    Ok((offset(start)?, offset(end)?))
}

/// A byte count as the offset a line carries.
fn offset(count: usize) -> Result<u64, AxError> {
    u64::try_from(count).map_err(|_| {
        refused(
            format!("offset {count} does not fit a line"),
            "quote a shorter stretch",
        )
    })
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
