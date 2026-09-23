<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A strip across the top of a page for the condition the whole page is
// under: the city is stopped, the link is down, somebody is waiting to
// be answered. It is not a field's error and not a toast - it belongs to
// the page, it stays while the condition lasts, and it carries the one
// action that ends the condition.
//
// The strip drops in from the edge it belongs to, and stays still for a
// machine that asks for less motion.

import type { Snippet } from "svelte";

// A condition a person should know about, against one they must act on.
// This two-step scale is the banner's own; it is not the three paint
// tiers of `glyph.ts`.
export type Weight = "notice" | "alert";

export interface BannerProps {
  // Already in the person's language.
  readonly text: string;
  readonly detail?: string;
  readonly weight?: Weight;
  // Usually a Button: the way out of the condition.
  readonly action?: Snippet;
}
</script>

<script lang="ts">
  const { text, detail, weight = "notice", action }: BannerProps = $props();

  const loud = $derived(weight === "alert");
</script>

<!-- The condition and the one control that ends it. The role is the
whole contract: a page-level alert announces itself the moment it
appears, a notice waits its turn. -->
<div
  role={loud ? "alert" : "status"}
  class={[
    "drop flex w-full min-w-0 items-center justify-between gap-base border-b px-pane py-snug text-note",
    loud ? "border-alert/50 bg-raised text-alert" : "border-edge-panel bg-raised text-text-quiet",
  ]}
>
  <div class="flex min-w-0 items-baseline gap-snug">
    <span class="truncate">{text}</span>
    {#if detail !== undefined}
      <span class="truncate text-text-faint">{detail}</span>
    {/if}
  </div>
  {#if action !== undefined}
    <div class="shrink-0">{@render action()}</div>
  {/if}
</div>
