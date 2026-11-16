// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createAttachmentKey } from "svelte/attachments";
import type { Attachment } from "svelte/attachments";

export const HOLD = createAttachmentKey();

export type KeyPress = Pick<KeyboardEvent, "key" | "preventDefault">;

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
