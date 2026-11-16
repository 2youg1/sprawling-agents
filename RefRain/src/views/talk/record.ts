// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import type { HTMLButtonAttributes } from "svelte/elements";

import type { Lang } from "../../core/lang";
import { say } from "../../core/lang";


export type Hearing = "taking" | "hearing" | "ready";

export interface RecordLook {
  readonly hearing: Hearing;
  readonly label: string;
  
  readonly refused: string | undefined;
  readonly wire: RecordWire;
}

export interface RecordWire {
  readonly type: "button";
  readonly "aria-label": string;
  
  
  readonly "aria-disabled": boolean;
  readonly onclick: HTMLButtonAttributes["onclick"];
}

export interface Heard {
  readonly taking: boolean;
  readonly hearing: boolean;
  readonly refused: boolean;
}


export function hearingOf(heard: Heard): Hearing {
  if (heard.hearing) return "hearing";
  return heard.taking ? "taking" : "ready";
}

export function lookOf(heard: Heard, lang: Lang, press: () => void): RecordLook {
  const hearing = hearingOf(heard);
  const label = say(lang, LABEL[hearing]);
  return {
    hearing,
    label,
    refused: heard.refused ? say(lang, "link_refused") : undefined,
    wire: {
      type: "button",
      
      
      "aria-label": label,
      "aria-disabled": hearing === "hearing",
      
      
      onclick: press,
    },
  };
}

const LABEL = {
  taking: "talk_recording",
  hearing: "talk_hearing",
  ready: "talk_record",
} as const;
