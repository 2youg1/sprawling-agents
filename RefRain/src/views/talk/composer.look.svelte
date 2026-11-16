<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts">
  import type { ComposerLook } from "./composer";

  const look: ComposerLook = $props();
</script>

<form
  class={[
    "relative rounded-control transition-colors",
    look.over ? "wash ease-arrive" : "ease-leave",
  ]}
  {...look.form}
>
  {#if look.menu !== undefined}{@render look.menu()}{/if}
  {#if look.band !== undefined}{@render look.band()}{/if}
  <div class="relative pb-snug">
    <div class="flex min-h-key items-end gap-base">
      <div class="relative min-w-0 flex-1">
        {#if look.door !== undefined}
          <div
            class="pointer-events-none absolute inset-x-0 top-0 z-(--layer-raised) flex py-snug text-body leading-read text-text-faint"
          >
            {@render look.door()}
          </div>
        {/if}
        <textarea
          class="block max-h-output min-h-key w-full resize-none overflow-y-auto bg-transparent py-snug text-body leading-read text-text caret-accent outline-hidden field-sizing-content placeholder:text-text-faint"
          {...look.box}></textarea>
      </div>
      {@render look.keys()}
    </div>
    {@render look.line()}
  </div>
  {@render look.under()}
</form>
