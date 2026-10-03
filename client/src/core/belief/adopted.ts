// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { PHASES, moves } from "../doing";
import type { Doing } from "../doing";
import type { RunSummary } from "../../wire";
import type { RunBelief } from "./shape";

// What an answer says a frozen run's ending was: the completion it
// names, else the one this page folded, else an ending nobody stated.
function frozen(summary: RunSummary, held: RunBelief | undefined): Doing {
  const completion = summary.completion ?? null;
  if (completion !== null) return { kind: "frozen", completion };
  return held?.doing.kind === "frozen" ? held.doing : PHASES.run_frozen;
}

// What an answer says a run still going is doing. A wait it names comes
// first, because the city folded it from the payload the last kind
// alone cannot carry; then the phase the last kind states; then what
// the page knew - except a wait, which the newer answer has just said
// is over, and which it does not replace with anything it could state
// (client/Spec.lean D88).
function going(summary: RunSummary, held: RunBelief | undefined): Doing {
  const wait = summary.waiting ?? null;
  if (wait !== null) return { kind: "awaiting_reply", wait };
  if (moves(summary.last_kind)) return PHASES[summary.last_kind];
  const known = held?.doing ?? { kind: "unknown" };
  return known.kind === "awaiting_reply" ? { kind: "unknown" } : known;
}

// One `city_view` row read as a belief, keeping whatever the stream
// already knew that the row does not carry. The run page reads it too:
// a run reached by somebody's link was never streamed here, so the
// row is the only reading of that run this page has.
export function adopted(summary: RunSummary, held: RunBelief | undefined): RunBelief {
  // The page's own reading is the newer of the two, so it wins on
  // everything it knows; a field the stream never carried is still the
  // row's to state, and the answer settles nothing else but that.
  if (held !== undefined && held.lastSeq >= summary.last_seq) {
    const at = held.started ?? summary.started ?? null;
    const pr = held.pr ?? summary.pr ?? null;
    return {
      ...held,
      addr: held.addr ?? summary.addr ?? null,
      started: at,
      task: held.task ?? summary.task ?? null,
      goal: held.goal ?? summary.goal ?? null,
      pr,
      local: false,
    };
  }
  // The answer is the newer reading, and `going` says what it states
  // about a run that has not frozen; a run this page never saw keeps the
  // one thing it knows, which is that the run exists.
  return {
    run: summary.run,
    addr: summary.addr ?? held?.addr ?? null,
    started: summary.started ?? held?.started ?? null,
    task: summary.task ?? held?.task ?? null,
    goal: summary.goal ?? held?.goal ?? null,
    lastSeq: summary.last_seq,
    doing: summary.frozen ? frozen(summary, held) : going(summary, held),
    model: held?.model ?? null,
    pr: summary.pr ?? held?.pr ?? null,
    ask: summary.ask ?? null,
    local: false,
    saying: held?.saying ?? "",
    thinking: held?.thinking ?? "",
  };
}
