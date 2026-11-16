// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import type { AxError } from "../../wire";

export type Delivery =
  | { readonly kind: "none" }
  | { readonly kind: "held"; readonly words: string; readonly refusal: AxError | null }
  | {
      readonly kind: "pending";
      readonly words: string;
      readonly refusal: AxError | null;
      readonly mark: number;
    }
  | { readonly kind: "accepted"; readonly words: string }
  | {
      readonly kind: "unknown";
      readonly words: string;
      readonly refusal: AxError | null;
      readonly mark: number;
    };

export const NONE: Delivery = { kind: "none" };


export interface Heard {
  readonly live: boolean;
  readonly newest: number;
  readonly refusal: AxError | null;
}


export function sent(words: string, heard: Heard): Delivery {
  return heard.live
    ? { kind: "pending", words, refusal: heard.refusal, mark: heard.newest }
    : { kind: "held", words, refusal: heard.refusal };
}


export function delivered(delivery: Delivery, heard: Heard): Delivery {
  switch (delivery.kind) {
    case "none":
    case "accepted":
      return delivery;
    case "held":
      if (heard.refusal !== delivery.refusal) return NONE;
      return heard.live
        ? { kind: "pending", words: delivery.words, refusal: delivery.refusal, mark: heard.newest }
        : delivery;
    case "pending":
    case "unknown":
      if (heard.newest > delivery.mark) return { kind: "accepted", words: delivery.words };
      if (heard.refusal !== delivery.refusal) return NONE;
      return heard.live || delivery.kind === "unknown"
        ? delivery
        : { ...delivery, kind: "unknown" };
  }
}
