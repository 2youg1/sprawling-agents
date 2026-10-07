<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How the files a drop did not keep are drawn under the box, and
  // nothing else: each file's line in the alert colour, at note size,
  // one under another (`./drop_refused.ts`, `DropRefusedLook`). Two
  // files of one name are two lines, so a line is keyed by its place.
  import type { DropRefusedLook } from "./drop_refused";

  const look: DropRefusedLook = $props();
</script>

{#each look.lines as line, index (index)}
  <p class="text-note text-alert" {...look.wire}>{line}</p>
{/each}
