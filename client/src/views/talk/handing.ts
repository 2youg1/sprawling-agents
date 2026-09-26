// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What happens to the words a box sent before the city has answered.
// The box empties the moment the socket takes the frame, so a send is
// seen to land at once; the words are kept here until the city answers,
// and a refusal puts them back.
//
// **The wire does not say which command a refusal answers** (no idem
// rides on it), so the answer is read off the belief: a refusal newer
// than the send is taken as refusing it, and any record the city wrote
// after the send is taken as accepting it. When both land in one paint
// the acceptance wins, because words given back after an accepted
// dispatch are sent twice by the person who meets them, while words
// lost after a refused one are only what the box did before.

import type { Belief } from "../../core/belief/shape";
import type { AxError } from "../../wire";

export type Handing =
  | { readonly kind: "idle" }
  | {
      readonly kind: "handed";
      readonly words: string;
      readonly refusal: AxError | null;
      readonly mark: number;
    };

export const IDLE: Handing = { kind: "idle" };

export function hand(words: string, belief: Belief): Handing {
  return { kind: "handed", words, refusal: belief.refusal, mark: newestSeq(belief) };
}

// The next handing, and the words that go back into the box when the
// city refused them.
export function settle(
  handing: Handing,
  belief: Belief,
  typed: string,
): { readonly handing: Handing; readonly box: string | null } {
  if (handing.kind === "idle") return { handing, box: null };
  if (newestSeq(belief) > handing.mark) return { handing: IDLE, box: null };
  if (belief.refusal === null || belief.refusal === handing.refusal) return { handing, box: null };
  return { handing: IDLE, box: typed === "" ? handing.words : `${handing.words}\n${typed}` };
}

function newestSeq(belief: Belief): number {
  return Object.values(belief.runs).reduce((most, run) => Math.max(most, run.lastSeq), 0);
}
