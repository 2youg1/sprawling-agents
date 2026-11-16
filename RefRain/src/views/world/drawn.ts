// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How a world-layer seat holds the elements its look drew. A look holds
// no element reference of its own (client D95): the seat hands it a
// Svelte attachment inside a wire bag, and spreading the bag on the
// element carries the attachment along, which tells the seat which
// element to move the focus to or scroll.

import { createAttachmentKey } from "svelte/attachments";
import type { Attachment } from "svelte/attachments";

// One key for every attachment a world-layer seat hands its look inside
// a wire bag. Svelte keeps one attachment per element under each symbol
// key, so one key serves every element a look draws.
export const HOLD = createAttachmentKey();

// What a key handler in a wire bag reads from the event, and nothing
// more, so a wiring test can press a key without a DOM.
export type KeyPress = Pick<KeyboardEvent, "key" | "preventDefault">;

// The elements a seat's look drew, by key, and the attachment that
// records each one. `hold` answers the same attachment for the same key
// on every call, so a redraw of the look does not let go of an element
// and take it again. Plain records rather than reactive state: no draw
// reads them, and a reactive write from an attachment the derived look
// handed out would be a write during a derivation.
export interface Drawn {
  readonly get: (key: string) => HTMLElement | undefined;
  readonly hold: (key: string) => Attachment<HTMLElement>;
}

export function drawnElements(): Drawn {
  const drawn: Record<string, HTMLElement | undefined> = {};
  const holds: Record<string, Attachment<HTMLElement> | undefined> = {};
  return {
    get: (key) => drawn[key],
    hold: (key) => {
      const kept = holds[key];
      if (kept !== undefined) return kept;
      const made: Attachment<HTMLElement> = (node) => {
        drawn[key] = node;
        return () => {
          if (drawn[key] === node) drawn[key] = undefined;
        };
      };
      holds[key] = made;
      return made;
    },
  };
}
