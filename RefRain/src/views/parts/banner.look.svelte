<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // How the page banner is drawn (`./banner`, `BannerLook`): the
  // condition and the one control that ends it. The strip drops in from
  // the edge it belongs to, and stays still for a machine that asks for
  // less motion.
  import type { Weight } from "./banner";

  const PAINT: Record<Weight, string> = {
    notice: "border-edge-panel text-text-quiet",
    alert: "border-alert/50 text-alert",
  };
</script>

<script lang="ts">
  import type { BannerLook } from "./banner";

  const look: BannerLook = $props();
</script>

<div
  {...look.strip}
  class={[
    "drop flex w-full min-w-0 items-center justify-between gap-base border-b bg-raised px-pane py-snug text-note",
    PAINT[look.weight],
  ]}
>
  <!-- The condition wraps rather than truncating, because on a narrow
  screen a cut sentence hides the very thing the strip is there to say. -->
  <div class="flex min-w-0 flex-wrap items-baseline gap-x-snug wrap-anywhere">
    <span>{look.text}</span>
    {#if look.detail !== undefined}
      <span class="text-text-faint">{look.detail}</span>
    {/if}
  </div>
  {#if look.action !== undefined}
    <div class="shrink-0">{@render look.action()}</div>
  {/if}
</div>
