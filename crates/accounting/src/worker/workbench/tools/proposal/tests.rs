// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a run's `proposal` calls write to the history, and what they
//! leave alone (`crates/accounting/spec/Worker/Workbench/Tools.lean` §8-30).

use std::path::Path;
use std::sync::{Arc, Mutex};

use documents::{Offer, SliceKind, decide};
use kernel::event::record::{ProposalOffered, ProposalWithdrawn, SliceVerdict, Verdict};
use kernel::{
    Address, AxCode, AxError, B3Hash, EventDraft, EventKind, EventRef, Payload, RunId, TimeMs,
    Tool as _, ToolCall, ToolName,
};
use serde_json::{Value, json};

use super::{Filing, ProposalTool};

/// The lines every tool of a test wrote, in the order they wrote them.
#[derive(Clone, Default)]
struct Written(Arc<Mutex<Vec<EventDraft>>>);

impl kernel::Ledger for Written {
    fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError> {
        let mut lines = self.0.lock().unwrap();
        lines.push(draft.clone());
        let seq = u64::try_from(lines.len()).unwrap();
        Ok(
            kernel::EventRecord::from_draft(draft, kernel::Seq::new(seq), kernel::GENESIS_PREV)
                .to_ref(),
        )
    }
}

impl Written {
    fn lines(&self) -> Vec<EventDraft> {
        self.0.lock().unwrap().clone()
    }
}

/// A clock that always reads the same moment, so a line is compared
/// whole.
struct Stopped;

const MOMENT: u64 = 7;

impl crate::Clock for Stopped {
    fn now(&self) -> Result<TimeMs, AxError> {
        Ok(TimeMs::new(MOMENT))
    }
}

const ROOM: &str = "lab/room1";

const NOTES: &str = "The kiln is fired at noon. It cools overnight.\n";

const OLD: &str = "It cools overnight.";

const NEW: &str = "It cools by dawn.";

fn run(n: u8) -> RunId {
    RunId::from_bytes([n; 16])
}

fn who(n: u8) -> String {
    format!("potter@lab.{n}")
}

/// The tool a run numbered `n` is given, over a city at `city` whose
/// every building is open to it.
fn tool(city: &Path, n: u8, written: &Written) -> ProposalTool<Written> {
    let open: runtime::ReadBound = Arc::new(|_: &Address| kernel::ReadVerdict::Open);
    ProposalTool::new(
        runtime::BoundReader::new(city, open, &city.join("cas")),
        Filing {
            run: run(n),
            who: who(n),
            room: Address::parse(ROOM).unwrap(),
            clock: Arc::new(Stopped),
        },
        written.clone(),
    )
    .unwrap()
}

fn call(args: &Value) -> ToolCall {
    ToolCall {
        id: "call-1".to_owned(),
        name: ToolName::parse("proposal").unwrap(),
        args: Payload::new(args.as_object().cloned().unwrap()).unwrap(),
    }
}

fn offering(path: &str, old: &str, new: &str) -> ToolCall {
    call(&json!({ "action": "offer", "path": path, "old": old, "new": new }))
}

fn withdrawing(id: B3Hash) -> ToolCall {
    call(&json!({ "action": "withdraw", "proposal": id.to_string() }))
}

/// The line run `n` files for `kind`.
fn line(n: u8, kind: EventKind, data: Payload) -> EventDraft {
    EventDraft {
        run: run(n),
        t: TimeMs::new(MOMENT),
        who: who(n),
        addr: Some(Address::parse(ROOM).unwrap()),
        kind,
        data,
        ig: false,
    }
}

/// Writes `bytes` at `doc` under `city`.
fn lay(city: &Path, doc: &str, bytes: &[u8]) -> std::path::PathBuf {
    let path = doc
        .split('/')
        .fold(city.to_path_buf(), |at, part| at.join(part));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, bytes).unwrap();
    path
}

/// The notes as the first run offers to change them.
fn notes_offered() -> ProposalOffered {
    ProposalOffered {
        doc: Address::parse("lab/notes.md").unwrap(),
        baseline: B3Hash::digest(NOTES.as_bytes()),
        start: 27,
        end: 46,
        before: OLD.to_owned(),
        after: NEW.to_owned(),
    }
}

#[test]
fn an_offer_writes_one_line_and_leaves_the_document_as_it_was() {
    let dir = tempfile::tempdir().unwrap();
    let path = lay(dir.path(), "lab/notes.md", NOTES.as_bytes());
    let written = Written::default();

    let answered = tool(dir.path(), 1, &written)
        .invoke(&offering("lab/notes.md", OLD, NEW))
        .map(|outcome| outcome.result.as_map().get("proposal").cloned())
        .map_err(|err| *err.code());

    let offered = notes_offered();
    assert_eq!(
        written.lines(),
        vec![line(
            1,
            EventKind::ProposalOffered,
            Payload::of(&offered).unwrap()
        )]
    );
    let id = Offer::of(run(1), &offered).unwrap().id();
    assert_eq!(answered, Ok(Some(json!(id.to_string()))));
    assert_eq!(std::fs::read(&path).unwrap(), NOTES.as_bytes());
}

#[test]
fn a_run_withdraws_only_an_open_card_it_offered() {
    let dir = tempfile::tempdir().unwrap();
    lay(dir.path(), "lab/notes.md", NOTES.as_bytes());
    let written = Written::default();
    let offering_run = tool(dir.path(), 1, &written);
    let other_run = tool(dir.path(), 2, &written);
    let id = Offer::of(run(1), &notes_offered()).unwrap().id();
    let code = |answer: Result<kernel::ToolOutcome, AxError>| {
        answer.map(|_| ()).map_err(|err| *err.code())
    };

    let offered = code(offering_run.invoke(&offering("lab/notes.md", OLD, NEW)));
    let by_another = code(other_run.invoke(&withdrawing(id)));
    let by_its_run = code(offering_run.invoke(&withdrawing(id)));
    let twice = code(offering_run.invoke(&withdrawing(id)));
    let offered_again = code(offering_run.invoke(&offering("lab/notes.md", OLD, NEW)));

    assert_eq!(
        [offered, by_another, by_its_run, twice, offered_again],
        [
            Ok(()),
            Err(AxCode::InvalidArgs),
            Ok(()),
            Err(AxCode::InvalidArgs),
            Err(AxCode::InvalidArgs),
        ]
    );
    assert_eq!(
        written.lines(),
        vec![
            line(
                1,
                EventKind::ProposalOffered,
                Payload::of(&notes_offered()).unwrap()
            ),
            line(
                1,
                EventKind::ProposalWithdrawn,
                Payload::of(&ProposalWithdrawn { proposal: id }).unwrap()
            ),
        ]
    );
}

/// The interval a quote names is the interval the decision cuts, in a
/// version whose characters are two bytes or four.
#[test]
fn a_quote_on_a_utf16_document_names_the_stretch_that_lands() {
    let utf16 =
        |text: &str| -> Vec<u8> { text.encode_utf16().flat_map(u16::to_le_bytes).collect() };
    let before = utf16("\u{feff}Der Ofen glüht 🔥. Er kühlt über Nacht.\n");
    let dir = tempfile::tempdir().unwrap();
    lay(dir.path(), "lab/notes.txt", &before);
    let written = Written::default();

    let answered = tool(dir.path(), 1, &written)
        .invoke(&offering(
            "lab/notes.txt",
            "Er kühlt über Nacht.",
            "Er kühlt bis zum Morgen.",
        ))
        .map(|_| ())
        .map_err(|err| *err.code());

    let landed: Vec<Option<Vec<u8>>> = written
        .lines()
        .iter()
        .map(|line| {
            let offer = Offer::of(run(1), &line.data.read::<ProposalOffered>().unwrap()).unwrap();
            let verdicts: Vec<SliceVerdict> = (0_u32..)
                .zip(offer.review().slices())
                .filter(|(_, slice)| slice.kind != SliceKind::Same)
                .map(|(slice, _)| SliceVerdict {
                    slice,
                    verdict: Verdict::Accept,
                })
                .collect();
            decide(&before, &[(&offer, &verdicts)])
                .unwrap()
                .map(|applied| applied.bytes().to_vec())
        })
        .collect();
    assert_eq!(answered, Ok(()));
    assert_eq!(
        landed,
        vec![Some(utf16(
            "\u{feff}Der Ofen glüht 🔥. Er kühlt bis zum Morgen.\n"
        ))]
    );
}
