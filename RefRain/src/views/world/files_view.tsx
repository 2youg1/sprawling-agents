// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal } from "solid-js";

import { buildingOf } from "../../core/route";
import type { Address } from "../../wire";
import BuildingTree, { type Picked } from "../building/tree.solid";
import { openDocument, type DocumentItem } from "../inspect/open";

export interface FilesProps {
  readonly here: Address;
  readonly onFile?: ((file: DocumentItem) => void) | undefined;
}

export function Files(props: FilesProps) {
  const building = createMemo(() => buildingOf(props.here));
  const [picked, setPicked] = createSignal<Picked | null>(null);

  function pick(chosen: Picked): void {
    const root = building();
    const prefix = `${root}/`;
    setPicked(chosen);
    if (chosen.kind !== "file" || !chosen.at.startsWith(prefix)) return;
    const open = props.onFile ?? openDocument;
    open({ building: root, path: chosen.at.slice(prefix.length), version: null });
  }

  return (
    <div class="min-h-0 flex-1 overflow-y-auto">
      <BuildingTree root={building()} picked={picked()} onPick={pick} />
    </div>
  );
}
