// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Where a run's checkpoints stand among its turns. A checkpoint is the
// tree the city committed after a wave (`Note::checkpointed`), so it is
// the only commit a page can compare against for "what this run changed":
// the run page, the room's produced line and the inspector's diff all
// measure from here, and none of them counts an edit somebody made after
// the run.

import type { GitOid, RoundsAnswer, Seq, Turn } from "../wire";

// The tree the run last checkpointed, or nothing when it never did.
export function lastCheckpointIn(turns: readonly Turn[]): GitOid | null {
  for (let at = turns.length - 1; at >= 0; at -= 1) {
    for (const note of turns[at]?.notes ?? []) {
      if ("checkpointed" in note) return note.checkpointed.oid;
    }
  }
  return null;
}

// The two trees one call sits between: the newest checkpoint written
// before it, or the tree the run opened at when there is none, and the
// first checkpoint written after it. A checkpoint follows a wave, so the
// pair brackets every call of that wave and not this call alone.
export interface Bracket {
  readonly base: GitOid | null;
  readonly head: GitOid | null;
}

export function bracketOf(rounds: RoundsAnswer, at: Seq): Bracket {
  const marks = rounds.turns.flatMap((turn) => turn.notes.flatMap((note) => ("checkpointed" in note ? [note.checkpointed] : [])));
  const before = marks.filter((mark) => mark.at < at).sort((a, b) => b.at - a.at)[0];
  const after = marks.filter((mark) => mark.at > at).sort((a, b) => a.at - b.at)[0];
  return { base: before?.oid ?? rounds.opened_at ?? null, head: after?.oid ?? null };
}
