// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Snippet } from "../../core/snippet";
import type { ElementRef } from "../../core/element_ref";
import type { JSX } from "@solidjs/web";

export type TipSide = "above" | "right";

export type Placing = "by-engine" | "against-wrapper";

export type Showing = "wanted" | "dismissed" | "standing";

export type HolderWire = Readonly<Record<symbol, ElementRef<HTMLSpanElement>>>;
export type HintWire = JSX.HTMLAttributes<HTMLSpanElement>;

export interface TipWiring {
  readonly holder: HolderWire;
  readonly hint: HintWire;

  readonly id: string;

  readonly anchor: string;
  readonly text: string;
  readonly side: TipSide;
  readonly placing: Placing;
  readonly showing: Showing;
}

export interface TipLook extends TipWiring {
  readonly children: Snippet<[string]>;
}

export interface TipProps {
  readonly text: string;

  readonly children: Snippet<[string]>;
  readonly side?: TipSide;

  readonly exposable?: boolean;

  readonly id?: string;
}

export interface TipState {
  readonly uid: string;
  readonly dismissed: boolean;
  readonly reengage: ElementRef<HTMLSpanElement>;
}

const REENGAGE = Symbol();

export function dismisses(key: string): boolean {
  return key === "Escape";
}

export function lookOf(props: Omit<TipProps, "children">, state: TipState): TipWiring {
  const id = props.id ?? state.uid;
  return {
    holder: { [REENGAGE]: state.reengage },
    hint: {
      id,
      role: "tooltip",
      "data-exposable": props.exposable === true && !state.dismissed ? "" : undefined,
    },
    id,
    anchor: `--tip-${id}`,
    text: props.text,
    side: props.side ?? "above",
    placing: "by-engine",
    showing: state.dismissed ? "dismissed" : "wanted",
  };
}
