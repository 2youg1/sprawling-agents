// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Where a page's back key at the top left leads (A3, A6). A page is
// entered from the settings tree, so its way back is the settings panel,
// which reopens at the group drawn last; a building is entered from the
// city's overview, so its way back is the city. The key is one target,
// not a step back through the browser's history, because a page opened
// from a bookmark or a reload has no history entry to step back to.

import type { View } from "../../core/route";

export function backOf(page: View): View | null {
  switch (page.kind) {
    default:
      return null;
  }
}
