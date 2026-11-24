// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { GlyphName } from "./glyph";

export type ButtonResult = "success" | "refused" | "unknown";

export type Tone = "primary" | "secondary" | "quiet" | "destructive";

export type ButtonState = "idle" | "loading" | "stopped";

export interface ButtonProps {
  readonly label: string;

  readonly name?: string;
  readonly tone?: Tone;
  readonly type?: "button" | "submit";

  readonly loading?: boolean;
  readonly glyph?: GlyphName | undefined;
  readonly result?: ButtonResult | undefined;

  readonly why?: string;
  readonly onPress?: () => void;
}

export interface ButtonWire {
  readonly type: "button" | "submit";
  readonly "aria-disabled": boolean;
  readonly "aria-busy": boolean;
  readonly "aria-describedby": string | undefined;
  readonly "aria-label": string | undefined;
  readonly onclick: () => void;
}

export interface ButtonLook {
  readonly label: string;
  readonly tone: Tone;
  readonly state: ButtonState;
  readonly glyph?: GlyphName | undefined;
  readonly result?: ButtonResult | undefined;

  readonly why: string | undefined;

  readonly wire: (hint: string | undefined) => ButtonWire;
}

export function stateOf(props: ButtonProps): ButtonState {
  if (props.why !== undefined) return "stopped";
  return props.loading === true ? "loading" : "idle";
}

export function lookOf(props: ButtonProps): ButtonLook {
  const state = stateOf(props);
  return {
    label: props.label,
    tone: props.tone ?? "secondary",
    state,
    glyph: props.glyph,
    result: props.result,
    why: props.why,
    wire: (hint) => ({
      type: props.type ?? "button",
      "aria-disabled": state !== "idle",
      "aria-busy": state === "loading",
      "aria-describedby": hint,
      "aria-label": props.name,
      onclick: () => {
        if (state !== "idle") return;
        props.onPress?.();
      },
    }),
  };
}
