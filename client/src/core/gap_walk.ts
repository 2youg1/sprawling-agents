// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The walk over a gap, and the mark it walks from. A `lagged` frame
// names a range of ledger records this page never received, and a
// reconnect names the ledger head this page may have fallen behind; the
// range is asked for a page at a time and folded like any other record.
// That is a sequence of questions and answers rather than one frame in
// and one action out, which is why it is a conversation of its own and
// not a judgement in `link.ts`.
//
// Every record the page folds passes through `fold`, so the mark - the
// highest sequence folded, from the live stream or a gap - is always
// the point a reconnect resumes after.

import type { Asking } from "./asking";
import type { BeliefStore } from "./belief";
import type { Lang } from "./lang";
import { unreadableRecord } from "./link";
import { Seq } from "../wire";
import type { AskId, AskOutcome, EventRecord, HistoryRangeAnswer, Query } from "../wire";

// A range of ledger records this page was told it never received.
interface Gap {
  // The first sequence still to fetch.
  readonly at: Seq;
  // The last sequence the range reaches.
  readonly to: Seq;
  // What the records of the range do to the answers on the page once
  // folded: see `filled` and `resume`.
  readonly records: GapRecords;
}

// A range `lagged` named holds records the live stream skipped, which
// invalidate nothing; a range written while the socket was down holds
// the news the answers on the page missed, and each record invalidates
// what it touches.
type GapRecords = "folded" | "invalidating";

// One page of a gap. The server caps an answer at its own limit whatever
// this asks for, and a page small enough to fold in one frame keeps a
// long gap from holding up the records that are still arriving.
const GAP_PAGE = 200;

// How many gap pages a reconnect fetches before a snapshot is the
// cheaper way to the present: past this, asking every watched question
// again moves fewer bytes than walking the range (client/Spec.lean §4-40).
const RESUME_PAGES = 2;

export interface GapWalk {
  // Folds one record and moves the mark past it, answering the first
  // field this build could not read.
  readonly fold: (record: EventRecord) => string | null;
  // The live stream skipped a range; it is fetched after any range
  // already owed.
  readonly lagged: (from: Seq, to: Seq) => void;
  // A welcome arrived, naming its ledger and that ledger's head.
  readonly welcomed: (epoch: string | null, head: Seq | null) => void;
  // True when the answer was to the walk's own question and has been
  // taken; false when it belongs to a view.
  readonly answered: (askId: AskId, outcome: AskOutcome) => boolean;
}

// `ask` sends one question and answers the id it went out under, or null
// when the socket is not open.
export function createGapWalk(
  ask: (query: Query) => AskId | null,
  store: Pick<BeliefStore, "apply" | "refused" | "forget">,
  asking: Pick<Asking, "invalidate" | "reconnected" | "resumed">,
  lang: Lang,
): GapWalk {
  // The ranges this page has been told it never received, oldest first,
  // and the near end of the page in flight. Ranges queue rather than
  // replace: two losses are two ranges, and the answer's own cursor is
  // what advances one - so a range that arrives while another is being
  // filled waits its turn and nothing is dropped between them.
  const gaps: Gap[] = [];
  let fetching: Seq | null = null;
  // The id the page in flight went out under: its answer is recognised
  // by the id, never by its content.
  let gapAsk: AskId | null = null;
  // The highest ledger sequence this page has folded. Null until the
  // first record, and a page with no mark has nothing to resume.
  let mark: Seq | null = null;
  // Which ledger the mark belongs to, as the last welcome named it. A
  // welcome naming another one means the city was made again: the mark
  // and every fold point into a history that no longer exists.
  let epoch: string | null = null;

  function fold(record: EventRecord): string | null {
    if (mark === null || record.seq > mark) mark = record.seq;
    return store.apply(record);
  }

  // Asks for one page of the oldest range still owed, and only when none
  // is in flight: the next page starts at the cursor the last one returned.
  function askGap(): void {
    const front = gaps[0];
    if (front === undefined || fetching !== null) {
      return;
    }
    fetching = front.at;
    gapAsk = ask({ history_range: { from: front.at, to: front.to, limit: GAP_PAGE } });
  }

  // The records of one page of a gap, folded like any other record: what
  // the page lost is what happened, and the fold is what draws it. A
  // `folded` gap (the stream lagged while the socket stayed open)
  // invalidates nothing: its answers were already asked after those
  // records. An `invalidating` gap (written while the socket was down)
  // marks stale each answer a record touches, since no answer since
  // the disconnect has seen it.
  function filled(range: HistoryRangeAnswer): void {
    // One report per page, naming the first field this build could not
    // read: a page of two hundred records is one question's answer, and
    // a notice per record would bury the question in its answer.
    let bad: string | null = null;
    const records = gaps[0]?.records ?? "folded";
    for (const record of range.records) {
      const unreadable = fold(record);
      if (records === "invalidating") asking.invalidate(record);
      bad ??= unreadable;
    }
    if (bad !== null) store.refused(unreadableRecord(lang, bad));
    if (fetching === null || range.from !== fetching) {
      // An answer to a page this page no longer waits for. Its records
      // are folded above; the range it belonged to has moved on.
      return;
    }
    fetching = null;
    const front = gaps.shift();
    // `??` rather than a null test: an absent cursor and an explicit one
    // are the same fact on this wire, and the server leaves the field out
    // when the range is answered.
    const cursor = range.next ?? null;
    if (front !== undefined && cursor !== null) {
      // The server's cursor, not arithmetic here: it is the one place
      // that knows where the Ledger holds the next record of the range.
      gaps.unshift({ ...front, at: cursor });
    }
    askGap();
  }

  // Forgets what the page folded when the welcome names another ledger,
  // so the resume below finds no mark and rebuilds from a snapshot.
  function renewed(named: string | null): void {
    if (epoch !== null && named !== null && named !== epoch) {
      mark = null;
      gaps.splice(0, gaps.length);
      store.forget();
    }
    epoch = named ?? epoch;
  }

  // A welcome names the ledger head. A page that knows where it stopped
  // and is at most `RESUME_PAGES` pages behind fetches the records in
  // between and asks again only what the dead socket took with it;
  // any other page asks every watched question again.
  function resume(head: Seq | null): void {
    const owed = gaps.reduce((far, gap) => (gap.to > far ? gap.to : far), mark ?? head ?? Seq.make(0));
    if (mark === null || head === null || head < mark || head - owed > RESUME_PAGES * GAP_PAGE) {
      asking.reconnected();
      return;
    }
    // A lagged gap still pending was folded-only because the answers of
    // the open socket had seen it; those answers died with the socket,
    // so its records now mark stale what they touch like the new gap's.
    gaps.splice(0, gaps.length, ...gaps.map((gap): Gap => ({ ...gap, records: "invalidating" })));
    if (head > owed) gaps.push({ at: Seq.make(owed + 1), to: head, records: "invalidating" });
    asking.resumed();
  }

  return {
    fold,
    lagged(from, to) {
      gaps.push({ at: from, to, records: "folded" });
      askGap();
    },
    welcomed(named, head) {
      // A new connection cannot be holding an answer the old one was
      // asked for, so the walk starts its front range again rather than
      // waiting for a page that will never arrive.
      fetching = null;
      gapAsk = null;
      renewed(named);
      resume(head);
      askGap();
    },
    answered(askId, outcome) {
      if (askId !== gapAsk) return false;
      gapAsk = null;
      if ("answer" in outcome && "history_range" in outcome.answer) filled(outcome.answer.history_range);
      return true;
    },
  };
}
