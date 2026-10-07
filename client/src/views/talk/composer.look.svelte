<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts">
  // How the composer is laid out, and nothing else (docs/frontend-method.md
  // §7I): a page, not a card - the words on top, a one-pixel line under
  // them, the settings under the line, with no box, no ground and no
  // shadow. Focus is said by the line coming to full strength, and a drag
  // over the box by the wash it takes. Every handler, name and state
  // arrives in `ComposerLook` (`./composer.ts`); the parts around the
  // words arrive seated. `field-sizing: content` grows the box with the
  // words where the engine has it; the seat grows it by hand where it
  // does not.
  import type { ComposerLook } from "./composer";

  const look: ComposerLook = $props();
</script>

<form
  class={["relative rounded-control transition-colors", look.over ? "wash ease-arrive" : "ease-leave"]}
  {...look.form}
>
  {#if look.menu !== undefined}{@render look.menu()}{/if}
  {#if look.band !== undefined}{@render look.band()}{/if}
  <div class="relative pb-snug">
    <div class="flex min-h-key items-end gap-base">
      <textarea
        class="block max-h-output min-h-key min-w-0 flex-1 resize-none overflow-y-auto bg-transparent py-snug text-body leading-relaxed text-text caret-accent outline-hidden field-sizing-content placeholder:text-text-faint"
        {...look.box}
      ></textarea>
      {@render look.keys()}
    </div>
    {@render look.line()}
  </div>
  {@render look.under()}
</form>
