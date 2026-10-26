<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The shared workspace entries on the city and building pages
  // (client/Spec.lean §4-50a): one link to the existing raise page, and,
  // on a building page, a native select for the buildings and adoptable
  // folders the city already answered with. Selection only navigates;
  // creation and adoption still wait for confirmation on the raise page.
  import { QUERIES } from "../../core/asking";
  import { say } from "../../core/lang";
  import { buildingDestination, toFragment } from "../../core/route";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";
  import { buildingsOf } from "./buildings";

  interface Props {
    // The building page's current address. The city page omits it and
    // keeps its existing building table as the way to switch.
    readonly current?: Address | undefined;
  }

  const { current }: Props = $props();

  const u = ui();
  const { lang } = u;
  const city = u.conn.asking.ask(QUERIES.city);
  const buildings = $derived($city !== undefined && "city" in $city ? $city.city.buildings : []);
  const options = $derived(
    buildingsOf($city).flatMap((addr) => {
      const held = buildings.find((each) => each.addr === addr);
      return held === undefined ? [] : [held];
    }),
  );

  function pick(raw: string): void {
    const held = buildings.find((each) => each.addr === raw);
    if (held === undefined) return;
    u.go(buildingDestination(held.addr, buildings).view);
  }
</script>

<div class="flex flex-wrap items-center gap-base">
  {#if current !== undefined && options.length > 0}
    <select
      class="h-control max-w-full rounded-control border border-edge-input bg-raised px-base text-body text-text"
      aria-label={say($lang, "workspace_switch")}
      value={current}
      onchange={(event) => {
        pick(event.currentTarget.value);
      }}
    >
      {#each options as each (each.addr)}
        <option value={each.addr}>{each.addr}{each.adoptable ? ` · ${say($lang, "city_adoptable")}` : ""}</option>
      {/each}
    </select>
  {/if}
  <a
    class="inline-flex h-control items-center rounded-control bg-raised px-base text-label text-text transition-colors ease-leave hover:bg-raised-hover hover:ease-arrive still:transition-none"
    href={toFragment({ kind: "raise" })}>{say($lang, "workspace_new_import")}</a
  >
</div>
