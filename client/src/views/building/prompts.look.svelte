<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How the prompts section is drawn, and nothing else (`./prompts.ts`,
  // client/Spec.lean 4-50, its ninth part): the room it is about, the
  // bytes each slot of the newest run here was sent, when a change takes
  // effect, and one card per document in the order they are sent. A
  // document the next run does not receive is drawn in the faint ink.
  import Button from "../parts/button.svelte";
  import type { CardAction, PromptsLook } from "./prompts";

  const look: PromptsLook = $props();

  function actionKey(action: CardAction): string {
    return action.kind === "open" ? "open" : action.step;
  }
</script>

<div class="flex min-w-0 flex-col gap-wide">
  {#if look.rooms.length > 1}
    <div class="flex flex-wrap items-center gap-snug" role="group" aria-label={look.roomsLabel}>
      {#each look.rooms as room (room.label)}
        <Button
          label={room.label}
          tone={room.current ? "secondary" : "quiet"}
          onPress={room.onPress}
        />
      {/each}
    </div>
  {/if}
  <div class="flex min-w-0 flex-col gap-snug">
    <p class="text-note text-text-quiet">{look.heading}</p>
    {#if look.slots.length > 0}
      <ol class="flex flex-wrap gap-x-wide gap-y-snug text-note">
        {#each look.slots as slot (slot.key)}
          <li class="flex items-baseline gap-snug">
            <span class="text-text-faint">{slot.slot}</span>
            <span class="figure">{slot.size}</span>
          </li>
        {/each}
      </ol>
    {/if}
    {#if look.run !== undefined}
      <a class="w-fit text-note text-accent underline-offset-2 hover:underline" href={look.run.href}
        >{look.run.label}</a
      >
    {/if}
    <p class="text-note text-text-faint">{look.note}</p>
  </div>
  <ul class="flex min-w-0 flex-col gap-snug" aria-label={look.documentsLabel}>
    {#each look.cards as card (card.key)}
      <li class="flex min-w-0 flex-col gap-tight rounded-panel bg-raised p-snug text-note">
        <span class="flex min-w-0 items-baseline gap-base">
          <span class={["shrink-0 font-mono", card.sent ? "text-text" : "text-text-faint"]}
            >{card.title}</span
          >
          <span class="min-w-0 flex-1 truncate text-text-faint">{card.about}</span>
          <span class="shrink-0 text-text-faint">{card.layer}</span>
        </span>
        <span class="min-w-0 truncate font-mono text-text-faint">{card.at}</span>
        <span class={["min-w-0 wrap-anywhere", card.sent ? "text-text-quiet" : "text-text-faint"]}>
          {card.state}{#if card.standing !== undefined}
            · {card.standing}{/if}
        </span>
        {#if card.actions.length > 0}
          <span class="flex flex-wrap gap-snug">
            {#each card.actions as action (actionKey(action))}
              <Button
                label={look.actionLabel(action)}
                name={look.actionName(card, action)}
                tone="quiet"
                onPress={() => {
                  look.onAction(card, action);
                }}
              />
            {/each}
          </span>
        {/if}
      </li>
    {/each}
  </ul>
</div>
