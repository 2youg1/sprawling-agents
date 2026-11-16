// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { View } from "../../core/route";

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
