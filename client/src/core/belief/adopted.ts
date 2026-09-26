// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { PHASES, moves } from "../doing";
import type { Doing } from "../doing";
import type { RunSummary } from "../../wire";

import type { RunBelief } from "./shape";

// What an answer says a frozen run's ending was. The completion is the
// record's, and an answer carries none, so a run this page never
// streamed is frozen with nothing to cite.
function frozen(held: RunBelief | undefined): Doing {
  return held?.doing.kind === "frozen" ? held.doing : PHASES.run_frozen;
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
    return { ...held, addr: held.addr ?? summary.addr ?? null, started: at, local: false };
  }
  // The answer is the newer reading. Its `last_kind` states the phase
  // where the kind does; a kind that states none leaves what the page
  // already knew in place, and a run this page never saw keeps the one
  // thing it knows, which is that the run exists.
  const stated = moves(summary.last_kind) ? PHASES[summary.last_kind] : undefined;
  return {
    run: summary.run,
    addr: summary.addr ?? held?.addr ?? null,
    started: summary.started ?? held?.started ?? null,
    task: held?.task ?? null,
    lastSeq: summary.last_seq,
    doing: summary.frozen ? frozen(held) : (stated ?? held?.doing ?? { kind: "unknown" }),
    local: false,
    saying: held?.saying ?? "",
    thinking: held?.thinking ?? "",
  };
}
