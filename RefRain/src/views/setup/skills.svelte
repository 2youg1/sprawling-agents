<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import { Address } from "../../wire";
  import FileView from "../building/file.svelte";
  import Skills from "../building/skills.svelte";
  import EmptyState from "../parts/empty.svelte";
  import { BuildingColumn, HALL, useBuildings } from "../shared/buildings";
  import Shelves from "./shelves.svelte";
  import SkillUsage from "./skill_usage.svelte";

  const { lang } = ui();
  const buildings = useBuildings();

  let chosen = $state<Address>(HALL);
  let reading = $state<Address | null>(null);
</script>

<div class="flex min-w-0 flex-col gap-wide">
  <Shelves building={chosen} />
  <div class="grid items-start gap-wide @wide/page:grid-cols-[var(--spacing-tree)_minmax(0,1fr)]">
    <BuildingColumn
      label={say($lang, "mcp_building")}
      buildings={$buildings}
      {chosen}
      hall={say($lang, "city_hall")}
      onPick={(addr: Address) => {
        chosen = addr;
        reading = null;
      }}
    />
    <div class="flex min-w-0 flex-col gap-wide">
      <Skills
        building={chosen}
        onPick={(picked: { at: string }) => {
          reading = Address.make(picked.at);
        }}
      />
      {#if reading !== null}
        <FileView at={reading} root={chosen} />
      {:else}
        <EmptyState missing="skills_unopened" seat="region" />
      {/if}
    </div>
  </div>
  <div class="border-t border-edge pt-base">
    <SkillUsage />
  </div>
</div>
