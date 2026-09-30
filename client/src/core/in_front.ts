// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Belief, RunBelief } from "./belief";
import type { View } from "./route";

// The palette's reading, moved here unchanged: the newest run still
// going anywhere, whatever the page shows.
export function runInFront(belief: Belief, _view: View): RunBelief | undefined {
  return belief.live.at(-1);
}
