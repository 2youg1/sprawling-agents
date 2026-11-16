// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { RunBelief } from "../../core/belief";
import type { View } from "../../core/route";
import type { Call, Origin, RoundsAnswer, RunId, SessionLine, Turn } from "../../wire";
import { Seq } from "../../wire";
import { noteAt } from "./note_line";

export type ForkEntry =
  | { readonly kind: "message"; readonly turn: Turn; readonly text: string }
  | { readonly kind: "turn"; readonly turn: Turn }
  | { readonly kind: "call"; readonly turn: Turn; readonly call: Call };

export interface ForkPlan {
  readonly origin: Origin;
  readonly draft: string | null;
  readonly turn: number;
  readonly mother: RunId;
  readonly walkedBack: boolean;
}

export type Boundary =
  | { readonly kind: "opened"; readonly at: number | null }
  | {
      readonly kind: "forked";
      readonly at: number | null;
      readonly turn: number | null;
      readonly mother: RunId;
      readonly origin: Origin;
    };

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
  const open = turn.calls.some((each) => each.outcome === "waiting" && each.at <= entry.call.at);
  return {
    origin: { run: mother, at_seq: open ? before : entry.call.at },
    draft: null,
    turn: turn.number,
    mother,
    walkedBack: open,
  };
}

const SENTENCE = /^.*?(?:[.!?](?=\s|$)|[。！？])/u;

export function motherName(task: string): string | null {
  const line = task.trim().split("\n")[0] ?? "";
  const named = SENTENCE.exec(line)?.[0] ?? line;
  return named === "" ? null : named;
}

export function originalOf(
  mother: RunId,
  held: Pick<RunBelief, "addr" | "lastSeq"> | undefined,
  lines: readonly SessionLine[],
): View {
  const room = held?.addr ?? null;
  const line = held === undefined ? undefined : lines.find((each) => each.began <= held.lastSeq);
  if (room === null || line === undefined) return { kind: "run", run: mother };
  return { kind: "talk", address: room, session: line.began };
}

export function roundsThrough(answer: RoundsAnswer, through: Seq | null): RoundsAnswer {
  if (through === null) return answer;
  return {
    ...answer,
    closing: null,
    turns: answer.turns
      .filter((turn) => turn.opened <= through)
      .map((turn) => ({
        ...turn,
        calls: turn.calls.filter((call) => call.at <= through),
        notes: turn.notes.filter((note) => noteAt(note) <= through),
      })),
  };
}
