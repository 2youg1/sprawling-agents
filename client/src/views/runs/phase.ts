// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How a run's phase looks wherever a time bar draws it - the runs
// board and the city's building table - so the two bars are read with
// one legend.

import type { Key } from "../../core/lang";
import type { Status } from "../parts/glyph";
import type { Phase } from "./lineage";

export const PHASES: readonly Phase[] = ["model", "tool", "person", "idle", "done"];

export const PHASE_STATUS: Record<Phase, Status> = { model: "live", tool: "live", person: "waiting", idle: "idle", done: "done" };

export const PHASE_FILL: Record<Phase, string> = {
  model: "bg-accent",
  tool: "bg-accent-solid",
  person: "bg-alert",
  idle: "bg-edge-input",
  done: "bg-text-disabled",
};

export const PHASE_WORD: Record<Phase, Key> = {
  model: "run_doing_thinking",
  tool: "run_doing_calling",
  person: "run_doing_waiting",
  idle: "runs_phase_idle",
  done: "run_doing_frozen",
};
