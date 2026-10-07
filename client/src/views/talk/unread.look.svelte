<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- The count of what arrived below, floating at the foot of the column
it is about: a raised pill with the way down drawn in front of the count.
Its place is the scroller's; the pill only draws itself. -->
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
