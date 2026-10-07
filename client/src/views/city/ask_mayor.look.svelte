<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // How the way to the Mayor is drawn, and nothing else: a link in the
  // primary button's paint, because it is the one thing an empty page is
  // for. Its fill arrives under the pointer and leaves after it on the
  // theme's two curves (docs/frontend-method.md §4-43).
  import type { AskMayorLook } from "./ask_mayor";

  const look: AskMayorLook = $props();
</script>

<a
  {...look.wire}
  class="inline-flex h-control items-center rounded-control bg-accent px-base text-label text-on-accent transition-colors ease-leave hover:bg-accent-hover hover:ease-arrive"
>
  {look.label}
</a>
