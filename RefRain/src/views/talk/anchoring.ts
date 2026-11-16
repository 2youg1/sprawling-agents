// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export interface Foot {
  readonly scrollTop: number;
  readonly clientHeight: number;
  readonly scrollHeight: number;
}

export type Anchoring = "follow" | "hold";

export const FOOT_TOLERANCE = 40;

export function footOf(box: Element): Foot {
  return {
    scrollTop: box.scrollTop,
    clientHeight: box.clientHeight,
    scrollHeight: box.scrollHeight,
  };
}

export type Moment =
  { readonly kind: "opened" } | { readonly kind: "scrolled"; readonly foot: Foot };

export function anchorAt(moment: Moment): Anchoring {
  switch (moment.kind) {
    case "opened":
      return "follow";
    case "scrolled": {
      const { scrollHeight, scrollTop, clientHeight } = moment.foot;
      return scrollHeight - scrollTop - clientHeight < FOOT_TOLERANCE ? "follow" : "hold";
    }
  }
}
