<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import type { FactInk, FactLinkLook } from "./fact_link";
  const look: FactLinkLook = $props();

  const INK: Record<FactInk, string> = {
    text: "text-text",
    quiet: "text-text-quiet",
  };

  const paint = $derived([
    "transition-colors ease-leave hover:text-accent hover:ease-arrive focus-visible:text-accent still:transition-none",
    look.face === "figure" ? "figure" : "",
    INK[look.ink],
  ]);
</script>

{#if "href" in look.wire}
  <a {...look.wire} class={paint}>{look.label}</a>
{:else}
  <button {...look.wire} class={paint}>{look.label}</button>
{/if}
