// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { View } from "../../core/route";

export function homeSidesOf(view: View): boolean {
  switch (view.kind) {
    case "talk":
    case "setup":
    case "welcome":
    case "gallery":
      return false;
    case "city":
    case "building":
    case "raise":
    case "run":
    case "mcp":
    case "record":
    case "cost":
    case "registry":
    case "monitor":
      return true;
  }
}
