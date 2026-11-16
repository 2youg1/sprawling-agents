// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// When the thread may say "the model said nothing". The one place that
// decides it, so the per-turn box and the whole-run card cannot disagree.

import type { Turn } from "../../wire";

// The provider's own words for a reply that ended the way replies end.
// Anything else - the ceiling, and whatever a provider adds next - is a
// reply that was cut off, and a reader is told so.
const FINISHED: readonly string[] = ["end_turn", "tool_use"];

// Why a reply was cut off, in the provider's word, or `null` when it ended
// the way replies end or the provider said nothing.
export function cutOff(stopped: string | null | undefined): string | null {
  return stopped === null || stopped === undefined || FINISHED.includes(stopped) ? null : stopped;
}

// Whether the run behind a thread has stopped for good.
export type Phase = "live" | "frozen";

function wordless(turn: Turn): boolean {
  return (turn.said ?? "") === "";
}

// A refused call is its own box with the reason; saying "nothing came
// back" under it would blame the output ceiling for a request that failed.
function refused(turn: Turn): boolean {
  return turn.notes.some((note) => "refused" in note);
}

// A turn is over once the model's answer is back or the run has stopped.
// Before that, an empty turn is one being waited on or streamed into, and
// the growing text lives outside the rounds.
function over(turn: Turn, phase: Phase): boolean {
  return phase === "frozen" || (turn.stopped !== null && turn.stopped !== undefined);
}

// One finished turn that said nothing and called nothing.
export function silentTurn(turn: Turn, phase: Phase): boolean {
  return wordless(turn) && turn.calls.length === 0 && over(turn, phase) && !refused(turn);
}

// A run that stopped without a word in any of its turns: one card with the
// reason and a way out, rather than one box per turn.
export function silentRun(turns: readonly Turn[], phase: Phase): boolean {
  return phase === "frozen" && turns.every(wordless) && !turns.some(refused);
}
