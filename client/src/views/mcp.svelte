<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The tool servers a scope may reach, laid out the way Claude Code
  // taught people to expect: what is already reachable, three ways to
  // add one more, and Composio as a directory.
  //
  // The page starts at the rail and caps itself at the page width,
  // which is where every content column begins (client-SPEC 4-33). It
  // keeps its own address because a tool server is a thing a person
  // returns to on its own; the way in is the settings page's MCP group.
  //
  // This file is the router. It owns the two choices that are the
  // page's own - which scope a server is written into and which
  // building's list is on screen - and hands every door one `Intake`,
  // so a door never learns what a scope is and the scope rule stays in
  // one place. The three doors are `parts/tabs`' (7-4), and the
  // tab/panel association is the part's to draw (7-8 item 6): one
  // `panel` snippet receives the lens being shown, and this file
  // answers with one exhaustive chain over the three door ids.

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

  // The three scopes Claude Code offers. Only the middle one has a
  // field on the wire; the page says what the other two are waiting for
  // rather than hiding them, because a person who used `--scope user`
  // looks for them first.
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
  import Segmented from "./parts/segmented.svelte";
  import Tabs from "./parts/tabs.svelte";
  import ByCommand from "./mcp/by_command.svelte";
  import ByJson from "./mcp/by_json.svelte";
  import ByUrl from "./mcp/by_url.svelte";
  import Composio from "./mcp/composio.svelte";
  import DesktopForm from "./desktop.svelte";
  import ContextRung from "./context_rung.svelte";
  import { reachOf } from "./mcp/reach.svelte";
  import Servers from "./mcp/servers.svelte";
  import { BuildingColumn, HALL, useBuildings } from "./shared/buildings";

  const u = ui();
  const { lang } = u;
   
  const buildings = useBuildings();

   
  let chosen = $state<Address>(HALL);
  let scope = $state<Scope>("building");
  let door = $state<Door>("command");

  // The three scopes as one question: a screen reader hears one choice
  // with three answers instead of three pressed buttons, and the two
  // that the wire cannot carry stay choosable, because the banner under
  // them is where their gap is explained.
  const scopes = $derived(SCOPES.map((each) => ({ value: each, label: say($lang, scoped(each)) })));
  // The sentence that stops every send, or null when a send may go.
  const banner = $derived.by((): string | null => {
    const said = gapOf(scope);
    return said === null ? null : say($lang, said);
  });
  const why = (): string | null => banner;

  // One question and one door for the chosen building: the list, the
  // three doors and Composio all write into the same scope, so they
  // share one reading of it and one `Intake`.
  const reach = reachOf({
    get addr() {
      return chosen;
    },
    get why() {
      return why();
    },
  });

  const lenses = $derived(DOORS.map((each) => ({ id: each, label: say($lang, doorWord(each)) })));

  // The lens id comes back as a string; this is where it becomes one of
  // the three doors again, and a name that is no door picks nothing.
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

<div class="flex w-full max-w-page flex-1 flex-col px-pane py-wide">
  <div class="mb-wide flex flex-wrap items-baseline gap-base">
    <h1 class="text-title font-title" tabindex="-1">{say($lang, "mcp_title")}</h1>
    <span class="text-note text-text-quiet">{say($lang, "mcp_scope")}</span>
    <Segmented
      label={say($lang, "mcp_scope")}
      options={scopes}
      held={scope}
      onPick={(value) => {
        scope = value;
      }}
    />
  </div>
  {#if banner !== null}
    <Banner text={banner} />
  {/if}
  <div class="flex min-h-0 flex-1 gap-wide">
    <BuildingColumn
      label={say($lang, "mcp_building")}
      buildings={$buildings}
      chosen={chosen}
      hall={say($lang, "city_hall")}
      onPick={(addr) => {
        chosen = addr;
      }}
    />
    <section class="flex min-w-0 flex-1 flex-col gap-wide">
      <div class="flex flex-col gap-wide">
        <Servers
          servers={reach.servers()}
          health={reach.health()}
          onCheck={reach.check}
          onRemove={reach.withdraw}
        />
        <div class="flex flex-col gap-base rounded-panel bg-chrome/60 p-base">
          <Tabs
            label={say($lang, "mcp_doors")}
            lenses={lenses}
            current={door}
            onPick={pick}
          >
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
      </div>
      <div>
        <h2 class="mb-base text-label font-label text-text-quiet">{say($lang, "mcp_composio")}</h2>
        <Composio intake={reach.intake} />
      </div>
      <div>
        <h2 class="mb-base text-label font-label text-text-quiet">{say($lang, "desktop_title")}</h2>
        <DesktopForm addr={chosen} />
        <ContextRung addr={chosen} />
      </div>
    </section>
  </div>
</div>
