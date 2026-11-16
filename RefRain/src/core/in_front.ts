// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Belief, RunBelief } from "./belief";
import { newestWorking } from "./belief/live";
import type { View } from "./route";


export function runInFront(belief: Belief, view: View): RunBelief | undefined {
  switch (view.kind) {
    case "talk":
      return newestWorking(belief, view.address);
    case "run":
      return belief.live.find((each) => each.run === view.run);
    case "city":
    case "building":
    case "raise":
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
