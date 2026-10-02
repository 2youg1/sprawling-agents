<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts" module>
  // The conversation page as the shell lays it out (client-SPEC 4-33, 7H,
  // 7K): the world layer, the conversation and the right pane, each placed
  // on the column lines of the shell's one grid by the tier the page is
  // in, by whether the right pane is open and, in the panorama tier, by
  // how the person arranged the workbench. Nothing here draws a pane; it
  // decides where each one stands, which is the one decision this file
  // owns.
  import type { Tier } from "../core/prefs";
  import type { Pane, Workbench } from "../core/workbench";

  // The grid lines a region runs between, counted from one; the last of
  // the twelve columns ends on line thirteen.
  export type Lines = readonly [number, number];

  export interface Placed {
    readonly pane: Pane;
    readonly lines: Lines;
  }

  export interface Layout {
    readonly world: "none" | "beside" | "workbench";
    readonly panes: readonly Placed[];
    readonly talk: Lines;
    readonly right: Lines | null;
  }

  export type Right = "open" | "closed";

  // With the right pane open the panorama keeps the sessions and the
  // chosen session in the person's order, at two and five columns, and
  // the commits fold away (4-33's table).
  const BESIDE_RIGHT: Readonly<Record<Pane, number>> = { sessions: 2, session: 5, commits: 0 };

  export function layoutOf(tier: Tier, right: Right, bench: Workbench): Layout {
    switch (tier) {
      case "zen":
        return { world: "none", panes: [], talk: right === "open" ? [2, 7] : [4, 10], right: [7, 13] };
      case "blend":
        return right === "open"
          ? { world: "none", panes: [], talk: [2, 7], right: [7, 13] }
          : {
              world: "beside",
              panes: [
                { pane: "sessions", lines: [1, 4] },
                { pane: "commits", lines: [10, 13] },
              ],
              talk: [4, 10],
              right: null,
            };
      case "panorama":
        return workbenchOf(right === "open" ? bench.map((column) => ({ pane: column.pane, span: BESIDE_RIGHT[column.pane] })) : bench, right);
    }
  }

  // The workbench's panes side by side from the first column line, and
  // the conversation as a band under the chosen session. The band never
  // starts on the first column, which the edge keys stand at the foot of.
  function workbenchOf(columns: readonly { readonly pane: Pane; readonly span: number }[], right: Right): Layout {
    let from = 1;
    const panes = columns.flatMap((column): Placed[] => {
      if (column.span === 0) return [];
      const lines: Lines = [from, from + column.span];
      from += column.span;
      return [{ pane: column.pane, lines }];
    });
    const session = panes.find((placed) => placed.pane === "session")?.lines ?? [4, 9];
    return {
      world: "workbench",
      panes,
      talk: [Math.max(2, session[0]), session[1]],
      right: right === "open" ? [from, 13] : null,
    };
  }

  export function gridColumn(lines: Lines): string {
    return `${String(lines[0])} / ${String(lines[1])}`;
  }
</script>

<script lang="ts">
  import { MediaQuery } from "svelte/reactivity";

  import { newestWorking } from "../core/belief/live";
  import { heldIn } from "../core/belief/rooms";
  import { say } from "../core/lang";
  import { MAYOR, buildingOf, roomOf } from "../core/route";
  import { ui } from "../ui";
  import type { Address, RoundsAnswer } from "../wire";
  import { rightItem } from "./inspect/open.svelte";
  import { followingIn } from "./inspect/reading";
  import Right from "./right.svelte";
  import Talk from "./talk.svelte";
  import Divider from "./world/divider.svelte";
  import PaneMenu from "./world/pane_menu.svelte";
  import Place from "./world/place.svelte";
  import Session from "./world/session.svelte";
  import Sessions from "./world/sessions.svelte";

  interface Props {
    readonly address: Address;
    readonly tier: Tier;
    // Where the page stands: as the page itself, the one `<main>` and its
    // `<h1>`, or as a specimen on `#/gallery`, a named region under the
    // gallery's own heading. A specimen also states whether its right
    // pane is open rather than reading the person's preference, so the
    // fixture draws the same thing in every browser.
    readonly seat?: "page" | "specimen";
    readonly panel?: boolean;
  }

  const { address, tier, seat = "page", panel }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const held = u.prefs.held;
  const bench = u.prefs.workbench;
  const uid = $props.id();

  // A window narrower than the grid's columns can be read in is one
  // column: the conversation alone, and the right pane over it whole.
  const narrow = new MediaQuery("width < 768px");

  // The run the right pane speaks for: the one going, or the last one
  // this room finished; its rounds are the same question the thread
  // asks, merged by `asking`.
  const current = $derived(newestWorking($belief, address) ?? heldIn($belief, address).at(-1));
  let answer = $state<RoundsAnswer | undefined>(undefined);
  $effect(() => {
    const run = current?.run;
    answer = undefined;
    if (run === undefined) return;
    return u.conn.asking.ask({ rounds: { run } }).subscribe((answered) => {
      answer = answered !== undefined && "rounds" in answered ? answered.rounds : undefined;
    });
  });
  // What the inspector follows while nobody has opened an item; a run
  // that read nothing, changed nothing and ran nothing has nothing to
  // follow and no pane to open.
  const following = $derived(answer === undefined ? [] : followingIn(answer));
  const produced = $derived(following.length > 0);
  // An item somebody opened (`inspect/open.svelte.ts`) opens the right
  // side whatever the preference says; without one, the preference opens
  // it on what the run is doing.
  const item = $derived(rightItem());
  const open = $derived(item !== null || ((panel ?? $held.panel) && produced));

  const layout = $derived(layoutOf(narrow.current ? "zen" : tier, open ? "open" : "closed", $bench));
  // A narrow window is one column, where every region spans it whole
  // through its `narrow:` classes and stands on no line of its own.
  const at = (lines: Lines): string | undefined => (narrow.current ? undefined : gridColumn(lines));
  // The workbench's labels move its panes; beside the conversation in
  // the blend tier the world takes no input, and the labels are words.
  const arranged = $derived(layout.world === "workbench" ? "menu" : "words");
  // Each pane's name: the third is the place it draws - the city's name
  // beside the Mayor's room, the building's beside a room in one.
  const label = $derived({
    sessions: say($lang, "world_sessions"),
    session: say($lang, "world_session"),
    commits: address === MAYOR ? ($belief.city ?? say($lang, "world_city")) : buildingOf(address),
  });
  const title = $derived(address === MAYOR ? say($lang, "talk_empty_mayor") : roomOf(address));
</script>

{#snippet sessionsHead()}
  <PaneMenu pane="sessions" label={label.sessions} {arranged} />
{/snippet}
{#snippet sessionHead()}
  <PaneMenu pane="session" label={label.session} {arranged} />
{/snippet}
{#snippet placeHead()}
  <PaneMenu pane="commits" label={label.commits} {arranged} />
{/snippet}

<!-- Two rows: the page, and under it the panorama band. A pane that runs
the window's height spans both; the chosen session stands in the first
and the band in the second, so the band is as tall as it needs and never
covers the pane above it. -->
<svelte:element
  this={seat === "page" ? "main" : "section"}
  id={seat === "page" ? "main" : undefined}
  class="col-span-full row-start-2 -m-margin grid min-h-0 grid-cols-subgrid grid-rows-[minmax(0,1fr)_auto] p-margin narrow:m-0 narrow:p-0"
  aria-label={seat === "page" ? say($lang, "region_main") : title}
>
  <!-- The page's own name: a reader arriving by keyboard or screen reader
  lands on it, and `theme.css` hangs the view transition off `main h1`. -->
  <svelte:element this={seat === "page" ? "h1" : "h2"} tabindex="-1" class="sr-only">{title}</svelte:element>
  {#if layout.world !== "none"}
    <!-- The world layer, on the shell's own columns through `subgrid`, so
    a pane edge and a divider stand on a column line every other region
    stands on. Beside the conversation it is dimmed, outlined and takes
    no input; as the workbench it is the page. -->
    <div
      class={[
        "col-span-full row-[1/3] grid min-h-0 grid-cols-subgrid grid-rows-subgrid transition-opacity duration-page",
        layout.world === "beside" ? "pointer-events-none opacity-(--blend-opacity)" : "",
      ]}
      inert={layout.world === "beside"}
    >
      {#each layout.panes as placed, index (placed.pane)}
        {#if layout.world === "workbench" && layout.right === null && (index === 1 || index === 2)}
          {@const before = layout.panes[index - 1]?.pane ?? "sessions"}
          <Divider divider={index === 1 ? 0 : 1} label={label[before]} controls="{uid}-{before}" column={placed.lines[0]} />
        {/if}
        <div
          id="{uid}-{placed.pane}"
          class={[
            "flex min-h-0 flex-col",
            placed.pane === "session" ? "row-[1]" : "row-[1/3]",
            // The pane on the first column stops above the edge keys at its foot.
            placed.lines[0] === 1 ? "mb-[calc(3*var(--spacing-key)+2*var(--spacing-snug)+var(--spacing-wide))]" : "",
            layout.world === "beside" ? "rounded-panel border border-edge-panel px-pane pt-snug" : "",
          ]}
          style:grid-column={at(placed.lines)}
        >
          {#if placed.pane === "sessions"}
            <Sessions here={address} narrow={open} head={sessionsHead} />
          {:else if placed.pane === "session"}
            <Session here={address} {title} head={sessionHead} />
          {:else}
            <Place here={address} head={placeHead} />
          {/if}
        </div>
      {/each}
    </div>
  {/if}
  <section
    style:grid-column={at(layout.talk)}
    class={[
      "relative -mx-wide flex min-h-0 flex-col px-wide narrow:col-span-full narrow:mx-0 narrow:px-0",
      layout.world === "workbench" ? "row-[2] pt-wide" : "row-[1/3]",
    ]}
    aria-label={say($lang, "region_conversation")}
  >
    <Talk {address} band={layout.world === "workbench"} />
  </section>
  {#if open}
    <div
      class="row-[1/3] -mt-margin -mr-margin -mb-margin flex min-h-0 border-l border-edge-panel narrow:fixed narrow:inset-0 narrow:col-span-full narrow:m-0"
      style:grid-column={layout.right === null ? undefined : at(layout.right)}
    >
      <Right {following} current={answer} talk={address} />
    </div>
  {/if}
</svelte:element>
