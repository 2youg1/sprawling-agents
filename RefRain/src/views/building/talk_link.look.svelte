<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<!-- The way into a room's conversation, worn as a secondary control: the
raised surface at a control's height, lifting one step under the
pointer, the lift arriving on the arriving curve and leaving on the
leaving one. -->
<script lang="ts">
  import type { TalkLinkLook } from "./talk_link";

  const look: TalkLinkLook = $props();
</script>

<a
  {...look.wire}
  class="inline-flex h-control items-center rounded-control bg-raised px-base text-label text-text transition-colors ease-leave hover:bg-raised-hover hover:ease-arrive still:transition-none"
>
  {look.label}
</a>
