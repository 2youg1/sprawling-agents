// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A way from a settings group to another page, drawn as a control. It
// is a link rather than a button because it moves the address bar like
// every other way off the page, so Back returns to the group. The look
// is `door.look.svelte`.

import type { View } from "../../core/route";
import { toFragment } from "../../core/route";

export interface DoorWire {
  readonly href: string;
}

export interface DoorLook {
  readonly wire: DoorWire;
  // Already in the person's language.
  readonly label: string;
}

export function doorOf(to: View, label: string): DoorLook {
  return { wire: { href: toFragment(to) }, label };
}
