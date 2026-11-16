// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Attachment } from "svelte/attachments";
import { createAttachmentKey } from "svelte/attachments";

export const HOLD = createAttachmentKey();

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
  readonly [hold: symbol]: Attachment<HTMLElement>;
}

export interface FrameWire {
  readonly onfocusout: (event: Left) => void;
  readonly [hold: symbol]: Attachment<HTMLElement>;
}
