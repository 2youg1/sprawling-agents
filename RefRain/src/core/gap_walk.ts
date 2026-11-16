// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Asking } from "./asking";
import type { BeliefStore } from "./belief";
import type { Lang } from "./lang";
import { unreadableRecord } from "./link";
import { Seq } from "../wire";
import type { AskId, AskOutcome, EventRecord, HistoryRangeAnswer, Query } from "../wire";

interface Gap {
  readonly at: Seq;

  readonly to: Seq;

  readonly records: GapRecords;
}

type GapRecords = "folded" | "invalidating";

const GAP_PAGE = 200;

const RESUME_PAGES = 2;

export interface GapWalk {
  readonly fold: (record: EventRecord) => string | null;

  readonly lagged: (from: Seq, to: Seq) => void;

  readonly welcomed: (epoch: string | null, head: Seq | null) => void;

  readonly answered: (askId: AskId, outcome: AskOutcome) => boolean;
}

export function createGapWalk(
  ask: (query: Query) => AskId | null,
  store: Pick<BeliefStore, "apply" | "refused" | "forget">,
  asking: Pick<Asking, "invalidate" | "reconnected" | "resumed">,
  lang: Lang,
): GapWalk {
  const gaps: Gap[] = [];
  let fetching: Seq | null = null;

  let gapAsk: AskId | null = null;

  let mark: Seq | null = null;

  let epoch: string | null = null;

  function fold(record: EventRecord): string | null {
    if (mark === null || record.seq > mark) mark = record.seq;
    return store.apply(record);
  }

  function askGap(): void {
    const front = gaps[0];
    if (front === undefined || fetching !== null) {
      return;
    }
    fetching = front.at;
    gapAsk = ask({ history_range: { from: front.at, to: front.to, limit: GAP_PAGE } });
  }

  function filled(range: HistoryRangeAnswer): void {
    let bad: string | null = null;
    const records = gaps[0]?.records ?? "folded";
    for (const record of range.records) {
      const unreadable = fold(record);
      if (records === "invalidating") asking.invalidate(record);
      bad ??= unreadable;
    }
    if (bad !== null) store.refused(unreadableRecord(lang, bad));
    if (fetching === null || range.from !== fetching) {
      return;
    }
    fetching = null;
    const front = gaps.shift();

    const cursor = range.next ?? null;
    if (front !== undefined && cursor !== null) {
      gaps.unshift({ ...front, at: cursor });
    }
    askGap();
  }

  function renewed(named: string | null): void {
    if (epoch !== null && named !== null && named !== epoch) {
      mark = null;
      gaps.splice(0, gaps.length);
      store.forget();
    }
    epoch = named ?? epoch;
  }

  function resume(head: Seq | null): void {
    const owed = gaps.reduce(
      (far, gap) => (gap.to > far ? gap.to : far),
      mark ?? head ?? Seq.make(0),
    );
    if (mark === null || head === null || head < mark || head - owed > RESUME_PAGES * GAP_PAGE) {
      asking.reconnected();
      return;
    }

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
      fetching = null;
      gapAsk = null;
      renewed(named);
      resume(head);
      askGap();
    },
    answered(askId, outcome) {
      if (askId !== gapAsk) return false;
      gapAsk = null;
      if ("answer" in outcome && "history_range" in outcome.answer)
        filled(outcome.answer.history_range);
      return true;
    },
  };
}
