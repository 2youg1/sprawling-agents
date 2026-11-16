// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { GlyphName } from "./glyph";

export interface IconButtonProps {
  readonly glyph: GlyphName;

  readonly label: string;

  readonly why?: string;
  readonly onPress?: () => void;
}

export interface IconButtonWire {
  readonly type: "button";
  readonly "aria-label": string;
  readonly "aria-disabled": boolean;
  readonly "aria-describedby": string | undefined;
  readonly onclick: () => void;
}

export interface IconButtonLook {
  readonly glyph: GlyphName;

  readonly hint: string;

  readonly wire: (hint: string) => IconButtonWire;
}

export function lookOf(props: IconButtonProps): IconButtonLook {
  const refused = props.why !== undefined;
  return {
    glyph: props.glyph,
    hint: props.why ?? props.label,
    wire: (hint) => ({
      type: "button",
      "aria-label": props.label,
      "aria-disabled": refused,
      "aria-describedby": refused ? hint : undefined,
      onclick: () => {
        if (refused) return;
        props.onPress?.();
      },
    }),
  };
}
