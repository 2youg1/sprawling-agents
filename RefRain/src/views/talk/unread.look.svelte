<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import Glyph from "../parts/glyph.svelte";
  import type { UnreadLook } from "./unread";

  const look: UnreadLook = $props();
</script>

<button
  {...look.wire}
  class="inline-flex h-control-sm items-center gap-tight rounded-pill border border-edge-panel bg-raised px-base text-note text-text shadow-float hover:bg-raised-hover"
>
  <Glyph name="chevron" size="sm" class="rotate-90" />
  {look.text}
</button>
