// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { get, readonly, writable } from "../../core/store";

export class TextBox {
  private readonly placed = writable<HTMLTextAreaElement | undefined>(undefined);
  readonly node = readonly(this.placed);

  get element(): HTMLTextAreaElement | undefined {
    return get(this.placed);
  }

  readonly hold = (node: HTMLTextAreaElement): (() => void) => {
    this.placed.set(node);
    this.sync(this.words());
    return () => {
      if (this.element === node) this.placed.set(undefined);
    };
  };

  constructor(private readonly words: () => string) {}

  sync(value: string): void {
    const node = this.element;
    if (node !== undefined && node.value !== value) node.value = value;
  }

  grow(): void {
    if (CSS.supports("field-sizing", "content")) return;
    if (this.element === undefined) return;
    this.element.style.height = "auto";
    this.element.style.height = `${String(this.element.scrollHeight)}px`;
  }
}
