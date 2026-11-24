// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For } from "solid-js";

import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Address } from "../../wire";
import { HALL, useBuildings } from "../shared/buildings";

interface BuildingPickerProps {
  readonly label: string;
  readonly value: Address;
  readonly onPick: (address: Address) => void;
}

export default function BuildingPicker(props: BuildingPickerProps) {
  const lang = storeValue(ui().lang);
  const buildings = storeValue(useBuildings());
  return (
    <select
      class="h-control w-tree max-w-full rounded-control border border-edge-input bg-raised px-base text-body text-text"
      aria-label={props.label}
      value={props.value}
      onChange={(event) => {
        const value = event.currentTarget.value;
        const chosen = buildings().find((building) => building === value);
        if (chosen !== undefined) props.onPick(chosen);
      }}
    >
      <For each={buildings()} keyed={true}>
        {(building) => (
          <option value={building}>
            {building === HALL ? say(lang(), "city_hall") : building}
          </option>
        )}
      </For>
    </select>
  );
}
