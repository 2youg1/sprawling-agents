// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The wiring every fact of the composer's settings row that opens a menu
// shares - the model entry and the permissions entry - and that
// `fact.look.svelte` spreads on its button. Nothing here touches the
// DOM, so `bun test` presses a key without a compiled component.

import type { Attachment } from "svelte/attachments";
import { createAttachmentKey } from "svelte/attachments";

// One key for every element a seat holds. Svelte keeps one attachment
// per element under each symbol key, and each bag is spread on its own
// element, so one key is enough for every bag of the settings row.
export const HOLD = createAttachmentKey();

// What a key handler reads from the event, and nothing more, so a
// wiring test presses a key without a DOM.
export type KeyPress = Pick<KeyboardEvent, "key" | "preventDefault" | "stopPropagation">;

// Where focus went when it left an element, read the same way.
export interface Left {
  readonly relatedTarget: EventTarget | null;
}

// The bag spread on a fact that opens a menu. The accessible name says
// which fact it is before its value, because the drawn face shows the
// value alone.
export interface TriggerWire {
  readonly "aria-label": string;
  readonly "aria-haspopup": "dialog";
  readonly "aria-expanded": boolean;
  readonly "aria-controls"?: string | undefined;
  readonly onclick: () => void;
  readonly [hold: symbol]: Attachment<HTMLElement>;
}

// The bag spread on the element holding a trigger and its menu: focus
// leaving it closes the menu without taking focus back.
export interface FrameWire {
  readonly onfocusout: (event: Left) => void;
  readonly [hold: symbol]: Attachment<HTMLElement>;
}
