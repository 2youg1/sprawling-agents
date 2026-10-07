<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How an unanswered question is drawn (`./unanswered`,
  // `UnansweredLook`): the sentence, the city's reason in its own
  // words, and the one recovery.
  import Button from "./button.svelte";
  import type { UnansweredLook } from "./unanswered";

  const look: UnansweredLook = $props();
</script>

<div class="flex flex-col items-start gap-snug text-note">
  <p class="text-text-faint">{look.said}</p>
  {#if look.reason !== undefined}
    <p class="font-mono text-text-faint wrap-anywhere">{look.reason}</p>
  {/if}
  <Button label={look.retry.label} tone="quiet" onPress={look.retry.press} />
</div>
