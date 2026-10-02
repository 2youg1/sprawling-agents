<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The first screen: three things a person can do next, each one a
  // card they can click rather than a hint line they read past (ux
  // A4). Each card leaves this screen by doing the thing it names, so
  // there is no separate "next" to press: walking the welcome and
  // starting the work are the same gesture.
  //
  // **Without a main model the provider card comes first and says so**
  // - it is the one step every other card waits on, and a person who
  // assigns work before it meets a refusal instead of a run. The work
  // card stays a link, but its hint says what it is waiting for. Once a
  // main model exists the provider card goes away: a card pointing at a
  // finished errand is a card nobody should meet.
  //
  // **`assign work` writes through the draft door.** The composer box
  // in the room the card opens reads its first text from
  // `PreferenceDoor.draft`, so this is the same unsent-message row a
  // reload keeps - one door, one row per room, never a second place a
  // draft is kept.

  import Glyph from "./parts/glyph.svelte";
  import { QUERIES } from "../core/asking";
  import { say } from "../core/lang";
  import { MAYOR, toFragment } from "../core/route";
  import { ui } from "../ui";

  interface Props {
    // Whether this layout is the page or a region inside one: a
    // document may have exactly one heading of the page's own rank.
    readonly rank?: "page" | "section" | undefined;
  }

  const { rank = "page" }: Props = $props();

  const u = ui();
  const { lang } = u;
  const endpoints = u.conn.asking.ask(QUERIES.endpoints);

  const mainReady = $derived.by((): boolean => {
    const held = $endpoints;
    if (held === undefined || !("endpoints" in held)) return false;
    return held.endpoints.chosen.some((each) => each.tag === "main");
  });

  // The card is a link like any other and the draft is written before
  // the address bar moves; marking the welcome walked is what keeps the
  // shell from sending the person back here in the next breath.
  function assignWork(): void {
    u.prefs.setDraft(MAYOR, say($lang, "welcome_card_work_draft"));
    u.prefs.setWelcomed(true);
  }

  function walked(): void {
    u.prefs.setWelcomed(true);
  }
</script>

<!-- The welcome has no rail and no strip of facts, so it is the one
page drawn in the middle of the window: one column the width of a page,
its title and its three doors sharing one left edge. -->
<div class="mx-auto flex w-full max-w-page min-w-0 flex-1 flex-col px-wide pt-[18vh] pb-section">
  {#if rank === "page"}
    <h1 tabindex="-1" class="mb-wide text-title font-title">{say($lang, "welcome_title")}</h1>
  {:else}
    <h2 class="mb-wide text-title font-title">{say($lang, "welcome_title")}</h2>
  {/if}
  <div class="grid w-full grid-cols-[repeat(auto-fit,minmax(min(320px,100%),1fr))] gap-base">
    {#if !mainReady}
      <a
        href={toFragment({ kind: "setup" })}
        class="rise flex items-start gap-base rounded-card bg-raised px-pane py-base transition-colors hover:bg-raised-hover"
        onclick={walked}
      >
        <Glyph name="setup" class="mt-tight shrink-0 text-text-quiet" />
        <span class="flex min-w-0 flex-col gap-tight">
          <span class="flex min-w-0 flex-wrap items-baseline gap-snug">
            <span class="text-label font-label">{say($lang, "welcome_card_provider")}</span>
            <span class="text-note font-label text-text">{say($lang, "welcome_card_provider_first")}</span>
          </span>
          <span class="text-note text-text-quiet">{say($lang, "welcome_card_provider_hint")}</span>
        </span>
      </a>
    {/if}
    <a
      href={toFragment({ kind: "talk", address: MAYOR })}
      class="rise flex items-start gap-base rounded-card bg-raised px-pane py-base transition-colors hover:bg-raised-hover"
      onclick={assignWork}
    >
      <Glyph name="talk" class="mt-tight shrink-0 text-text-quiet" />
      <span class="flex min-w-0 flex-col gap-tight">
        <span class="text-label font-label">{say($lang, "welcome_card_work")}</span>
        <span class="text-note text-text-quiet">
          {say($lang, mainReady ? "welcome_card_work_hint" : "welcome_card_work_hint_unready")}
        </span>
      </span>
    </a>
    <a
      href={toFragment({ kind: "city" })}
      class="rise flex items-start gap-base rounded-card bg-raised px-pane py-base transition-colors hover:bg-raised-hover"
      onclick={walked}
    >
      <Glyph name="city" class="mt-tight shrink-0 text-text-quiet" />
      <span class="flex min-w-0 flex-col gap-tight">
        <span class="text-label font-label">{say($lang, "welcome_card_city")}</span>
        <span class="text-note text-text-quiet">{say($lang, "welcome_card_city_hint")}</span>
      </span>
    </a>
  </div>
</div>
