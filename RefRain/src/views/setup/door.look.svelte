<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import type { DoorLook } from "./door";

  const { wire, label }: DoorLook = $props();
</script>

<!-- A door stands on a card as often as on the page, and the card's fill
is `raised`, the same step as `edge`, so the door takes the edge a text
box has (`edge-input`) to be seen there, and lifts to `raised-hover`
under the pointer. -->
<a
  {...wire}
  class="inline-flex h-control w-fit items-center rounded-control border border-edge-input bg-raised px-base text-label text-text transition-colors duration-short ease-leave hover:bg-raised-hover hover:ease-arrive"
>
  {label}
</a>
