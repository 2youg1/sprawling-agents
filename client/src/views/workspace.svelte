<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts" module>
  // The conversation page as the shell lays it out (docs/frontend-method.md §4-33, §7H,
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

  // The shell's silver cut (client D24): the middle part, which the
  // conversation and the chosen session stand in, and the two side parts.
  const MIDDLE: Lines = [4, 10];
  const LEFT: Lines = [1, 4];
  const RIGHT: Lines = [10, 13];

  // The line the open right side starts on (client D72): two middle
  // columns past the right silver line, so a file or a letter reads at
  // about ninety monospace characters at 1920 px where the side part alone
  // held sixty. The conversation gives up those two columns and takes the
  // left side part's last two instead, keeping column 1 for the edge keys;
  // it stays wider than its 760 px reading column. Moving this one line
  // moves every tier's right side.
  const RIGHT_FROM = 8;
  const RIGHT_OPEN: Lines = [RIGHT_FROM, 13];
  const TALK_BESIDE_RIGHT: Lines = [2, RIGHT_FROM];

  // With the right side open the panorama keeps the sessions and the
  // chosen session in the person's order, at two and five columns, and
  // the commits fold away: together they end on the right side's line.
  const BESIDE_RIGHT: Readonly<Record<Pane, number>> = { sessions: 2, session: RIGHT_FROM - 3, commits: 0 };

  // The conversation keeps the middle part while the right side is
  // closed; opening the right side moves it left by the two columns the
  // right side takes, and in blend the sessions fold away with the
  // commits, because a text panel never lies under the conversation.
  export function layoutOf(tier: Tier, right: RightSide, bench: Workbench): Layout {
    switch (tier) {
      case "zen":
        return right === "open"
          ? { world: "none", panes: [], talk: TALK_BESIDE_RIGHT, right: RIGHT_OPEN }
          : { world: "none", panes: [], talk: MIDDLE, right: null };
      case "blend":
        return right === "open"
          ? { world: "none", panes: [], talk: TALK_BESIDE_RIGHT, right: RIGHT_OPEN }
          : {
              world: "beside",
              panes: [
                { pane: "sessions", lines: LEFT },
                { pane: "commits", lines: RIGHT },
              ],
              talk: MIDDLE,
              right: null,
            };
      case "panorama":
        return workbenchOf(right === "open" ? bench.map((column) => ({ pane: column.pane, span: BESIDE_RIGHT[column.pane] })) : bench, right);
    }
  }

  // The workbench's panes side by side from the first column line, and
  // the conversation under the chosen session, the taller of the two by
  // the silver ratio. The conversation never starts on the first column,
  // which the edge keys stand at the foot of.
  function workbenchOf(columns: readonly { readonly pane: Pane; readonly span: number }[], right: RightSide): Layout {
    let from = 1;
    const panes = columns.flatMap((column): Placed[] => {
      if (column.span === 0) return [];
      const lines: Lines = [from, from + column.span];
      from += column.span;
      return [{ pane: column.pane, lines }];
    });
    const session = panes.find((placed) => placed.pane === "session")?.lines ?? MIDDLE;
    return {
      world: "workbench",
      panes,
      talk: [Math.max(2, session[0]), session[1]],
      right: right === "open" ? [from, 13] : null,
    };
  }

  // One column (client/Spec.lean §4-52): the conversation alone, or in the
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
  import { MAYOR, buildingOf, roomOf, type ItemLink } from "../core/route";
  import { stretchesOf } from "../core/stretches";
  import { ui } from "../ui";
  import type { Address, RoundsAnswer, Seq } from "../wire";
  import { openCall, openDocument, rightItem } from "./inspect/open.svelte";
  import { followingIn } from "./inspect/reading";
  import Right from "./right.svelte";
  import { watchColumns, type Columns } from "./shared/frame";
  import { leaveSheet, openSheet } from "./sheets.svelte";
  import Talk from "./talk.svelte";
  import Past from "./talk/past.svelte";
  import Divider from "./world/divider.svelte";
  import PaneMenu from "./world/pane_menu.svelte";
  import Place from "./world/place.svelte";
  import Session from "./world/session.svelte";
  import Sessions from "./world/sessions.svelte";
  import SheetHead from "./world/sheet_head.svelte";

  interface Props {
    readonly address: Address;
    // The session of that room in main: absent is its current session.
    readonly session?: Seq | undefined;
    readonly tier: Tier;
    // Where the page stands: as the page itself, the one `<main>` and its
    // `<h1>`, or as a specimen on `#/gallery`, a named region under the
    // gallery's own heading. A specimen also states whether its right
    // pane is open rather than reading the person's preference, so the
    // fixture draws the same thing in every browser.
    readonly seat?: "page" | "specimen";
    readonly panel?: boolean;
    // The item a link asked the right side to open beside this
    // conversation (client/Spec.lean §4-63).
    readonly item?: ItemLink | undefined;
  }

  const { address, session, tier, seat = "page", panel, item: linked }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const held = u.prefs.held;
  const bench = u.prefs.workbench;
  const uid = $props.id();

  // A shell narrower than the grid's columns can be read in is one
  // column (`theme/surface.css` decides, client D30): the conversation alone, the world
  // and the right side as sheets over it (4-52).
  let columns = $state<Columns>("twelve");
  const narrow = $derived(columns === "one");

  // An earlier session the address bar names, which main reads rather
  // than talks to (`talk/past.svelte`): `undefined` is the room's current
  // session, `null` one the city has not answered for.
  const stretches = $derived(u.conn.asking.ask({ sessions: { room: address } }));
  const past = $derived.by(() => {
    if (session === undefined) return undefined;
    const answer = $stretches !== undefined && "sessions" in $stretches ? [$stretches.sessions] : [];
    const found = stretchesOf(answer, (room) => heldIn($belief, room)).find((each) => each.line.began === session);
    return found?.current === true ? undefined : (found ?? null);
  });

  // The run the right pane speaks for: the past session's last, else the
  // one going, or the last one this room finished; its rounds are the
  // same question the thread asks, merged by `asking`.
  const current = $derived(
    past === undefined ? (newestWorking($belief, address) ?? heldIn($belief, address).at(-1)) : past?.runs.at(-1),
  );
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
  // A link opens its item once, when the page arrives at it; nothing a
  // link names is sent or saved, and what is open stays the inspector's.
  $effect(() => {
    const asked = linked;
    if (asked === undefined) return;
    untrack(() => {
      if (asked.kind === "call") openCall(asked);
      else openDocument(asked);
    });
  });
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

<!-- Two rows: the page, and under it the conversation in the panorama
tier. A pane that runs the window's height spans both. On the workbench
the chosen session stands in the first and the conversation in the
second, cut 1 : √2 by the shell's one ratio so the conversation is the
taller; on one column the second row is the band, as tall as it needs. -->
<svelte:element
  this={seat === "page" ? "main" : "section"}
  id={seat === "page" ? "main" : undefined}
  {@attach (element: HTMLElement) => watchColumns(element, (next) => (columns = next))}
  class={[
    "workspace col-span-full row-start-2 -m-margin grid min-h-0 grid-cols-subgrid p-margin narrow:relative narrow:-mx-pane narrow:mt-0 narrow:-mb-pane narrow:px-pane narrow:pt-0 narrow:pb-pane",
    layout.world === "workbench" ? "grid-rows-[minmax(0,calc(100%/(1+var(--silver))))_minmax(0,1fr)]" : "grid-rows-[minmax(0,1fr)_auto]",
  ]}
  aria-label={seat === "page" ? say($lang, "region_main") : title}
>
  <!-- The page's own name: a reader arriving by keyboard or screen reader
  lands on it, and `theme/motion-state.css` hangs the view transition off `main h1`. -->
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
      ]}
      data-side="left"
      role={sheet ? "region" : undefined}
      aria-label={sheet ? say($lang, "world_layer") : undefined}
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
            // Beside the conversation the world is dimmed and takes no
            // input, but for the sessions, which move main to a session.
            layout.world === "beside" && placed.pane !== "sessions" ? "pointer-events-none opacity-(--blend-opacity)" : "",
            layout.world === "beside" && placed.pane === "sessions" ? "opacity-(--blend-opacity) transition-opacity duration-page hover:opacity-100 focus-within:opacity-100" : "",
          ]}
          inert={layout.world === "beside" && placed.pane !== "sessions"}
          style:grid-column={at(placed.lines)}
          role={sheet ? "tabpanel" : undefined}
          aria-labelledby={sheet ? `${uid}-tab-${placed.pane}` : undefined}
          hidden={sheet && placed.pane !== shown}
        >
          {#if placed.pane === "sessions"}
            <Sessions here={address} {session} narrow={open} head={sheet ? unnamed : sessionsHead} />
          {:else if placed.pane === "session"}
            <Session here={address} shown={past === undefined ? undefined : (current?.run ?? null)} {title} head={sheet ? unnamed : sessionHead} />
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
    aria-label={say($lang, past === undefined ? "region_conversation" : "region_past_session")}
  >
    {#if past === undefined}
      <Talk {address} band={sheet} />
    {:else}
      <Past {address} stretch={past} band={sheet} />
    {/if}
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
