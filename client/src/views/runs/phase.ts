// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How a run's phase looks wherever a time bar draws it - the runs
// board and the city's building table - so the two bars are read with
// one legend.

import type { Key } from "../../core/lang";
import type { GlyphName, Weight } from "../parts/glyph";
import type { Phase } from "./lineage";

export const PHASES: readonly Phase[] = ["model", "tool", "person", "idle", "done", "stopped", "capped"];

// The mark a run's row carries: each ending its own drawing, because a
// forced-colours mode repaints the ink and leaves only the shape.
export const PHASE_MARK: Record<Phase, { readonly glyph: GlyphName; readonly weight: Weight }> = {
  model: { glyph: "pulse", weight: "live" },
  tool: { glyph: "tool", weight: "live" },
  person: { glyph: "hand", weight: "alert" },
  idle: { glyph: "ring", weight: "quiet" },
  done: { glyph: "check", weight: "quiet" },
  stopped: { glyph: "stopped", weight: "quiet" },
  capped: { glyph: "capped", weight: "alert" },
};

export const PHASE_FILL: Record<Phase, string> = {
  model: "bg-accent",
  tool: "bg-accent-solid",
  person: "bg-alert",
  idle: "bg-edge-input",
  done: "bg-text-disabled",
  stopped: "bg-text-disabled",
  capped: "bg-alert",
};

export const PHASE_WORD: Record<Phase, Key> = {
  model: "run_doing_thinking",
  tool: "run_doing_calling",
  person: "run_doing_waiting",
  idle: "runs_phase_idle",
  done: "run_doing_frozen",
  stopped: "runs_phase_stopped",
  capped: "runs_phase_capped",
};
