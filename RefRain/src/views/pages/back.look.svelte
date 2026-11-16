<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import Glyph from "../parts/glyph.svelte";
  import type { BackLook } from "./back";

  const look: BackLook = $props();
</script>

<a
  {...look.wire}
  class="flex h-control-sm w-fit items-center gap-tight rounded-control pr-snug text-note text-text-faint transition-colors ease-leave hover:bg-raised hover:text-text hover:ease-arrive"
>
  <Glyph name="chevron" size="sm" class="rotate-180" />
  {look.label}
</a>
