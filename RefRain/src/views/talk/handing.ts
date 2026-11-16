// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

export type Conversing =
  | { readonly kind: "idle" }
  | { readonly kind: "waiting"; readonly handing: Handing }
  | { readonly kind: "answered"; readonly refusal: AxError };

export const NOT_CONVERSING: Conversing = { kind: "idle" };

export function claims(conversing: Conversing, refusal: AxError, belief: Belief): boolean {
  switch (conversing.kind) {
    case "idle":
      return false;
    case "answered":
      return conversing.refusal === refusal;
    case "waiting":
      return belief.refusal === refusal && settle(conversing.handing, belief, "").box !== null;
  }
}

export function newestSeq(belief: Belief): number {
  return Object.values(belief.runs).reduce((most, run) => Math.max(most, run.lastSeq), 0);
}
