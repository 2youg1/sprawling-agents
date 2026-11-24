// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { createMemo, Show } from "solid-js";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { lookOf, type EmptySeat, type EmptyStateProps } from "./empty";
import Glyph from "./glyph.solid";

const SEATS: Record<EmptySeat, string> = {
  centred: "items-center px-pane py-section text-center",
  region:
    "items-start rounded-card border border-dashed border-edge-input px-wide py-wide text-left",
  inset: "items-start px-wide py-wide text-left",
};

export default function EmptyState(props: EmptyStateProps<JSX.Element>) {
  const lang = storeValue(ui().lang);
  const look = createMemo(() => lookOf(props, say(lang(), props.missing)));
  return (
    <div class={["flex w-full flex-col gap-base", SEATS[look().seat]]}>
      <div {...look().shapeWire}>
        <Show
          when={look().shape !== undefined}
          fallback={<Glyph name="ring" class="text-text-faint" />}
        >
          {look().shape}
        </Show>
      </div>
      <p class="max-w-measure text-note text-text-quiet">{look().sentence}</p>
      {look().action}
    </div>
  );
}
