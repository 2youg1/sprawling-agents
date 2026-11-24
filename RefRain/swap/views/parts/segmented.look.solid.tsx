// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Show } from "solid-js";
import Glyph from "../../../src/views/parts/glyph.solid";
import type { CellLook, SegmentedLook } from "../../../src/views/parts/segmented";
import Tip from "../../../src/views/parts/control_tip";
import "./segmented.solid.css";

function Cell(props: { readonly cell: CellLook }) {
  const option = () => (
    <button type="button" class="option refrain-segmented-alt-option" {...props.cell.wire}>
      <span class="mark refrain-segmented-alt-mark" aria-hidden="true">
        <Glyph name="check" size="sm" />
      </span>
      <span class="label refrain-segmented-alt-label">{props.cell.label}</span>
    </button>
  );
  return (
    <Show when={props.cell.why} fallback={option()}>
      {(why) => (
        <Tip id={why().id} text={why().text}>
          {option()}
        </Tip>
      )}
    </Show>
  );
}

export default function SegmentedLookView(look: SegmentedLook) {
  return (
    <div class="choices refrain-segmented-alt-choices" {...look.group}>
      <For each={look.bands} keyed={(band) => band.key}>
        {(band) => (
          <div class="band refrain-segmented-alt-band">
            <Show when={band().heading}>
              {(heading) => <span class="heading refrain-segmented-alt-heading">{heading()}</span>}
            </Show>
            <For each={band().cells} keyed={(cell) => cell.key}>
              {(cell) => <Cell cell={cell()} />}
            </For>
          </div>
        )}
      </For>
    </div>
  );
}
