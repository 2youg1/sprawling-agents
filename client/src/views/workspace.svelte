<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts" module>
  // The conversation page as the shell lays it out (client-SPEC 4-33, 7H):
  // the world layer, the conversation and the right pane, each placed on
  // the column lines of the shell's one grid by the tier the page is in
  // and by whether the right pane is open. Nothing here draws a pane; it
  // decides where each one stands, which is the one decision this file
  // owns.
  import type { Tier } from "../core/prefs";

  // Where each region stands, as the column lines of the twelve-column
  // grid (4-33's table). Spelled out whole rather than built from
  // numbers, because Tailwind reads class names out of this file as text.
  export interface Layout {
    readonly world: "none" | "beside" | "workbench";
    readonly sessions: string;
    readonly session: string;
    readonly commits: string;
    readonly talk: string;
    readonly right: string;
  }

  export function layoutOf(tier: Tier, open: boolean): Layout {
    switch (tier) {
      case "zen":
        return { world: "none", sessions: "", session: "", commits: "", talk: open ? "col-[2/7]" : "col-[4/10]", right: "col-[7/-1]" };
      case "blend":
        return open
          ? { world: "none", sessions: "", session: "", commits: "", talk: "col-[2/7]", right: "col-[7/-1]" }
          : { world: "beside", sessions: "col-[1/4]", session: "", commits: "col-[10/-1]", talk: "col-[4/10]", right: "" };
      case "panorama":
        return open
          ? { world: "workbench", sessions: "col-[1/3]", session: "col-[3/8]", commits: "", talk: "col-[3/8]", right: "col-[8/-1]" }
          : { world: "workbench", sessions: "col-[1/4]", session: "col-[4/9]", commits: "col-[9/-1]", talk: "col-[4/9]", right: "" };
    }
  }
</script>

<script lang="ts">
  import { MediaQuery } from "svelte/reactivity";

  import { newestWorking } from "../core/belief/live";
  import { heldIn } from "../core/belief/rooms";
  import { say } from "../core/lang";
  import { MAYOR, roomOf } from "../core/route";
  import { ui } from "../ui";
  import type { Address, RoundsAnswer } from "../wire";
  import { closeRight, rightItem } from "./inspect/open.svelte";
  import Button from "./parts/button.svelte";
  import RefRain from "./refrain/refrain.svelte";
  import Talk from "./talk.svelte";
  import Artifact from "./talk/artifact.svelte";
  import { NOTHING, artifactsIn } from "./talk/trace";
  import Commits from "./world/commits.svelte";
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
  const artifacts = $derived(answer === undefined ? NOTHING : artifactsIn(answer.turns));
  // A run that read nothing, changed nothing and ran nothing has no pane
  // to open.
  const produced = $derived(artifacts.read !== null || artifacts.wrote !== null || artifacts.terminal !== null);
  // An item somebody opened (`inspect/open.svelte.ts`) opens the right
  // side whatever the preference says; without one, the run's own
  // artefacts stand there as before.
  const item = $derived(rightItem());
  const open = $derived(item !== null || ((panel ?? $held.panel) && produced));

  const layout = $derived(layoutOf(narrow.current ? "zen" : tier, open));
  const title = $derived(address === MAYOR ? say($lang, "talk_empty_mayor") : roomOf(address));
</script>

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
    <div
      class={[
        "col-span-full row-[1/3] grid min-h-0 grid-cols-subgrid grid-rows-subgrid transition-opacity duration-300 ease-arrive",
        layout.world === "beside" ? "pointer-events-none opacity-60" : "",
      ]}
    >
      <!-- The foot of the first column is the edge keys', so the sessions
      pane stops above them. -->
      <div class="{layout.sessions} row-[1/3] flex min-h-0 flex-col pb-[calc(3*var(--spacing-key)+2*var(--spacing-snug)+var(--spacing-wide))]">
        <Sessions here={address} narrow={open} />
      </div>
      {#if layout.session !== ""}
        <div class="{layout.session} row-[1] flex min-h-0 flex-col">
          <Session here={address} {title} />
        </div>
      {/if}
      {#if layout.commits !== ""}
        <div class="{layout.commits} row-[1/3] flex min-h-0 flex-col">
          <Commits here={address} />
        </div>
      {/if}
    </div>
  {/if}
  <section
    class={[
      layout.talk,
      "relative -mx-wide flex min-h-0 flex-col px-wide narrow:col-span-full narrow:mx-0 narrow:px-0",
      layout.world === "workbench" ? "row-[2] pt-wide" : "row-[1/3]",
    ]}
    aria-label={say($lang, "region_conversation")}
  >
    <Talk {address} band={layout.world === "workbench"} />
  </section>
  {#if produced || item !== null}
    <div
      class={[
        layout.right,
        "row-[1/3] -mt-margin -mr-margin -mb-margin min-h-0 border-l border-edge-panel narrow:fixed narrow:inset-0 narrow:col-span-full narrow:m-0",
        open ? "flex" : "hidden",
        item?.kind === "document" ? "flex-col bg-page" : "",
      ]}
    >
      {#if item?.kind === "document"}
        <div class="flex justify-end border-b border-edge px-snug py-tight">
          <Button label={say($lang, "panel_close")} tone="quiet" onPress={closeRight} />
        </div>
        <div class="min-h-0 flex-1 overflow-auto">
          <RefRain building={item.building} path={item.path} version={item.version} />
        </div>
      {:else}
        <Artifact {artifacts} {open} run={current?.run} />
      {/if}
    </div>
  {/if}
</svelte:element>
