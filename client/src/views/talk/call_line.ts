// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a tool call's line is given. The seat (`call_line.svelte`) reads
// the call, the clock, the right side and the line keys; the look draws
// four cells and the marks at their end from this alone.

import type { LineMove } from "../../core/lines";

// The time cell: what the Ledger measured once the call is in, the
// running tenths while it runs, nothing for a span nobody measured.
export type CallFigure =
  | { readonly kind: "landed"; readonly text: string }
  | { readonly kind: "running"; readonly text: string }
  | { readonly kind: "none" };

// How the call came out, in words: a failure or a wait on the person
// is an alert, any other result is quiet.
export interface CallVerdict {
  readonly tone: "alert" | "quiet";
  readonly text: string;
}

// The bag spread on the line's button. `data-call-line` is how the line
// keys find the lines beside this one, so it rides the bag rather than
// being the look's to remember.
export interface CallLineWire {
  readonly type: "button";
  readonly "data-call-line": "";
  readonly "aria-pressed": boolean;
  readonly onclick: () => void;
  readonly onkeydown: (event: KeyboardEvent) => void;
}

export interface CallLineLook {
  // Every word already in the person's language.
  readonly kind: string;
  readonly subject: string;
  // The call has not answered yet: its subject is in full ink and a
  // pulse stands before its time.
  readonly running: boolean;
  readonly time: CallFigure;
  readonly verdict: CallVerdict | null;
  // The keys drawn at the line's end, in the order refrain 3-12 names
  // them; shown on the line the right side holds, and on the focused one.
  readonly moves: readonly LineMove[];
  readonly opened: boolean;
  // A steer pressed now is heard after this call.
  readonly pinned: boolean;
  // When the call started or finished, as an instant.
  readonly hint: string;
  readonly wire: CallLineWire;
}
