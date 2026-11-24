// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createUniqueId, For } from "solid-js";
import { say } from "../../../core/lang";
import { storeValue } from "../../../core/solid_store";
import { ui } from "../../../ui";
import ControlTip from "../../parts/control_tip";
import { GalleryCase } from "../case";

function TipButton(props: {
  readonly standing?: boolean;
  readonly side?: "above" | "right";
  readonly placing?: "by-engine" | "against-wrapper";
}) {
  const id = createUniqueId();
  const lang = storeValue(ui().lang);
  return (
    <ControlTip id={id} text={say(lang(), "part_why_halted")} {...props}>
      <button
        type="button"
        class="rounded-control px-base py-tight text-label text-text-quiet"
        data-control="surface"
        aria-describedby={id}
      >
        {say(lang(), "part_save")}
      </button>
    </ControlTip>
  );
}

const PLACED = [
  { label: "tip · above, against the anchor", side: "above", placing: "by-engine" },
  { label: "tip · above, against the wrapper", side: "above", placing: "against-wrapper" },
  { label: "tip · to the right, against the anchor", side: "right", placing: "by-engine" },
  { label: "tip · to the right, against the wrapper", side: "right", placing: "against-wrapper" },
] as const;

export default function TipCase() {
  return (
    <GalleryCase label="tip · a hint on a control named by its own text">
      <TipButton />
    </GalleryCase>
  );
}

export function PlacedTipCases() {
  return (
    <For each={PLACED}>
      {(placement) => (
        <GalleryCase label={placement.label}>
          <div class="flex justify-center py-section">
            <TipButton side={placement.side} placing={placement.placing} standing />
          </div>
        </GalleryCase>
      )}
    </For>
  );
}
