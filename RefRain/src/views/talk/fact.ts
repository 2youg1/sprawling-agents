// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export const HOLD = Symbol();
export type Reference = (node: HTMLElement) => void;

export type KeyPress = Pick<KeyboardEvent, "key" | "preventDefault" | "stopPropagation">;

export interface Left {
  readonly relatedTarget: EventTarget | null;
}

export interface TriggerWire {
  readonly "aria-label": string;
  readonly "aria-haspopup": "dialog";
  readonly "aria-expanded": boolean;
  readonly "aria-controls"?: string | undefined;
  readonly onclick: () => void;
  readonly [hold: symbol]: Reference;
}

export interface FrameWire {
  readonly onfocusout: (event: Left) => void;
  readonly [hold: symbol]: Reference;
}
