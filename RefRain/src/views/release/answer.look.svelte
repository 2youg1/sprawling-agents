<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import Copy from "../parts/copy.svelte";
  import type { AnswerLook, Ink } from "./answer";

  const look: AnswerLook = $props();

  const INK: Record<Ink, string> = {
    text: "text-text",
    quiet: "text-text-quiet",
    accent: "text-accent",
    alert: "text-alert",
  };
</script>

<div class="flex min-w-0 flex-col gap-snug">
  {#if look.refused !== undefined}
    <p class="text-note text-alert">
      {look.refused.text}
      <code class="font-mono text-note text-text-quiet">{look.refused.recovery}</code>
    </p>
  {/if}
  {#each look.statements as statement (statement.text)}
    <p class="text-note {INK[statement.ink]}">{statement.text}</p>
  {/each}
  {#if look.registries.length > 0}
    <dl class="registries grid gap-x-base gap-y-tight text-note">
      {#each look.registries as line (line.name)}
        <dt class="text-text-faint">{line.name}</dt>
        <dd class="min-w-0 break-words">
          {#if line.newest !== undefined}
            <span class="font-mono text-text">{line.newest.version}</span>
            <span class="text-text-faint">{line.newest.released}</span>
          {/if}
          {#if line.reason !== undefined}
            <span class="text-text-quiet">{line.reason.text}</span>
            {#if line.reason.said !== undefined}
              <span class="font-mono text-text-faint">{line.reason.said}</span>
            {/if}
          {/if}
        </dd>
      {/each}
    </dl>
  {/if}
  {#if look.update !== undefined}
    {@const update = look.update}
    <div class="flex min-w-0 flex-col gap-tight">
      <span class="text-note text-text-quiet">{update.channel}</span>
      {#if update.choice !== undefined}
        <select
          class="h-control w-fit max-w-full min-w-0 rounded-control bg-raised px-snug text-note text-text transition-colors ease-leave hover:bg-raised-hover hover:ease-arrive"
          {...update.choice.wire}
        >
          <option value="">{update.choice.unchosen}</option>
          {#each update.choice.commands as command (command)}<option value={command}
              >{command}</option
            >{/each}
        </select>
      {/if}
      {#if update.command !== undefined}
        <div class="flex min-w-0 items-start gap-tight">
          <code
            class="min-w-0 flex-1 break-words rounded-control bg-chrome px-snug py-tight font-mono text-note text-text"
            >{update.command}</code
          >
          <Copy text={update.command} />
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .registries {
    grid-template-columns: minmax(0, 8rem) minmax(0, 1fr);
  }
</style>
