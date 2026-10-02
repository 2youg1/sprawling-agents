// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The tree a run last checkpointed. The run page, the results room and
// the city's "just finished" rows all measure "what the run changed" up
// to here, so none of them counts an edit made after the run by someone
// else.

import type { GitOid, Turn } from "../../wire";

// Searched backwards from the last turn; `null` when the run never
// checkpointed.
export function lastCheckpointIn(turns: readonly Turn[]): GitOid | null {
  for (let at = turns.length - 1; at >= 0; at -= 1) {
    for (const note of turns[at]?.notes ?? []) {
      if ("checkpointed" in note) return note.checkpointed.oid;
    }
  }
  return null;
}
