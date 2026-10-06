// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Where a branch may cut the mother's conversation, what each cut takes
// with it, and how a stretch of conversation began. The picker a typed
// `/fork` opens (`forking.svelte`) asks this file; no control in the
// thread branches the conversation (docs/frontend-method.md section 7I).
//
// **A person's words fork at their turn's parent and come back to the
// box.** A message is re-sayable - the point of returning it is that the
// person may edit what was heard - so the cut sits before the turn it
// landed in and the text travels to the draft.
//
// **A reply or a call forks at its own line and leaves the box empty.**
// The line is the entry's own position on the ledger, which the wire
// already carries (`Turn.opened`, `Call.at`); the rebuild on the city's
// side settles on that line or the nearest safe point before it, so a
// cut a provider cannot read comes back whole rather than half.
//
// **A cut that lands inside a call still open walks back to the turn's
// parent and says so.** `tool_called` is written and `tool_result` is
// not: an assistant message whose tool uses nothing answers is a shape
// no provider accepts, so the exchange is dropped whole and the picker
// reports the walk-back instead of hiding it.

import type { Call, Origin, RunId, Turn } from "../../wire";
import { Seq } from "../../wire";

// What a person pointed at. A message is a person's own words - the
// opening task or a steer that arrived mid-turn - and carries the text
// that goes back to the box.
export type ForkEntry =
  | { readonly kind: "message"; readonly turn: Turn; readonly text: string }
  | { readonly kind: "turn"; readonly turn: Turn }
  | { readonly kind: "call"; readonly turn: Turn; readonly call: Call };

// Everything one pick decides: where the branch cuts, what the box gets
// back, and which turn the divider names. `mother` travels with the
// rest because the divider's link needs it a moment later and the four
// always travel together.
export interface ForkPlan {
  readonly origin: Origin;
  readonly draft: string | null;
  readonly turn: number;
  readonly mother: RunId;
  readonly walkedBack: boolean;
}

// How the stretch the person is looking at began. A session opened here
// with nothing behind it is one thing; a branch is the same act with an
// origin, and the divider names the turn it cut at and the run it cut
// from, because that is the whole difference to a reader wondering where
// this conversation came from.
export type Boundary =
  | { readonly kind: "opened"; readonly at: number | null }
  | {
      readonly kind: "forked";
      readonly at: number | null;
      readonly turn: number;
      readonly mother: RunId;
    };

// The three lists the picker offers, in the order Ctrl-O cycles them.
// `quiet` hides the turns that reached for tools, `user` keeps only the
// person's own words - the two narrowing questions a person asks of a
// long conversation.
export type ForkFilter = "all" | "quiet" | "user";

export function planFork(mother: RunId, entry: ForkEntry): ForkPlan {
  const turn = entry.turn;
  const before = Seq.make(turn.opened - 1);
  if (entry.kind === "message") {
    return {
      origin: { run: mother, at_seq: before },
      draft: entry.text,
      turn: turn.number,
      mother,
      walkedBack: false,
    };
  }
  if (entry.kind === "turn") {
    return {
      origin: { run: mother, at_seq: turn.opened },
      draft: null,
      turn: turn.number,
      mother,
      walkedBack: false,
    };
  }
  // A cut at or after a call nobody has answered is inside that call:
  // walk back to the turn's parent and let the footer say so.
  const open = turn.calls.some((each) => each.outcome === "waiting" && each.at <= entry.call.at);
  return {
    origin: { run: mother, at_seq: open ? before : entry.call.at },
    draft: null,
    turn: turn.number,
    mother,
    walkedBack: open,
  };
}

// A Latin stop ends a sentence only before a space or the end, so a
// version number or a file name keeps its dots; a CJK stop always does.
const SENTENCE = /^.*?(?:[.!?](?=\s|$)|[。！？])/u;

// The mother run as the divider names it: the first sentence of its task,
// because a run id is thirty-six characters nobody recognises and the
// task is what the person remembers asking. A blank task names nothing,
// and answers `null` so the divider says "the mother run" instead.
export function motherName(task: string): string | null {
  const line = task.trim().split("\n")[0] ?? "";
  const named = SENTENCE.exec(line)?.[0] ?? line;
  return named === "" ? null : named;
}
