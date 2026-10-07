<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- The fold over the earlier stretch: quiet words that take the row's
wash under the pointer, the way every press inside the thread does. -->
<script lang="ts">
  import type { FoldLook } from "./fold";

  const look: FoldLook = $props();
</script>

<button {...look.wire} class="rounded-control px-tight hover:wash hover:text-text-quiet">{look.label}</button>
