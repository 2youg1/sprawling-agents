// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Option, Schema } from "effect";

import type { Tail } from "../../core/live_output";
import { Locator } from "../../wire";
import type { Call, TimeMs } from "../../wire";
import { lastedOf } from "../talk/timing";
import { commandOf, type CeilingNote, type Ending } from "../monitor/trace";

export interface Printed {
  readonly line: string;
  readonly out: string;
  readonly err: string;
  readonly cut: number;
  readonly ending: Ending | null;
  readonly took: number | null;
  readonly finished: TimeMs | null;
  readonly pinned: Locator | null;
  readonly live: boolean;
  readonly ceilings: readonly CeilingNote[];
}

export function printedOf(call: Call, tail: Tail): Printed {
  const finished = call.answered ?? null;
  const took = lastedOf(call);
  const pinned = Option.getOrNull(Schema.decodeUnknownOption(Locator)(call.output?.pinned));
  const live = call.outcome === "waiting";
  const entry = call.render === "terminal" ? commandOf(call) : null;
  if (entry?.kind === "command") {
    return {
      line: entry.text,
      out: live ? tail.out : entry.stdout,
      err: live ? tail.err : entry.stderr,
      cut: live ? tail.cut : entry.cut,
      ending: entry.ending,
      took,
      finished,
      pinned,
      live,
      ceilings: entry.ceilings,
    };
  }
  const answered = call.output?.head ?? "";
  return {
    line: call.subject ?? "",
    out: call.outcome === "failed" ? "" : answered,
    err: call.outcome === "failed" ? answered : "",
    cut: call.output?.cut ?? 0,
    ending: null,
    took,
    finished,
    pinned,
    live,
    ceilings: [],
  };
}
