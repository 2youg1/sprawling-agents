<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- How a workbench divider is drawn: the whole gutter is the handle, so
a pointer finds it without aiming at a line, and a hairline down its
middle lights when the pointer is over it, when it has the focus and
while it is being dragged. The line lights with the arriving curve and
fades with the leaving one. The focus ring stands inside the gutter,
because the panes on either side would cover one drawn outside it. -->
<script lang="ts">
  import type { DividerLook } from "./divider";

  const look: DividerLook = $props();
</script>

<div
  {...look.wire}
  class={[
    "relative flex flex-1 cursor-col-resize touch-none justify-center -outline-offset-2",
    "before:w-px before:transition-colors focus-visible:before:bg-accent",
    look.dragging
      ? "before:bg-accent before:ease-arrive"
      : "before:ease-leave hover:before:bg-edge-input hover:before:ease-arrive focus-visible:before:ease-arrive",
  ]}
></div>
