<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<!-- A box of several lines, on the surface and edge every input wears,
set in the code face because what it holds is a file or a list of
addresses and names. A refused box takes the alert edge. -->
<script lang="ts">
  import type { TextBoxLook, TextBoxSize } from "./text_box";

  const look: TextBoxLook = $props();

  const ROOM: Record<TextBoxSize, string> = {
    document: "min-h-palette leading-relaxed",
    list: "min-h-[calc(var(--spacing-control)*3)]",
  };
</script>

<textarea
  {...look.wire}
  class="w-full rounded-control border border-edge-input bg-raised px-base py-snug font-mono text-note text-text aria-invalid:border-alert {ROOM[
    look.size
  ]}"
></textarea>
