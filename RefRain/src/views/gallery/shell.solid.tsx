// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For } from "solid-js";
import type { Tier } from "../../core/prefs";
import { ui } from "../../ui";
import type { Address as AddressType } from "../../wire";
import { GalleryCase } from "./case";
import { recordsAt } from "./resulted.solid";
import { EMPTY_ROOM, ROOM, answering } from "./shelled";
import ShellSpecimen from "./shell_specimen";
import Stand from "./stand.solid";
interface Shown {
  readonly label: string;
  readonly room: AddressType;
  readonly tier: Tier;
  readonly panel: boolean;
}

const SHOWN: readonly Shown[] = [
  { label: "shell · an empty room, in zen", room: EMPTY_ROOM, tier: "zen", panel: false },
  { label: "shell · a working room, in zen", room: ROOM, tier: "zen", panel: false },
  { label: "shell · a working room, in blend", room: ROOM, tier: "blend", panel: false },
  { label: "shell · a working room, in panorama", room: ROOM, tier: "panorama", panel: false },
  { label: "shell · the right pane open, in zen", room: ROOM, tier: "zen", panel: true },
];

export default function ShellCases() {
  const records = recordsAt(ui().now());
  return (
    <For each={SHOWN}>
      {(shown) => (
        <GalleryCase label={shown.label} width={1440}>
          <Stand
            link={{ kind: "live", city: "sprawling" }}
            unread={[]}
            waiting={[]}
            answers={answering(shown.panel)}
            records={records}
            prepare={(preferences) => preferences.setPanel(shown.panel)}
          >
            {() => <ShellSpecimen address={shown.room} tier={shown.tier} height={860} />}
          </Stand>
        </GalleryCase>
      )}
    </For>
  );
}
