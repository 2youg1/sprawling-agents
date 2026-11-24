// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Show } from "solid-js";
import type { CellLook, SegmentedLook, Tone } from "./segmented";
import Tip from "./control_tip";
import "./segmented.solid.css";

const FILL: Record<Tone, string> = {
  plain: "bg-raised-hover",
  alert: "bg-alert",
  accent: "bg-accent",
};

function ink(cell: CellLook, tone: Tone): string {
  switch (cell.state) {
    case "refused":
      return "aria-disabled:text-text-disabled";
    case "free":
      return "text-text-quiet hover:text-text";
    case "held":
      return tone === "plain" ? "text-text" : "text-on-accent";
  }
}

function Cell(props: { readonly cell: CellLook; readonly tone: Tone }) {
  const button = () => (
    <button
      data-control=""
      type="button"
      {...props.cell.wire}
      class={[
        "cell refrain-segmented-cell relative h-full w-full rounded-pill px-base py-tight text-label text-balance",
        ink(props.cell, props.tone),
      ]}
    >
      {props.cell.label}
    </button>
  );
  return (
    <Show when={props.cell.why} fallback={button()}>
      {(why) => (
        <Tip id={why().id} text={why().text}>
          {button()}
        </Tip>
      )}
    </Show>
  );
}

export default function SegmentedLookView(look: SegmentedLook) {
  return (
    <div class="inline-flex max-w-full items-end gap-snug" {...look.group}>
      <For each={look.bands} keyed={(band) => band.key}>
        {(band, index) => (
          <>
            <Show when={index() > 0}>
              <span aria-hidden="true" class="w-hair self-stretch bg-edge-panel" />
            </Show>
            <div class="flex min-w-0 flex-col gap-tight">
              <Show when={band().heading}>
                {(heading) => <span class="px-base text-note text-text-quiet">{heading()}</span>}
              </Show>
              <div
                class="relative grid auto-cols-fr grid-flow-col rounded-pill bg-track"
                style={{
                  "--cells": String(band().cells.length),
                  "--held": String(Math.max(band().held, 0)),
                }}
              >
                <Show when={band().held >= 0}>
                  <span
                    aria-hidden="true"
                    class={[
                      "slider refrain-segmented-slider pointer-events-none absolute inset-y-0 left-0 rounded-pill",
                      FILL[band().tone],
                    ]}
                  />
                </Show>
                <For each={band().cells} keyed={(cell) => cell.key}>
                  {(cell) => <Cell cell={cell()} tone={band().tone} />}
                </For>
              </div>
            </div>
          </>
        )}
      </For>
    </div>
  );
}
