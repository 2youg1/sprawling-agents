// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the coin key's look is handed: the face that is up, the key's
// name and the command a press would send, all in the person's
// language, and the key's wiring as one bag the look spreads on its
// button. Which face is up and what a press of it sends are
// `./coin_face.ts`'s decisions; this file only builds the value.

import type { HTMLButtonAttributes } from "svelte/elements";

import type { Sending } from "../../core/doing";
import type { Lang } from "../../core/lang";
import { say } from "../../core/lang";
import { STOP } from "../../core/slash";
import { SPELLING } from "./composer";
import { pressCoin, type Face } from "./coin_face";

export interface CoinLook {
  // Which face is turned up; `idle` means the key is not drawn.
  readonly face: Face;
  // Whether a send is a fresh dispatch or a steer into a live run.
  readonly sending: Sending;
  // The command a press would run, shown as the tip on every acting face
  // so the coin and the ring band stay distinguishable by what they say.
  readonly command: string | undefined;
  // Spread on the key's button: its type, name, whether it can be
  // pressed, and what a press does.
  readonly wire: CoinWire;
}

export type CoinWire = Pick<
  HTMLButtonAttributes,
  "type" | "aria-label" | "aria-disabled" | "onclick"
>;

// The send face submits the form the key stands in, so Enter in the box
// and a press here are one path: `pressCoin` answers `words` and the
// click lets the submit through. The stop face is a plain button that
// cancels; the idle face is not drawn.
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
