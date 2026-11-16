// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A step whose door is a page of its own - the MCP page - opens onto a
// link to it rather than onto a copy of it (`body.svelte`). This is what
// `door.look.svelte` is handed.

import { say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import { toFragment } from "../../core/route";

export interface DoorLook {
  readonly label: string;
  // Spread on the link.
  readonly wire: { readonly href: string };
}

export function mcpDoorOf(lang: Lang): DoorLook {
  return { label: say(lang, "guide_step_mcp_open"), wire: { href: toFragment({ kind: "mcp" }) } };
}
