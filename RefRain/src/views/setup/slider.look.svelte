<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import type { SliderLook } from "./slider";

  const { wire, shown }: SliderLook = $props();
</script>

<!-- The track takes the accent, the colour the page reserves for what a
person moves; the figure keeps a fixed width so the track does not shift
as the number grows a digit. -->
<div class="flex h-control items-center gap-base">
  <input {...wire} class="min-w-0 flex-1 accent-accent" />
  <span class="figure w-figure text-right text-body text-text">{shown}</span>
</div>
