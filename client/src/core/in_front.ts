// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Belief, RunBelief } from "./belief";
import { newestWorking } from "./belief/live";
import type { View } from "./route";

// The run in front of the person: the one a stop, a steer and `/diff`
// reach from wherever they are pressed (client-SPEC 4-41, 12-16). A
// talk page answers with its room's newest working run, a run page
// with its own run while that run still works, and every other page
// with none: a city, a building or the monitor shows many runs, and a
// stop that chose one of them would choose for the person.
export function runInFront(belief: Belief, view: View): RunBelief | undefined {
  switch (view.kind) {
    case "talk":
      return newestWorking(belief, view.address);
    case "run":
      return belief.live.find((each) => each.run === view.run);
    case "city":
    case "building":
    case "setup":
    case "mcp":
    case "record":
    case "cost":
    case "registry":
    case "welcome":
    case "monitor":
    case "gallery":
      return undefined;
  }
}
