// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Attachment } from "svelte/attachments";
import { createAttachmentKey } from "svelte/attachments";

export class TextBox {
  element = $state<HTMLTextAreaElement | undefined>(undefined);

  readonly key = createAttachmentKey();

  readonly hold: Attachment<HTMLTextAreaElement> = (node) => {
    this.element = node;
    return () => {
      this.element = undefined;
    };
  };

  constructor(words: () => string) {
    $effect(() => {
      const written = words();
      if (this.element !== undefined && this.element.value !== written)
        this.element.value = written;
    });
  }

  grow(): void {
    if (CSS.supports("field-sizing", "content")) return;
    if (this.element === undefined) return;
    this.element.style.height = "auto";
    this.element.style.height = `${String(this.element.scrollHeight)}px`;
  }
}
