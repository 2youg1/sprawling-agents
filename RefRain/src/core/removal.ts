// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Address } from "../wire";
import { MAYOR, buildingOf } from "./route";

export type Removal = "offered" | "hall" | "busy";

export function removalOf(building: Address, living: number): Removal {
  if (building === buildingOf(MAYOR)) return "hall";
  return living > 0 ? "busy" : "offered";
}
