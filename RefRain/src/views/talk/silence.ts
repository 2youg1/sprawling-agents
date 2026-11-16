// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Turn } from "../../wire";

const FINISHED: readonly string[] = ["end_turn", "tool_use"];

export function cutOff(stopped: string | null | undefined): string | null {
  return stopped === null || stopped === undefined || FINISHED.includes(stopped) ? null : stopped;
}

export type Phase = "live" | "frozen";

function wordless(turn: Turn): boolean {
  return (turn.said ?? "") === "";
}

function refused(turn: Turn): boolean {
  return turn.notes.some((note) => "refused" in note);
}

function over(turn: Turn, phase: Phase): boolean {
  return phase === "frozen" || (turn.stopped !== null && turn.stopped !== undefined);
}

export function silentTurn(turn: Turn, phase: Phase): boolean {
  return wordless(turn) && turn.calls.length === 0 && over(turn, phase) && !refused(turn);
}

export function silentRun(turns: readonly Turn[], phase: Phase): boolean {
  return phase === "frozen" && turns.every(wordless) && !turns.some(refused);
}
