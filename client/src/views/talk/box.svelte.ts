// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The composer's text box as an element: the attachment its wiring
// carries to the look, so the seat can reach the box it does not draw,
// the words written into it, and the growth an engine without
// `field-sizing: content` needs. What the words are is the seat's state.

import type { Attachment } from "svelte/attachments";
import { createAttachmentKey } from "svelte/attachments";

export class TextBox {
  element = $state<HTMLTextAreaElement | undefined>(undefined);

  // The key the attachment travels under in the box's wiring bag; one
  // per box, so a new bag does not attach the box a second time.
  readonly key = createAttachmentKey();

  readonly hold: Attachment<HTMLTextAreaElement> = (node) => {
    this.element = node;
    return () => {
      this.element = undefined;
    };
  };

  // The words reach the box here rather than through a binding, so the
  // look stays a plain element with a bag spread on it. The guard keeps
  // a write from touching a box that already holds the words, which
  // would end an input method's composition. Made while the seat is
  // initialised, so the effect lives as long as the seat.
  constructor(words: () => string) {
    $effect(() => {
      const written = words();
      if (this.element !== undefined && this.element.value !== written) this.element.value = written;
    });
  }

  // `field-sizing: content` grows the box before the frame is painted
  // where the engine has it; this is the path for the engines without it.
  grow(): void {
    if (CSS.supports("field-sizing", "content")) return;
    if (this.element === undefined) return;
    this.element.style.height = "auto";
    this.element.style.height = `${String(this.element.scrollHeight)}px`;
  }
}
