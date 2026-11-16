<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import { Popover } from "../parts/popover";
  import type { PopoverLook } from "../parts/popover_wiring";
  import Look from "./picker.look.svelte";
  import type { PickerMenu } from "./picker_look";

  interface Props {
    readonly menu: PickerMenu;
  }

  const { menu }: Props = $props();
</script>

{#snippet drawn(popover: PopoverLook)}
  <Look look={popover} {menu} />
{/snippet}

<Popover
  label="picker_title"
  columns={menu.columns}
  onApply={menu.onApply}
  onClose={menu.onClose}
  bind={menu.bind}
  onCursorChange={menu.onCursorChange}
  look={drawn}
/>
