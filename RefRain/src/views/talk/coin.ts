// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Sending } from "../../core/doing";
import type { Lang } from "../../core/lang";
import { say } from "../../core/lang";
import { STOP } from "../../core/slash";
import { SPELLING } from "./composer";
import { pressCoin, type Face } from "./coin_face";

export interface CoinLook {
  readonly face: Face;
  readonly sending: Sending;
  readonly command: string | undefined;
  readonly wire: CoinWire;
}

export interface CoinWire {
  readonly type: "button" | "submit";
  readonly "aria-label": string;
  readonly "aria-disabled": "true" | undefined;
  readonly onclick: (event: MouseEvent) => void;
}

export function lookOf(face: Face, sending: Sending, lang: Lang, onStop: () => void): CoinLook {
  const command =
    face === "stop" ? STOP : face === "send" ? say(lang, SPELLING[sending]) : undefined;
  return {
    face,
    sending,
    command,
    wire: {
      type: face === "stop" ? "button" : "submit",
      "aria-label": command ?? say(lang, SPELLING[sending]),
      "aria-disabled": face === "idle" ? "true" : undefined,
      onclick: (event) => {
        switch (pressCoin(face)) {
          case "words":
            return;
          case "cancel":
            onStop();
            return;
          case "nothing":
            event.preventDefault();
        }
      },
    },
  };
}
