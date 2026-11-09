<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The conversation page as the shell lays it out: placement is
  // `workspace.ts` (client D113); this file draws the panes.
  import { untrack } from "svelte";

  import { newestWorking } from "../core/belief/live";
  import { heldIn } from "../core/belief/rooms";
  import { fill, say } from "../core/lang";
  import type { Tier } from "../core/prefs";
  import { MAYOR, buildingOf, roomOf, type ItemLink } from "../core/route";
  import { stretchesOf } from "../core/stretches";
  import type { Pane } from "../core/workbench";
  import { ui } from "../ui";
  import type { Address, RoundsAnswer, Seq } from "../wire";
  import { openCall, openDocument, rightItem } from "./inspect/open.svelte";
  import { followingIn } from "./inspect/reading";
  import Right from "./right.svelte";
  import { watchColumns, type Columns } from "./shared/frame";
  import { leaveSheet, openSheet } from "./sheets.svelte";
  import Talk from "./talk.svelte";
  import Past from "./talk/past.svelte";
  import GaugeStrip from "./talk/gauge_strip.svelte";
  import { PAGE, type Seat } from "./talk/seat";
  import { sessionRun } from "./world/chosen.svelte";
  import PaneMenu from "./world/pane_menu.svelte";
  import Place from "./world/place.svelte";
  import Session from "./world/session.svelte";
  import { titleOf } from "./world/sessions";
  import Sessions from "./world/sessions.svelte";
  import SheetHead from "./world/sheet_head.svelte";
  import { gridColumn, layoutOf, oneColumnOf, type Lines } from "./workspace";

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
  const roomStretches = $derived(
    stretchesOf(
      $stretches !== undefined && "sessions" in $stretches ? [$stretches.sessions] : [],
      (room) => heldIn($belief, room),
    ),
  );
  const past = $derived.by(() => {
    if (session === undefined) return undefined;
    const found = roomStretches.find((each) => each.line.began === session);
    return found?.current === true ? undefined : (found ?? null);
  });
  // The session the chosen-session pane is about, named the way the
  // sessions column's row names it (`world/sessions.ts`'s `titleOf`): one
  // column heads that pane with it, since the row is on another tab.
  const chosenStretch = $derived(
    roomStretches.find((each) =>
      session === undefined ? each.current : each.line.began === session,
    ),
  );

  // The run the right pane speaks for: the past session's last, else the
  // one going, or the last one this room finished; its rounds are the
  // same question the thread asks, merged by `asking`.
  const current = $derived(
    past === undefined
      ? (newestWorking($belief, address) ?? heldIn($belief, address).at(-1))
      : past?.runs.at(-1),
  );
  // The run the chosen session's pane speaks for, whose frozen facts its
  // gauge states, so the conversation under it leaves them off that run's
  // first head (`talk/seat.ts`).
  const gauged = $derived(
    past === undefined ? sessionRun($belief, address) : (current?.run ?? null),
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

  const layout = $derived(
    narrow ? oneColumnOf(tier, $bench) : layoutOf(tier, open ? "open" : "closed", $bench),
  );
  const sheet = $derived(layout.world === "sheet");
  // Blend dims the side panes; panorama keeps the same chrome at full
  // opacity and takes input on both sides (client D113).
  const dimmed = $derived(tier === "blend" && layout.world === "beside");
  // Where the conversation stands (`talk/seat.ts`): panorama's gauge
  // strip inside the talk column owns the frozen facts of the gauged run.
  const column = $derived<Seat>(
    tier === "panorama" && !narrow ? { kind: "workbench", gauged } : PAGE,
  );
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
  // On one-column panorama the sheet tabs still name panes; on the
  // desktop the side panes are fixed on the silver lines and the labels
  // are words.
  const arranged = $derived(sheet ? "menu" : "words");
  // Each pane's name: the third is the place it draws - the city's name
  // beside the Mayor's room, the building's beside a room in one.
  const label = $derived({
    sessions: say($lang, "world_sessions"),
    session: say($lang, "world_session"),
    commits: address === MAYOR ? ($belief.city ?? say($lang, "world_city")) : buildingOf(address),
  });
  const baseTitle = $derived(address === MAYOR ? say($lang, "talk_empty_mayor") : roomOf(address));
  const title = $derived.by(() => {
    const line = chosenStretch?.line;
    if (line?.parent === undefined || line.parent === null) return baseTitle;
    return line.branched_from === undefined || line.branched_from === null
      ? fill(say($lang, "world_session_child"), { title: baseTitle })
      : fill(say($lang, "world_session_branch"), {
          title: baseTitle,
          at: String(line.branched_from.at_seq),
        });
  });
  const chosenTitle = $derived(chosenStretch === undefined ? baseTitle : titleOf(chosenStretch));
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

<!-- One tall row for the conversation and the side panes; on one-column
panorama the sheet sits above the band. The talk box keeps that full
height on every desktop tier (client D113). -->
<svelte:element
  this={seat === "page" ? "main" : "section"}
  id={seat === "page" ? "main" : undefined}
  {@attach (element: HTMLElement) => watchColumns(element, (next) => (columns = next))}
  class="workspace col-span-full row-start-2 -m-margin grid min-h-0 grid-cols-subgrid grid-rows-[minmax(0,1fr)_auto] p-margin narrow:relative narrow:-mx-pane narrow:mt-0 narrow:-mb-pane narrow:px-pane narrow:pt-0 narrow:pb-pane"
  aria-label={seat === "page" ? say($lang, "region_main") : title}
>
  <!-- The page's own name: a reader arriving by keyboard or screen reader
  lands on it, and `theme/motion-state.css` hangs the view transition off `main h1`. -->
  <svelte:element this={seat === "page" ? "h1" : "h2"} tabindex="-1" class="sr-only"
    >{title}</svelte:element
  >
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
          : "col-span-full row-[1/3] grid min-h-0 grid-cols-subgrid grid-rows-subgrid",
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
      {#each layout.panes as placed (placed.pane)}
        <div
          id="{uid}-{placed.pane}"
          class={[
            "flex min-h-0 flex-col",
            sheet ? "row-[2] pt-snug" : "row-[1/3]",
            // The pane on the first column stops above the edge keys at its foot.
            !sheet && placed.lines[0] === 1
              ? "mb-[calc(3*var(--spacing-key)+2*var(--spacing-snug)+var(--spacing-wide))]"
              : "",
            // Blend and panorama share one chrome so a tier change does
            // not redraw borders as a zoom; only content opacity moves.
            layout.world === "beside"
              ? "rounded-panel border border-edge-panel px-pane pt-snug"
              : "",
          ]}
          inert={dimmed && placed.pane !== "sessions"}
          style:grid-column={at(placed.lines)}
          role={sheet ? "tabpanel" : undefined}
          aria-labelledby={sheet ? `${uid}-tab-${placed.pane}` : undefined}
          hidden={sheet && placed.pane !== shown}
        >
          <!-- Content fades in blend; the panel border stays at full
          ink so raising opacity cannot read as a resize (client D113). -->
          <div
            class={[
              "flex min-h-0 flex-1 flex-col",
              dimmed ? "opacity-(--blend-opacity)" : "",
              dimmed && placed.pane === "sessions"
                ? "transition-opacity duration-page ease-leave focus-within:opacity-100 focus-within:ease-arrive"
                : "",
            ]}
          >
            {#if placed.pane === "sessions"}
              <Sessions
                here={address}
                {session}
                narrow={open}
                head={sheet ? unnamed : sessionsHead}
              />
            {:else if placed.pane === "session"}
              <Session
                here={address}
                shown={past === undefined ? undefined : gauged}
                title={sheet ? chosenTitle : undefined}
                head={sheet ? unnamed : sessionHead}
              />
            {:else}
              <Place here={address} head={sheet ? unnamed : placeHead} />
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}
  <!-- Talk column: the gauge strip stands in this column above the
  conversation landmark so its run link is not a floating bar inside the
  conversation (client D113; crates/xtask render talk). The landmark keeps
  `-mx-wide px-wide` so thread chrome may bleed into that padding without
  crossing the region's box. -->
  <div
    style:grid-column={at(layout.talk)}
    class={["flex min-h-0 flex-col narrow:col-span-full", sheet ? "row-[2] pt-base" : "row-[1/3]"]}
  >
    {#if !sheet && column.kind === "workbench"}
      <div class="-mx-wide px-wide narrow:mx-0 narrow:px-0">
        <GaugeStrip here={address} run={gauged} />
      </div>
    {/if}
    <section
      class="relative -mx-wide flex min-h-0 flex-1 flex-col px-wide narrow:mx-0 narrow:px-0"
      aria-label={say($lang, past === undefined ? "region_conversation" : "region_past_session")}
    >
      {#if past === undefined || session === undefined}
        <Talk {address} band={sheet} seat={column} />
      {:else}
        <Past {address} {session} stretch={past} band={sheet} seat={column} />
      {/if}
    </section>
  </div>
  {#if open}
    <!-- On one column the right side is a sheet over the whole page, from
    the right with its close key at the top on that side (4-52): the page
    reaches the frame's edges there, and the sheet covers the page's box.
    It stands on no grid line: a line would make that grid area its box,
    inside the page's padding. -->
    <div
      class={[
        "row-[1/3] -mt-margin -mr-margin -mb-margin flex min-h-0 border-l border-edge-panel narrow:absolute narrow:inset-0 narrow:z-(--layer-sheet) narrow:col-auto narrow:row-auto narrow:m-0 narrow:border-l-0",
        narrow ? "sheet" : "",
      ]}
      data-side="right"
      style:grid-column={layout.right === null ? undefined : at(layout.right)}
    >
      <Right {following} current={answer} talk={address} />
    </div>
  {/if}
</svelte:element>
