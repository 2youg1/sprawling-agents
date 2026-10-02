// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The box as a drop target: whether a drag is over it, which the box
// draws as the wash it takes, and the files the last drop could not
// keep, each said under the box. What a drop carries is `dropping.ts`'s
// decision; where its paths land in the words is the box's, handed in
// as `place`.

import type { Ui } from "../../ui";
import { dropped } from "./dropping";
import type { Kept } from "./dropping";

export class DropZone {
  over = $state(false);
  refused = $state<readonly Kept[]>([]);

  constructor(
    private readonly ui: Ui,
    private readonly place: (paths: readonly string[]) => void,
  ) {}

  readonly enter = (event: DragEvent): void => {
    event.preventDefault();
    this.over = true;
  };

  readonly leave = (event: DragEvent & { readonly currentTarget: EventTarget & HTMLElement }): void => {
    const next = event.relatedTarget;
    if (!(next instanceof Node && event.currentTarget.contains(next))) this.over = false;
  };

  readonly drop = (event: DragEvent): void => {
    this.over = false;
    const drop = dropped(event.dataTransfer, this.ui.origin, this.ui.pairing);
    if (drop.kind === "nothing") return;
    event.preventDefault();
    this.refused = [];
    if (drop.kind === "named") {
      this.place(drop.paths);
      return;
    }
    void drop.kept.then((kept) => {
      this.place(kept.flatMap((each) => (each.kind === "path" ? [each.path] : [])));
      this.refused = kept.filter((each) => each.kind === "refused");
    });
  };
}
