// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the coin key's look is handed: the face that is up, the key's
// name and, for the faint face, why it does nothing, all in the person's
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
  // Which face is turned up; `idle` is the send face, faint.
  readonly face: Face;
  // Why the faint face does nothing, as the hint the look shows beside
  // it; nothing on the two faces that act.
  readonly why: string | undefined;
  // Spread on the key's button: its type, name, whether it can be
  // pressed, and what a press does.
  readonly wire: CoinWire;
}

export type CoinWire = Pick<HTMLButtonAttributes, "type" | "aria-label" | "aria-disabled" | "onclick">;

// The send face submits the form the key stands in, so Enter in the box
// and a press here are one path: `pressCoin` answers `words` and the
// click lets the submit through. The stop face is a plain button that
// cancels; the faint face swallows the press.
export function lookOf(face: Face, sending: Sending, lang: Lang, onStop: () => void): CoinLook {
  return {
    face,
    why: face === "idle" ? say(lang, "talk_enter_hint") : undefined,
    wire: {
      type: face === "stop" ? "button" : "submit",
      "aria-label": face === "stop" ? STOP : say(lang, SPELLING[sending]),
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
