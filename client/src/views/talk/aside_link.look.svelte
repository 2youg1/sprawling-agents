<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- A link inside a sentence of the thread. It keeps the ink of the words
around it and is underlined at rest, so it reads as a link without
relying on colour; under the pointer it rises to the text ink. It takes
no wash, because it sits in a sentence rather than standing as a press. -->
<script lang="ts">
  import type { AsideLinkLook } from "./aside_link";

  const look: AsideLinkLook = $props();
</script>

<a
  href={look.href}
  class="underline decoration-from-font underline-offset-2 transition-colors ease-leave hover:text-text hover:ease-arrive"
>{look.text}</a>
