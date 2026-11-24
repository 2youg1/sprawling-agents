// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For } from "solid-js";
import { say } from "../../core/lang";
import type { Landing } from "../../core/landing";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { Address, RunId } from "../../wire";
import Composer from "../talk/composer_view";
import { Landed } from "../talk/delivery_view";
import { GalleryCase } from "./case";

const LANDINGS: readonly (readonly [string, Landing])[] = [
  ["dispatch · the run started in the room it was sent from", { kind: "here" }],
  [
    "dispatch · the run started in a room of its own",
    {
      kind: "elsewhere",
      run: RunId.make("00000000-0000-4000-8000-00000000f011"),
      addr: Address.make("shop/给-price-加一个测试"),
    },
  ],
];

export default function Followed() {
  const lang = storeValue(ui().lang);
  return (
    <For each={LANDINGS}>
      {([label, landing]) => (
        <GalleryCase label={label}>
          <div class="px-pane pb-pane">
            <Landed landing={landing} />
            <Composer
              placeholder={say(lang(), "talk_placeholder_mayor")}
              sending="dispatch"
              hearing
              onSend={() => false}
              onStop={() => false}
            />
          </div>
        </GalleryCase>
      )}
    </For>
  );
}
