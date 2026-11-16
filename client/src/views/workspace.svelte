<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->
<script lang="ts">
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

    readonly session?: Seq | undefined;
    readonly tier: Tier;

    readonly seat?: "page" | "specimen";
    readonly panel?: boolean;

    readonly item?: ItemLink | undefined;
  }

  const { address, session, tier, seat = "page", panel, item: linked }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const held = u.prefs.held;
  const bench = u.prefs.workbench;
  const uid = $props.id();

  let columns = $state<Columns>("twelve");
  const narrow = $derived(columns === "one");

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

  const chosenStretch = $derived(
    roomStretches.find((each) =>
      session === undefined ? each.current : each.line.began === session,
    ),
  );

  const current = $derived(
    past === undefined
      ? (newestWorking($belief, address) ?? heldIn($belief, address).at(-1))
      : past?.runs.at(-1),
  );

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

  const following = $derived(answer === undefined ? [] : followingIn(answer));
  const produced = $derived(following.length > 0);

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

  const dimmed = $derived(tier === "blend" && layout.world === "beside");

  const column = $derived<Seat>(
    tier === "panorama" && !narrow ? { kind: "workbench", gauged } : PAGE,
  );

  const at = (lines: Lines): string | undefined => (narrow ? undefined : gridColumn(lines));

  $effect(() => {
    if (seat !== "page") return;
    const sheet = narrow && open;
    untrack(() => {
      if (sheet) openSheet("right");
      else if (!open) leaveSheet("right");
    });
  });

  let shown = $state<Pane>("commits");

  const arranged = $derived(sheet ? "menu" : "words");

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

<svelte:element
  this={seat === "page" ? "main" : "section"}
  id={seat === "page" ? "main" : undefined}
  {@attach (element: HTMLElement) => watchColumns(element, (next) => (columns = next))}
  class="workspace col-span-full row-start-2 -m-margin grid min-h-0 grid-cols-subgrid grid-rows-[minmax(0,1fr)_auto] p-margin narrow:relative narrow:-mx-pane narrow:mt-0 narrow:-mb-pane narrow:px-pane narrow:pt-0 narrow:pb-pane"
  aria-label={seat === "page" ? say($lang, "region_main") : title}
>
  <svelte:element this={seat === "page" ? "h1" : "h2"} tabindex="-1" class="sr-only"
    >{title}</svelte:element
  >
  {#if layout.world !== "none"}
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

            !sheet && placed.lines[0] === 1
              ? "mb-[calc(3*var(--spacing-key)+2*var(--spacing-snug)+var(--spacing-wide))]"
              : "",

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
