// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { GitOid, RoundsAnswer, Seq, Turn } from "../wire";

export function lastCheckpointIn(turns: readonly Turn[]): GitOid | null {
  for (let at = turns.length - 1; at >= 0; at -= 1) {
    for (const note of turns[at]?.notes ?? []) {
      if ("checkpointed" in note) return note.checkpointed.oid;
    }
  }
  return null;
}

export interface Bracket {
  readonly base: GitOid | null;
  readonly head: GitOid | null;
}

export function bracketOf(rounds: RoundsAnswer, at: Seq): Bracket {
  const marks = rounds.turns.flatMap((turn) =>
    turn.notes.flatMap((note) => ("checkpointed" in note ? [note.checkpointed] : [])),
  );
  const before = marks.filter((mark) => mark.at < at).sort((a, b) => b.at - a.at)[0];
  const after = marks.filter((mark) => mark.at > at).sort((a, b) => a.at - b.at)[0];
  return { base: before?.oid ?? rounds.opened_at ?? null, head: after?.oid ?? null };
}
