<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { Key } from "../core/lang";

  type Door = "command" | "url" | "json";

  const DOORS: readonly Door[] = ["command", "url", "json"];

  function doorWord(door: Door): Key {
    switch (door) {
      case "command":
        return "mcp_door_command";
      case "url":
        return "mcp_door_url";
      case "json":
        return "mcp_door_json";
    }
  }

  type Scope = "city" | "building" | "machine";

  const SCOPES: readonly Scope[] = ["city", "building", "machine"];

  function scoped(which: Scope): Key {
    switch (which) {
      case "city":
        return "mcp_scope_city";
      case "building":
        return "mcp_scope_building";
      case "machine":
        return "mcp_scope_machine";
    }
  }

  function gapOf(which: Scope): Key | null {
    switch (which) {
      case "city":
        return "mcp_gap_scope_city";
      case "building":
        return null;
      case "machine":
        return "mcp_gap_scope_machine";
    }
  }
</script>

<script lang="ts">
  import type { Address } from "../wire";
  import { say } from "../core/lang";
  import type { Lens } from "./parts/tabs.svelte";
  import { ui } from "../ui";
  import Banner from "./parts/banner.svelte";
  import Page from "./parts/page.svelte";
  import Segmented from "./parts/segmented.svelte";
  import Tabs from "./parts/tabs.svelte";
  import ByCommand from "./mcp/by_command.svelte";
  import ByJson from "./mcp/by_json.svelte";
  import ByUrl from "./mcp/by_url.svelte";
  import Composio from "./mcp/composio.svelte";
  import DesktopForm from "./desktop.svelte";
  import { reachOf } from "./mcp/reach.svelte";
  import Servers from "./mcp/servers.svelte";
  import Shells from "./mcp/shells.svelte";
  import McpUsage from "./mcp/usage.svelte";
  import { BuildingColumn, HALL, useBuildings } from "./shared/buildings";

  interface Props {
    readonly rank?: "page" | "section" | undefined;
  }

  const { rank = "page" }: Props = $props();

  const u = ui();
  const { lang } = u;

  const buildings = useBuildings();

  let chosen = $state<Address>(HALL);
  let scope = $state<Scope>("building");
  let door = $state<Door>("command");

  const scopes = $derived(SCOPES.map((each) => ({ value: each, label: say($lang, scoped(each)) })));
  const banner = $derived.by((): string | null => {
    const said = gapOf(scope);
    return said === null ? null : say($lang, said);
  });
  const why = (): string | null => banner;

  const reach = reachOf({
    get addr() {
      return chosen;
    },
    get why() {
      return why();
    },
  });

  const lenses = $derived(DOORS.map((each) => ({ id: each, label: say($lang, doorWord(each)) })));

  function doorOf(id: string): Door | null {
    for (const each of DOORS) {
      if (each === id) return each;
    }
    return null;
  }

  function pick(id: string): void {
    const picked = doorOf(id);
    if (picked !== null) door = picked;
  }
</script>

{#snippet scopeControl()}
  <span class="text-note text-text-quiet">{say($lang, "mcp_scope")}</span>
  <Segmented
    label={say($lang, "mcp_scope")}
    options={scopes}
    held={scope}
    onPick={(value) => {
      scope = value;
    }}
  />
{/snippet}

<Page title={say($lang, "mcp_title")} aside={scopeControl} {rank}>
  {#if banner !== null}
    <Banner text={banner} />
  {/if}
  <div class="@container flex min-h-0 min-w-0 flex-1">
    <div
      class="flex min-h-0 min-w-0 flex-1 flex-col gap-wide @min-[64rem]:silver-columns @min-[64rem]:items-start"
    >
      <BuildingColumn
        label={say($lang, "mcp_building")}
        buildings={$buildings}
        {chosen}
        hall={say($lang, "city_hall")}
        onPick={(addr) => {
          chosen = addr;
        }}
      />
      <section class="flex min-w-0 flex-col gap-wide" aria-label={say($lang, "mcp_doors")}>
        <Servers
          servers={reach.servers()}
          health={reach.health()}
          onCheck={reach.check}
          onRemove={reach.withdraw}
        />
        <div class="flex flex-col gap-base border-t border-edge pt-base">
          <h2 class="text-note text-text-faint">{say($lang, "mcp_doors")}</h2>
          <Tabs label={say($lang, "mcp_doors")} {lenses} current={door} onPick={pick}>
            {#snippet panel(lens: Lens)}
              {#if lens.id === "command"}
                <ByCommand intake={reach.intake} />
              {:else if lens.id === "url"}
                <ByUrl
                  intake={reach.intake}
                  onCommandDoor={() => {
                    door = "command";
                  }}
                />
              {:else if lens.id === "json"}
                <ByJson intake={reach.intake} />
              {/if}
            {/snippet}
          </Tabs>
        </div>
      </section>
      <div class="flex min-w-0 flex-col gap-wide">
        <section class="flex flex-col gap-base border-t border-edge pt-base">
          <h2 class="text-note text-text-faint">{say($lang, "mcp_composio")}</h2>
          <Composio intake={reach.intake} />
        </section>
        <section class="flex flex-col gap-base border-t border-edge pt-base">
          <h2 class="text-note text-text-faint">{say($lang, "desktop_title")}</h2>
          <DesktopForm addr={chosen} />
        </section>
        <div class="border-t border-edge pt-base">
          <McpUsage />
        </div>
        <div class="border-t border-edge pt-base">
          <Shells />
        </div>
      </div>
    </div>
  </div>
</Page>
