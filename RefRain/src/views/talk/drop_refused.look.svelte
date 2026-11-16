<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import type { DropRefusedLook } from "./drop_refused";

  const look: DropRefusedLook = $props();
</script>

{#each look.lines as line, index (index)}
  <p class="text-note text-alert" {...look.wire}>{line}</p>
{/each}
