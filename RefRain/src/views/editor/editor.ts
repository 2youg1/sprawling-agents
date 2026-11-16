// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createAttachmentKey } from "svelte/attachments";
import type { Attachment } from "svelte/attachments";

export type EditorWire = Readonly<Record<symbol, Attachment<HTMLDivElement>>>;

export interface EditorLook {
  readonly wire: EditorWire;
}

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
