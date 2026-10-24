<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // How the address line is drawn (`./address_line.ts`): one quiet line
  // at the head of the conversation, the building's part in the faint ink
  // and free to shrink, the room in the quiet ink and never cut, the
  // session's number after it on the figure face. The building gives way
  // a thousand times faster than the room, so the room is cut only once
  // the building is gone, on a column too narrow for the room alone; the
  // number is never cut. The line is a button
  // with no ground: under the pointer and the focus the ink lifts one step,
  // which arrives and leaves on the theme's two curves (§4-43), and a
  // press gives by the theme's press ratio. On a touch screen the area
  // that takes the touch reaches `touch` while the drawn line keeps its
  // size (docs/frontend-method.md §4-34).
  import type { AddressLineLook } from "./address_line";

  const look: AddressLineLook = $props();
</script>

<header aria-label={look.head} class="fade flex h-control shrink-0 items-center">
  <button
    {...look.wire}
    class={[
      "group relative flex max-w-full min-w-0 items-baseline rounded-control text-note text-text-faint",
      "transition-[color,transform] duration-short ease-leave hover:text-text-quiet hover:ease-arrive focus-visible:text-text-quiet",
      "active:scale-(--scale-press) active:ease-arrive still:transition-none still:active:scale-100",
      "pointer-coarse:before:absolute pointer-coarse:before:top-1/2 pointer-coarse:before:left-1/2 pointer-coarse:before:size-touch pointer-coarse:before:-translate-1/2 pointer-coarse:before:content-['']",
    ]}
  >
    {#if look.building !== ""}
      <span class="min-w-0 shrink-1000 truncate">{look.building}</span>
    {/if}
    <span class="min-w-0 truncate text-text-quiet group-hover:text-text">{look.room}</span>
    {#if look.session !== undefined}
      <!-- wording-ok: a separator glyph, hidden from screen readers, whose name the button's label already says -->
      <span class="shrink-0 px-tight" aria-hidden="true">·</span>
      <span class="figure shrink-0">{look.session}</span>
    {/if}
  </button>
</header>
