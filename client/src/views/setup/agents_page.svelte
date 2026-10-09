<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { AgentCatalogAnswer, AgentOffer } from "../../wire";
  import type { BoxReading } from "./agents";

  // The ACP agents page as drawn (client/Spec.lean §4-67, design H2):
  // the agents already added as one line that opens, then one box that
  // searches the bundled catalog as one types and turns into a parsed
  // consent card on a paste, the agents found on this computer as
  // consent cards, and the catalog's rows, each opening into its card.
  export interface AgentsLookProps {
    readonly answer: AgentCatalogAnswer;
    readonly text: string;
    readonly reading: BoxReading;
    // The city's reading of a pasted launch spec, once it answered.
    readonly parsed: AgentOffer | null;
    // The room "add and use here" seats an agent in, if the page knows one.
    readonly room: string | null;
    // The offers whose add went out and that the catalog has not listed since.
    readonly adding: ReadonlySet<string>;
    readonly onText: (text: string) => void;
    readonly onPaste: () => void;
    readonly onAdd: (offer: AgentOffer, here: boolean) => void;
    readonly onLogin: (agent: string, method: string) => void;
  }
</script>

<script lang="ts">
  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import Badge from "../parts/badge.svelte";
  import Button from "../parts/button.svelte";
  import Glyph from "../parts/glyph.svelte";
  import { shownOf, standingOf, signsInElsewhere } from "./agents";
  import Consent from "./consent.svelte";

  const { answer, text, reading, parsed, room, adding, onText, onPaste, onAdd, onLogin }: AgentsLookProps = $props();
  const { lang } = ui();
  const uid = $props.id();

  const shown = $derived(shownOf(answer, reading.kind === "search" ? reading.typed : ""));
  let opened = $state<string | null>(null);
  let summaryOpen = $state(false);
</script>

<div class="flex min-w-0 flex-col gap-wide">
  {#if room !== null}
    <p class="text-note text-text-quiet">{fill(say($lang, "acp_room"), { room })}</p>
  {/if}

  {#if answer.added.length > 0}
    <section class="flex flex-col gap-tight">
      <button
        type="button"
        aria-expanded={summaryOpen}
        class="flex min-h-touch items-center gap-base rounded-control px-snug text-left text-label text-text hover:wash"
        onclick={() => {
          summaryOpen = !summaryOpen;
        }}
      >
        <span class="min-w-0 flex-1 truncate">
          {fill(say($lang, "acp_added_summary"), { names: answer.added.map((line) => line.name).join(", ") })}
        </span>
        <Glyph name="chevron" size="sm" class={["shrink-0 text-text-faint", summaryOpen && "rotate-90"]} />
      </button>
      {#if summaryOpen}
        <ul class="flex flex-col border-t border-edge">
          {#each answer.added as line (line.id)}
            {@const standing = standingOf(line)}
            <li class="flex min-h-touch flex-wrap items-center gap-base border-b border-edge py-tight pl-snug">
              <span class="w-[9rem] shrink-0 text-label font-label text-text">{line.name}</span>
              <span class="flex flex-1 flex-wrap items-center gap-snug">
                <Badge text={say($lang, standing.key)} weight={standing.weight} dot />
                {#if line.seated_in.length > 0}
                  <span class="text-note text-text-quiet">
                    {fill(say($lang, "acp_seated_in"), { rooms: line.seated_in.join(", ") })}
                  </span>
                {/if}
              </span>
              {#if line.login_state === "required"}
                {#each line.auth_methods as method (method.id)}
                  <Button
                    label={method.kind === "terminal" ? say($lang, "acp_sign_in_terminal") : say($lang, "acp_sign_in")}
                    tone="secondary"
                    onPress={() => {
                      onLogin(line.id, method.id);
                    }}
                  />
                {/each}
              {/if}
            </li>
            {#if signsInElsewhere(line)}
              <li class="px-snug py-tight text-note text-text-quiet">{say($lang, "acp_subscription_elsewhere")}</li>
            {/if}
          {/each}
        </ul>
      {/if}
    </section>
  {/if}

  <div class="flex min-w-0 flex-col gap-tight">
    <label for={`${uid}-box`} class="text-note text-text-quiet">{say($lang, "acp_search")}</label>
    <div class="box-content flex h-touch min-w-0 items-center gap-snug rounded-control border border-edge-input bg-raised pl-base">
      <Glyph name="search" size="sm" class="shrink-0 text-text-faint" />
      <input
        id={`${uid}-box`}
        value={text}
        spellcheck="false"
        autocomplete="off"
        class={["h-full min-w-0 flex-1 bg-transparent text-body text-text", reading.kind === "paste" && "font-mono"]}
        oninput={(event) => {
          onText(event.currentTarget.value);
        }}
      />
      {#if text !== ""}
        <button type="button" class="h-full shrink-0 px-base text-label text-text-quiet hover:bg-raised-hover" onclick={() => {
            onText("");
          }}>
          {say($lang, "acp_clear")}
        </button>
      {:else}
        <button
          type="button"
          class="h-full shrink-0 rounded-r-control border-l border-edge-input px-pane text-label text-text hover:bg-raised-hover"
          onclick={onPaste}>{say($lang, "acp_paste")}</button
        >
      {/if}
    </div>
  </div>

  {#if reading.kind === "paste"}
    <section class="flex flex-col gap-snug">
      <h3 class="text-note text-text-quiet">{say($lang, "acp_pasted")}</h3>
      {#if parsed !== null}
        <Consent offer={parsed} named seats={room !== null} adding={adding.has(parsed.id)} {onAdd} />
      {/if}
    </section>
  {:else}
    <section class="flex flex-col gap-snug">
      <h3 class="text-note text-text-quiet">{say($lang, "acp_detected")}</h3>
      {#if shown.detected.length === 0}
        <p class="text-note text-text-faint">{say($lang, "acp_detected_none")}</p>
      {:else}
        <div class="grid grid-fit gap-base">
          {#each shown.detected as offer (offer.id)}
            <Consent {offer} named seats={room !== null} adding={adding.has(offer.id)} {onAdd} />
          {/each}
        </div>
      {/if}
    </section>

    <section class="flex flex-col gap-tight">
      <h3 class="text-note text-text-quiet">
        {say($lang, "acp_catalog")} · {fill(say($lang, "acp_catalog_snapshot"), { date: answer.snapshot.date })}
      </h3>
      <ul class="flex flex-col border-t border-edge">
        {#each shown.catalog as offer (offer.id)}
          {@const open = opened === offer.id}
          <li class={["flex flex-col border-b border-edge", open && "pb-snug"]}>
            <button
              type="button"
              aria-expanded={open}
              class={["flex min-h-touch items-center gap-base px-snug text-left hover:wash", open && "wash-strong"]}
              onclick={() => {
                opened = open ? null : offer.id;
              }}
            >
              <span class="flex-1 text-label font-label text-text">{offer.name}</span>
              {#if !open}
                <span class="text-note text-text-faint">
                  {[offer.version, offer.licence].filter((part) => part !== undefined && part !== null).join(" · ")}
                </span>
              {/if}
              <Glyph name="chevron" size="sm" class={["shrink-0 text-text-faint", open && "rotate-90"]} />
            </button>
            {#if open}
              <div class="pt-snug">
                <Consent {offer} named={false} seats={room !== null} adding={adding.has(offer.id)} {onAdd} />
              </div>
            {/if}
          </li>
        {:else}
          <li class="py-base text-note text-text-faint">{say($lang, "acp_catalog_none")}</li>
        {/each}
      </ul>
    </section>
  {/if}
</div>
