// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { View } from "../../core/route";
import { toFragment } from "../../core/route";

export interface DoorWire {
  readonly href: string;
}

export interface DoorLook {
  readonly wire: DoorWire;
  readonly label: string;
}

export function doorOf(to: View, label: string): DoorLook {
  return { wire: { href: toFragment(to) }, label };
}
