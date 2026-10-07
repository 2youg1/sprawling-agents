// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What one row of the mailbox's working section is handed: the room,
// its newest unfrozen run's phase as a mark and in words with how long
// the run has gone, its task, and the link to the room's conversation
// (client/Spec.lean §7, D88).

import type { RunBelief } from "../../core/belief";
import type { Lang } from "../../core/lang";
import { lasted } from "../../core/time";
import type { GlyphName, Weight } from "../parts/glyph";
import { phaseOf } from "../runs/lineage";
import { PHASE_MARK, phaseSaid } from "../runs/phase";
import type { EntryLink } from "./entries";

export interface WorkingRowLook {
  readonly room: string;
  readonly mark: { readonly glyph: GlyphName; readonly weight: Weight };
  // The phase in words, then how long the run has gone.
  readonly doing: string;
  readonly task: string | null;
  readonly link: EntryLink;
}

// `now` is the page's one ticking clock (client/Spec.lean §4-59), so the
// reading is recomputed from the run's own start.
export function rowOf(run: Pick<RunBelief, "doing" | "started" | "task">, room: string, lang: Lang, now: number, link: Omit<EntryLink, "data-entry">): WorkingRowLook {
  const gone = run.started === null ? "" : lasted(now - run.started);
  return {
    room,
    mark: PHASE_MARK[phaseOf(run.doing)],
    doing: `${phaseSaid(lang, run.doing, now)} · ${gone}`,
    task: run.task,
    link: { ...link, "data-entry": "" },
  };
}
