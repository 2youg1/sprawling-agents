<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The world layer's sessions pane (client-SPEC 7K): every room that has
  // run anything, grouped by building, one row each - a state dot, the
  // room, how long its run has been going or that it waits or is done,
  // and the task under it. The room the conversation is in is the chosen
  // row; any other row is a link to that room's conversation.
  //
  // It reads the runs the page already believes (`belief.rooms`, the
  // per-room index the city panel reads); only the context bar under each
  // row asks the city, for that row's run (`context_bar.svelte`).
  import type { Snippet } from "svelte";

  import { heldIn } from "../../core/belief/rooms";
  import type { RunBelief } from "../../core/belief";
  import { say } from "../../core/lang";
  import { buildingOf, roomOf, toFragment } from "../../core/route";
  import { lasted } from "../../core/time";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";
  import ContextBar from "./context_bar.svelte";

  interface Props {
    // The room the conversation is in.
    readonly here: Address;
    // Whether the pane is as narrow as the right pane leaves it, which
    // drops the second line and the time.
    readonly narrow: boolean;
    // The pane's label: a menu that moves it, in the panorama tier.
    readonly head: Snippet;
  }

  const { here, narrow, head }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;

  type State = "run" | "ask" | "done";

  function stateOf(run: RunBelief): State {
    switch (run.doing.kind) {
      case "frozen":
        return "done";
      case "waiting":
        return "ask";
      case "unknown":
      case "thinking":
      case "calling":
        return "run";
    }
  }

  // A running row says how long its run has gone, so the pane is redrawn
  // once a second while anything runs and not at all otherwise.
  let now = $state(u.now());
  $effect(() => {
    if ($belief.live.length === 0) return;
    const tick = setInterval(() => {
      now = u.now();
    }, 1000);
    return () => {
      clearInterval(tick);
    };
  });

  // One row per room, the room's newest run speaking for it, grouped by
  // building in the order buildings first ran something.
  const groups = $derived.by(() => {
    const rows = [...$belief.rooms.keys()].flatMap((room): { building: string; room: Address; run: RunBelief }[] => {
      const run = heldIn($belief, room).at(-1);
      const addr = run?.addr ?? null;
      return run === undefined || addr === null ? [] : [{ building: buildingOf(addr), room: addr, run }];
    });
    const buildings = rows.map((row) => row.building).filter((building, at, all) => all.indexOf(building) === at);
    return buildings.map((building) => ({
      building,
      rows: rows.filter((row) => row.building === building).sort((a, b) => (b.run.started ?? 0) - (a.run.started ?? 0)),
    }));
  });

  function when(run: RunBelief, state: State): string {
    switch (state) {
      case "run":
        return run.started === null ? "" : lasted(now - run.started);
      case "ask":
        return say($lang, "world_waiting");
      case "done":
        return say($lang, "world_frozen");
    }
  }

  const DOT: Record<State, string> = {
    run: "bg-accent shadow-[0_0_0_3px_color-mix(in_oklch,var(--color-accent)_14%,transparent)]",
    ask: "bg-alert shadow-[0_0_0_3px_color-mix(in_oklch,var(--color-alert)_12%,transparent)]",
    done: "border-[1.5px] border-mark",
  };
</script>

<section class="flex min-h-0 flex-1 flex-col overflow-hidden" aria-label={say($lang, "world_sessions")}>
  {@render head()}
  <div class="min-h-0 flex-1 overflow-y-auto">
    {#each groups as group (group.building)}
      <h3 class="mt-base mb-tight text-note text-text-faint first:mt-0">{group.building}</h3>
      <ul>
        {#each group.rows as row (row.room)}
          {@const state = stateOf(row.run)}
          <li>
            <a
              href={toFragment({ kind: "talk", address: row.room })}
              class={[
                "relative -mx-snug grid grid-cols-[12px_minmax(0,1fr)_auto] gap-x-base rounded-card px-snug py-snug",
                row.room === here ? "wash-strong" : "hover:wash",
              ]}
              aria-current={row.room === here ? "page" : undefined}
            >
              {#if row.room === here}
                <span class="absolute top-[10px] bottom-[10px] left-0 w-hair rounded-pill bg-accent" aria-hidden="true"></span>
              {/if}
              <span class={["mt-snug size-dot rounded-pill", DOT[state]]} aria-hidden="true"></span>
              <span class="truncate font-label text-text">{roomOf(row.room)}</span>
              {#if !narrow}
                <span class="figure text-note text-text-faint">{when(row.run, state)}</span>
                <span class="col-start-2 col-end-4 truncate text-note text-text-quiet">{row.run.task ?? ""}</span>
                <ContextBar room={row.room} run={row.run.run} />
              {/if}
            </a>
          </li>
        {/each}
      </ul>
    {/each}
  </div>
</section>
