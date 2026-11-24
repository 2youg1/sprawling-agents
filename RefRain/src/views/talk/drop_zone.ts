// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { get, readonly, writable } from "../../core/store";

import type { Ui } from "../../ui";
import { dropped } from "./dropping";
import type { Refused } from "./dropping";

export class DropZone {
  private readonly held = writable<{ over: boolean; refused: readonly Refused[] }>({
    over: false,
    refused: [],
  });
  readonly state = readonly(this.held);

  get over(): boolean {
    return get(this.held).over;
  }
  get refused(): readonly Refused[] {
    return get(this.held).refused;
  }

  constructor(
    private readonly ui: Ui,
    private readonly place: (paths: readonly string[]) => void,
  ) {}

  readonly enter = (event: DragEvent): void => {
    event.preventDefault();
    this.held.update((state) => ({ ...state, over: true }));
  };

  readonly leave = (
    event: DragEvent & { readonly currentTarget: EventTarget & HTMLElement },
  ): void => {
    const next = event.relatedTarget;
    if (!(next instanceof Node && event.currentTarget.contains(next)))
      this.held.update((state) => ({ ...state, over: false }));
  };

  readonly drop = (event: DragEvent): void => {
    this.held.update((state) => ({ ...state, over: false }));
    const drop = dropped(event.dataTransfer, this.ui.origin, this.ui.credential());
    if (drop.kind === "nothing") return;
    event.preventDefault();
    this.held.update((state) => ({ ...state, refused: [] }));
    if (drop.kind === "named") {
      this.place(drop.paths);
      return;
    }
    void drop.kept.then((kept) => {
      this.place(kept.flatMap((each) => (each.kind === "path" ? [each.path] : [])));
      const refused = kept.filter((each): each is Refused => each.kind === "refused");
      this.held.update((state) => ({ ...state, refused }));
    });
  };
}
