// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import { fill, say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import type { Seq } from "../../wire";
import type { Arrival } from "../palette_door.svelte";


export type At = "head" | "box";


export interface BoxWords {
  readonly before: string;
  readonly after: string;
}

const ADDRESS_SLOT = "{address}";


export function boxWords(lang: Lang): BoxWords {
  const phrase = say(lang, "talk_placeholder_address");
  const slot = phrase.indexOf(ADDRESS_SLOT);
  if (slot < 0) return { before: phrase, after: "" };
  return { before: phrase.slice(0, slot), after: phrase.slice(slot + ADDRESS_SLOT.length) };
}

export interface AddressLineWire {
  readonly type: "button";
  readonly "aria-haspopup": "dialog";
  readonly "aria-label": string;
  readonly onclick: () => void;
  
  
  readonly onpointerdown: (event: Pressed) => void;
}


export interface Pressed {
  readonly pointerType: string;
  readonly currentTarget: { readonly focus: () => void };
}

export interface AddressLineLook {
  
  readonly head: string;
  
  
  readonly building: string;
  readonly room: string;
  
  
  readonly session: string | undefined;
  readonly wire: AddressLineWire;
}


export interface Lesson {
  readonly words: string;
  readonly chord: string;
}

export function addressLineOf(
  lang: Lang,
  address: string,
  session: Seq | null,
  open: (arrival: Arrival) => void,
): AddressLineLook {
  let pressed: Arrival = "keys";
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
      onclick: () => {
        const arrival = pressed;
        pressed = "keys";
        open(arrival);
      },
      onpointerdown: (event) => {
        pressed = event.pointerType === "touch" || event.pointerType === "pen" ? "touch" : "keys";
        event.currentTarget.focus();
      },
    },
  };
}
