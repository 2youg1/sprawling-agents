// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the conversation's address line is given (client/Spec.lean
// 4-10a): the one place the room's address and the session's number are
// drawn in zen and blend, and the control that opens the command palette.
//
// **The room is cut last.** A long address loses its building before
// its room: two rooms of one building differ only in their last segment,
// so the look truncates `building` first and `room` only on a column too
// narrow for the room alone (#65's ruling, "the end of a long address
// stays visible"). The whole address is in the button's name.

import { fill, say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import type { Seq } from "../../wire";

// Whether a conversation column draws the address line at its head: zen
// and blend do, and the panorama workbench, which writes the address under
// the chosen session's title (docs/frontend-method.md §7D), does not.
export type Head = "address" | "none";

export interface AddressLineWire {
  readonly type: "button";
  readonly "aria-haspopup": "dialog";
  readonly "aria-label": string;
  readonly onclick: () => void;
}

export interface AddressLineLook {
  // The name of the head landmark the line stands in.
  readonly head: string;
  // Everything up to and including the last `/`; empty for an address
  // of one segment.
  readonly building: string;
  readonly room: string;
  // The session's number as the address bar spells it, `#<began>`; absent
  // while the room has no session.
  readonly session: string | undefined;
  readonly wire: AddressLineWire;
}

export function addressLineOf(lang: Lang, address: string, session: Seq | null, open: () => void): AddressLineLook {
  const cut = address.lastIndexOf("/") + 1;
  const number = session === null ? undefined : `#${String(session)}`;
  return {
    head: say(lang, "region_address_line"),
    building: address.slice(0, cut),
    room: address.slice(cut),
    session: number,
    wire: {
      type: "button",
      "aria-haspopup": "dialog",
      "aria-label":
        number === undefined
          ? fill(say(lang, "talk_address_line_new"), { address })
          : fill(say(lang, "talk_address_line"), { address, session: number }),
      onclick: open,
    },
  };
}
