<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import Copy from "./copy.svelte";
  import type { UnkeptLook } from "./unkept";

  const look: UnkeptLook = $props();
</script>

<p class="flex items-center gap-base text-note text-alert" {...look.line}>
  <span class="min-w-0 flex-1">{look.sentence}</span>
  <Copy {...look.copy} />
</p>
