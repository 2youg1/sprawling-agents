// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Where the words a person just sent stand, between the box and the run
// that answers them (refrain §3-14, "送达、断线与恢复"; client-SPEC 4-44).
//
// Five states, and two of them are not this file's: a **draft** is the
// words still in the box, and **running** is the run's own thread, which
// draws the words as its task the moment the city starts it. Between the
// two the page owes the person an answer within a tenth of a second, and
// that answer must not claim more than the page knows:
//
// - **held**: the link was down when the person pressed send, so the
//   words wait in this page's queue (`core/unsent.ts`) and go out when
//   the city is back;
// - **pending**: the frame went out and the city has written nothing
//   since;
// - **accepted**: the city wrote a record after the frame went out. The
//   wire carries no receipt naming the frame, so - as `handing.ts` reads
//   it - a newer record is taken as acceptance;
// - **unknown**: the link dropped while the frame was pending. The page
//   does not send it again: a dispatch the city did take, sent a second
//   time, is a second run. What the city did is learned from what it
//   tells the page after it reconnects, which moves this to accepted.
//
// A refusal newer than the send ends the echo: the refusal is drawn where
// the reply would have been (`talk.svelte`), and the words go back to the
// box.

import type { AxError } from "../../wire";

export type Delivery =
  | { readonly kind: "none" }
  | { readonly kind: "held"; readonly words: string; readonly refusal: AxError | null }
  | { readonly kind: "pending"; readonly words: string; readonly refusal: AxError | null; readonly mark: number }
  | { readonly kind: "accepted"; readonly words: string }
  | { readonly kind: "unknown"; readonly words: string; readonly refusal: AxError | null; readonly mark: number };

export const NONE: Delivery = { kind: "none" };

// What the page reads off the city at one moment: whether the link is
// live, the newest record it has folded, and the newest refusal.
export interface Heard {
  readonly live: boolean;
  readonly newest: number;
  readonly refusal: AxError | null;
}

// The words left the box at `heard`.
export function sent(words: string, heard: Heard): Delivery {
  return heard.live
    ? { kind: "pending", words, refusal: heard.refusal, mark: heard.newest }
    : { kind: "held", words, refusal: heard.refusal };
}

// Where the words stand now.
export function delivered(delivery: Delivery, heard: Heard): Delivery {
  switch (delivery.kind) {
    case "none":
    case "accepted":
      return delivery;
    case "held":
      if (heard.refusal !== delivery.refusal) return NONE;
      return heard.live ? { kind: "pending", words: delivery.words, refusal: delivery.refusal, mark: heard.newest } : delivery;
    case "pending":
    case "unknown":
      if (heard.newest > delivery.mark) return { kind: "accepted", words: delivery.words };
      if (heard.refusal !== delivery.refusal) return NONE;
      return heard.live || delivery.kind === "unknown" ? delivery : { ...delivery, kind: "unknown" };
  }
}
