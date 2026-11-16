// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Where a page's back key at the top left leads. A building is entered
// from the city's overview, so its way back is the city. Pages reached
// from the settings tree no longer draw a back key: the edge settings
// key reopens the panel, and a click on either blank side of the frame
// returns to the conversation. The key is one target, not a step back
// through the browser's history, because a page opened from a bookmark
// or a reload has no history entry to step back to.

import type { View } from "../../core/route";

// `null` where the page draws no back key: the conversation, the
// settings panel, every page the settings tree opens, and the gallery.
export function backOf(page: View): View | null {
  switch (page.kind) {
    case "building":
    case "raise":
      return { kind: "city" };
    case "city":
    case "mcp":
    case "record":
    case "cost":
    case "registry":
    case "welcome":
    case "monitor":
    case "run":
    case "talk":
    case "setup":
    case "gallery":
      return null;
  }
}
