<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import type { Ink, NoteLook } from "./note";
  const { role, pieces }: NoteLook = $props();
  const INK: Readonly<Record<Ink, string>> = {
    faint: "text-text-faint",
    quiet: "text-text-quiet",
    accent: "text-accent",
    alert: "text-alert",
  };
</script>

<div class="flex flex-wrap items-baseline gap-snug text-note text-text-faint" {role}>
  {#each pieces as piece, at (at)}
    {#if piece.kind === "link"}
      <a href={piece.href} class={["name", piece.face === "mono" && "font-mono underline"]}
        >{piece.text}</a
      >
    {:else}
      <span class={INK[piece.ink]}>{piece.text}</span>
    {/if}
  {/each}
</div>

<style>
  .name {
    color: var(--color-text-quiet);
    transition: color var(--transition-duration-short) var(--ease-leave);
  }
  .name:hover {
    color: var(--color-text);
    transition-timing-function: var(--ease-arrive);
  }
</style>
