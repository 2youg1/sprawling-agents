<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<!-- The key that shows a Markdown file's source: a small pill, lifted
while the source is showing, quiet otherwise. The lift and the ink
arrive on the arriving curve and leave on the leaving one. -->
<script lang="ts">
  import type { SourceToggleLook } from "./source_toggle";

  const look: SourceToggleLook = $props();
</script>

<button
  {...look.wire}
  class={[
    "h-control-sm rounded-pill px-snug font-mono text-note transition-colors still:transition-none",
    look.pressed ? "bg-raised text-text ease-arrive" : "text-text-faint ease-leave hover:bg-raised hover:text-text-quiet",
  ]}
>
  {look.label}
</button>
