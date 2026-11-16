// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the box a CodeMirror view is mounted into is given (client D95
// shape: a seat, a look, and this file). The view itself is built by
// `./editing.ts`, which is a lazy chunk; this file stays out of it, so a
// page can draw the box before the editor's chunk arrives.

import { createAttachmentKey } from "svelte/attachments";
import type { Attachment } from "svelte/attachments";

// The bag spread on the box: one attachment that hands the seat the
// element to mount the view into, and takes it back when the box goes.
export type EditorWire = Readonly<Record<symbol, Attachment<HTMLDivElement>>>;

export interface EditorLook {
  readonly wire: EditorWire;
}

// The wire for a seat that keeps the box in `keep`: called with the
// element once it is drawn and with `undefined` once it is removed.
export function editorWire(keep: (box: HTMLDivElement | undefined) => void): EditorWire {
  return {
    [createAttachmentKey()]: (box: HTMLDivElement) => {
      keep(box);
      return () => {
        keep(undefined);
      };
    },
  };
}
