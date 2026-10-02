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
    readonly world: "none" | "beside" | "workbench" | "sheet";
    readonly panes: readonly Placed[];
    readonly talk: Lines;
    readonly right: Lines | null;
  }

  export type RightSide = "open" | "closed";

  // With the right pane open the panorama keeps the sessions and the
  // chosen session in the person's order, at two and five columns, and
  // the commits fold away (4-33's table).
  const BESIDE_RIGHT: Readonly<Record<Pane, number>> = { sessions: 2, session: 5, commits: 0 };

  export function layoutOf(tier: Tier, right: RightSide, bench: Workbench): Layout {
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
  function workbenchOf(columns: readonly { readonly pane: Pane; readonly span: number }[], right: RightSide): Layout {
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

  // One column (client-SPEC 4-52): the conversation alone, or in the
  // panorama tier the world as a sheet with the conversation as its band
  // under it; the right side, when open, is a sheet over both. No region
  // stands on a line of its own.
  export function oneColumnOf(tier: Tier, bench: Workbench): Layout {
    return tier === "panorama"
      ? { world: "sheet", panes: bench.map((column) => ({ pane: column.pane, lines: [1, 13] })), talk: [1, 13], right: null }
      : { world: "none", panes: [], talk: [1, 13], right: null };
  }

  export function gridColumn(lines: Lines): string {
    return `${String(lines[0])} / ${String(lines[1])}`;
  }
</script>

<script lang="ts">
  import { untrack } from "svelte";

  import { newestWorking } from "../core/belief/live";
  import { heldIn } from "../core/belief/rooms";
  import { say } from "../core/lang";
  import { MAYOR, buildingOf, roomOf } from "../core/route";
  import { ui } from "../ui";
  import type { Address, RoundsAnswer } from "../wire";
  import { rightItem } from "./inspect/open.svelte";
  import { followingIn } from "./inspect/reading";
  import Right from "./right.svelte";
  import { watchColumns, type Columns } from "./shared/frame";
  import { leaveSheet, openSheet } from "./sheets.svelte";
  import Talk from "./talk.svelte";
  import Divider from "./world/divider.svelte";
  import PaneMenu from "./world/pane_menu.svelte";
  import Place from "./world/place.svelte";
  import Session from "./world/session.svelte";
  import Sessions from "./world/sessions.svelte";
  import SheetHead from "./world/sheet_head.svelte";

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

  // A shell narrower than the grid's columns can be read in is one
  // column (`theme.css` decides, 12-30): the conversation alone, the world
  // and the right side as sheets over it (4-52).
  let columns = $state<Columns>("twelve");
  const narrow = $derived(columns === "one");

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
  // On one column the preference does not open it: a sheet over the whole
  // conversation opens only when somebody opened an item.
  const item = $derived(rightItem());
  const open = $derived(item !== null || ((panel ?? (narrow ? false : $held.panel)) && produced));

  const layout = $derived(narrow ? oneColumnOf(tier, $bench) : layoutOf(tier, open ? "open" : "closed", $bench));
  const sheet = $derived(layout.world === "sheet");
  // A narrow window is one column, where every region spans it whole
  // through its `narrow:` classes and stands on no line of its own.
  const at = (lines: Lines): string | undefined => (narrow ? undefined : gridColumn(lines));

  // On the page itself the right side's sheet is an entry of the
  // browser's history, so the back button and the edge swipe leave it
  // (`sheets.svelte.ts`); a specimen opens and closes nothing there.
  $effect(() => {
    if (seat !== "page") return;
    const sheet = narrow && open;
    untrack(() => {
      if (sheet) openSheet("right");
      else if (!open) leaveSheet("right");
    });
  });

  // The pane the world's sheet shows, one at a time: the place first,
  // which is the git graph and the files the phone opens the world for.
  let shown = $state<Pane>("commits");
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
{#snippet unnamed()}{/snippet}

<!-- Two rows: the page, and under it the panorama band. A pane that runs
the window's height spans both; the chosen session stands in the first
and the band in the second, so the band is as tall as it needs and never
covers the pane above it. -->
<svelte:element
  this={seat === "page" ? "main" : "section"}
  id={seat === "page" ? "main" : undefined}
  {@attach (element: HTMLElement) => watchColumns(element, (next) => (columns = next))}
  class="workspace col-span-full row-start-2 -m-margin grid min-h-0 grid-cols-subgrid grid-rows-[minmax(0,1fr)_auto] p-margin narrow:relative narrow:-mx-pane narrow:mt-0 narrow:-mb-pane narrow:px-pane narrow:pt-0 narrow:pb-pane"
  aria-label={seat === "page" ? say($lang, "region_main") : title}
>
  <!-- The page's own name: a reader arriving by keyboard or screen reader
  lands on it, and `theme.css` hangs the view transition off `main h1`. -->
  <svelte:element this={seat === "page" ? "h1" : "h2"} tabindex="-1" class="sr-only">{title}</svelte:element>
  {#if layout.world !== "none"}
    <!-- The world layer, on the shell's own columns through `subgrid`, so
    a pane edge and a divider stand on a column line every other region
    stands on. Beside the conversation it is dimmed, outlined and takes
    no input; as the workbench it is the page. On one column it is a sheet
    from the left over the conversation, which stays under it as the band
    (4-52); the panes are the same elements in both, so turning a phone
    keeps them, and a draft in the band lives through. -->
    <div
      class={[
        sheet
          ? "sheet -mx-pane row-[1] grid min-h-0 grid-rows-[auto_minmax(0,1fr)] bg-page px-pane"
          : "col-span-full row-[1/3] grid min-h-0 grid-cols-subgrid grid-rows-subgrid transition-opacity duration-page",
        layout.world === "beside" ? "pointer-events-none opacity-(--blend-opacity)" : "",
      ]}
      data-side="left"
      role={sheet ? "region" : undefined}
      aria-label={sheet ? say($lang, "world_layer") : undefined}
      inert={layout.world === "beside"}
    >
      {#if sheet}
        <SheetHead
          tabs={layout.panes.map((placed) => ({ pane: placed.pane, label: label[placed.pane] }))}
          {shown}
          prefix={uid}
          onShow={(pane: Pane) => {
            shown = pane;
          }}
        />
      {/if}
      {#each layout.panes as placed, index (placed.pane)}
        {#if layout.world === "workbench" && layout.right === null && (index === 1 || index === 2)}
          {@const before = layout.panes[index - 1]?.pane ?? "sessions"}
          <Divider divider={index === 1 ? 0 : 1} label={label[before]} controls="{uid}-{before}" column={placed.lines[0]} />
        {/if}
        <div
          id="{uid}-{placed.pane}"
          class={[
            "flex min-h-0 flex-col",
            sheet ? "row-[2] pt-snug" : placed.pane === "session" ? "row-[1]" : "row-[1/3]",
            // The pane on the first column stops above the edge keys at its foot.
            !sheet && placed.lines[0] === 1 ? "mb-[calc(3*var(--spacing-key)+2*var(--spacing-snug)+var(--spacing-wide))]" : "",
            layout.world === "beside" ? "rounded-panel border border-edge-panel px-pane pt-snug" : "",
          ]}
          style:grid-column={at(placed.lines)}
          role={sheet ? "tabpanel" : undefined}
          aria-labelledby={sheet ? `${uid}-tab-${placed.pane}` : undefined}
          hidden={sheet && placed.pane !== shown}
        >
          {#if placed.pane === "sessions"}
            <Sessions here={address} narrow={open} head={sheet ? unnamed : sessionsHead} />
          {:else if placed.pane === "session"}
            <Session here={address} {title} head={sheet ? unnamed : sessionHead} />
          {:else}
            <Place here={address} head={sheet ? unnamed : placeHead} />
          {/if}
        </div>
      {/each}
    </div>
  {/if}
  <section
    style:grid-column={at(layout.talk)}
    class={[
      "relative -mx-wide flex min-h-0 flex-col px-wide narrow:col-span-full narrow:mx-0 narrow:px-0",
      layout.world === "workbench" ? "row-[2] pt-wide" : sheet ? "row-[2] pt-base" : "row-[1/3]",
    ]}
    aria-label={say($lang, "region_conversation")}
  >
    <Talk {address} band={layout.world === "workbench" || sheet} />
  </section>
  {#if open}
    <!-- On one column the right side is a sheet over the whole page, from
    the right with its close key at the top on that side (4-52): the page
    reaches the frame's edges there, and the sheet covers the page's box.
    It stands on no grid line: a line would make that grid area its box,
    inside the page's padding. -->
    <div
      class={[
        "row-[1/3] -mt-margin -mr-margin -mb-margin flex min-h-0 border-l border-edge-panel narrow:absolute narrow:inset-0 narrow:z-20 narrow:col-auto narrow:row-auto narrow:m-0 narrow:border-l-0",
        narrow ? "sheet" : "",
      ]}
      data-side="right"
      style:grid-column={layout.right === null ? undefined : at(layout.right)}
    >
      <Right {following} current={answer} talk={address} />
    </div>
  {/if}
</svelte:element>
