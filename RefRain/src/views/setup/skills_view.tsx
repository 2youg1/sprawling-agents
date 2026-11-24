// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createSignal, Show } from "solid-js";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { Address } from "../../wire";
import FileView from "../building/file_view";
import Skills from "../building/skills_view";
import EmptyState from "../parts/empty.solid";
import { HALL, useBuildings } from "../shared/buildings";
import BuildingColumn from "../shared/buildings.solid";
import Shelves from "./shelves_view";
import SkillUsage from "./skill_usage_view";

export default function SkillsSection() {
  const lang = storeValue(ui().lang);
  const buildings = storeValue(useBuildings());
  const [chosen, setChosen] = createSignal<Address>(HALL);
  const [reading, setReading] = createSignal<Address>();

  return (
    <div class="flex min-w-0 flex-col gap-wide">
      <Shelves building={chosen()} />
      <div class="grid items-start gap-wide @wide/page:grid-cols-[var(--spacing-tree)_minmax(0,1fr)]">
        <BuildingColumn
          label={say(lang(), "mcp_building")}
          buildings={buildings()}
          chosen={chosen()}
          hall={say(lang(), "city_hall")}
          onPick={(address) => {
            setChosen(address);
            setReading(undefined);
          }}
        />
        <div class="flex min-w-0 flex-col gap-wide">
          <Skills
            building={chosen()}
            onPick={(selected) => {
              setReading(Address.make(selected.at));
            }}
          />
          <Show when={reading()} fallback={<EmptyState missing="skills_unopened" seat="region" />}>
            {(at) => <FileView at={at()} root={chosen()} />}
          </Show>
        </div>
      </div>
      <div class="border-t border-edge pt-base">
        <SkillUsage />
      </div>
    </div>
  );
}
