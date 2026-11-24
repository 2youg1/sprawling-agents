// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { PHASES, moves } from "../doing";
import type { Doing } from "../doing";
import type { RunSummary } from "../../wire";
import type { RunBelief } from "./shape";

function frozen(summary: RunSummary, held: RunBelief | undefined): Doing {
  const completion = summary.completion ?? null;
  if (completion !== null) return { kind: "frozen", completion };
  return held?.doing.kind === "frozen" ? held.doing : PHASES.run_frozen;
}

function going(summary: RunSummary, held: RunBelief | undefined): Doing {
  const wait = summary.waiting ?? null;
  if (wait !== null) return { kind: "awaiting_reply", wait };
  if (moves(summary.last_kind)) return PHASES[summary.last_kind];
  const known = held?.doing ?? { kind: "unknown" };
  return known.kind === "awaiting_reply" ? { kind: "unknown" } : known;
}

export function adopted(summary: RunSummary, held: RunBelief | undefined): RunBelief {
  if (held !== undefined && held.lastSeq >= summary.last_seq) {
    const at = held.started ?? summary.started ?? null;
    const pr = held.pr ?? summary.pr ?? null;
    return {
      ...held,
      addr: held.addr ?? summary.addr ?? null,
      started: at,
      task: held.task ?? summary.task ?? null,
      goal: held.goal ?? summary.goal ?? null,
      sessionBegan: held.sessionBegan ?? summary.session_began ?? undefined,
      pr,
      local: false,
    };
  }

  return {
    run: summary.run,
    addr: summary.addr ?? held?.addr ?? null,
    started: summary.started ?? held?.started ?? null,
    task: summary.task ?? held?.task ?? null,
    goal: summary.goal ?? held?.goal ?? null,
    sessionBegan: summary.session_began ?? held?.sessionBegan ?? undefined,
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
