// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the inspector's terminal prints for one call (client-SPEC 7F).
//
// **The Ledger is the authority on a command's output; the live tail is a
// preview of it.** While the call is waiting, the terminal prints what
// the running command has written so far (`core/live_output.ts`); the
// moment the call has an outcome, it prints the result the Ledger holds
// and the tail is not read again, even if a stale one is still in hand.
// A command's line, its two streams and its exit code are read the way
// the monitor reads them (`monitor/trace.ts`), so the two terminals of
// this client cannot disagree about what a command printed.

import { Option, Schema } from "effect";

import type { Tail } from "../../core/live_output";
import { Locator } from "../../wire";
import type { Call, TimeMs } from "../../wire";
import { commandOf, type Ending } from "../monitor/trace";

export interface Printed {
  // The command line, or what any other call was called on.
  readonly line: string;
  readonly out: string;
  readonly err: string;
  // Lines the view withheld from `out` and `err` together.
  readonly cut: number;
  // How a command ended; `null` for a call that is not a command.
  readonly ending: Ending | null;
  // Milliseconds from call to answer, and the moment it was answered;
  // both `null` until it is.
  readonly took: number | null;
  readonly finished: TimeMs | null;
  // Where the whole original is kept, when the city kept it.
  readonly pinned: Locator | null;
  // Whether `out` and `err` are the live tail rather than the Ledger's.
  readonly live: boolean;
}

export function printedOf(call: Call, tail: Tail): Printed {
  const finished = call.answered ?? null;
  const took = finished === null ? null : finished - call.called;
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
  };
}
