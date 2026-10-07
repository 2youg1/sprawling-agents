<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The badge in each of its three forms: a status, a tier with and
  // without its dot, and the count pinned to a key's corner - a number,
  // the cap reached, and the lone dot for something new without a
  // number.
  import { say } from "../../../core/lang";
  import { ui } from "../../../ui";
  import Badge from "../../parts/badge.svelte";
  import Glyph from "../../parts/glyph.svelte";
  import Case from "../case.svelte";

  const { lang } = ui();
</script>

<Case label="badge · status, tier and dot">
  <div class="flex flex-wrap items-center gap-snug">
    <Badge text={say($lang, "status_in_progress")} status="live" />
    <Badge text={say($lang, "mcp_state_failed")} status="refused" />
    <Badge text={say($lang, "mcp_state_connected")} weight="live" dot />
    <Badge text={say($lang, "city_quiet")} dot />
    <Badge text={say($lang, "setup_local")} />
  </div>
</Case>

<Case label="badge · counts on a key's corner: a few, past the cap, and new without a number">
  <div class="flex flex-wrap items-center gap-wide">
    {#each [3, 120] as count (count)}
      <span class="relative inline-flex size-key items-center justify-center rounded-key bg-raised text-text-quiet">
        <Glyph name="inbox" size="key" />
        <span class="absolute -top-tight -right-tight"><Badge {count} /></span>
      </span>
    {/each}
    <span class="relative inline-flex size-key items-center justify-center rounded-key bg-raised text-text-quiet">
      <Glyph name="inbox" size="key" />
      <span class="absolute top-tight right-tight flex"><Badge count="fresh" weight="quiet" /></span>
    </span>
  </div>
</Case>
