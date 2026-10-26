<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // How the address line is drawn (`./address_line.ts`): the address
  // inside the box's placeholder, or one quiet line at the head of a past
  // session (client/Spec.lean 4-10a), the building's part in
  // the faint ink and free to shrink, the room in the quiet ink and never
  // cut, the session's number after it on the figure face. The building
  // gives way a thousand times faster than the room, so the room is cut
  // only once the building is gone, on a column too narrow for the room
  // alone; the number is never cut. The line is a button
  // with no ground: under the pointer and the focus the ink lifts one step,
  // which arrives and leaves on the theme's two curves (§4-43), and a
  // press gives by the theme's press ratio. On a touch screen the area
  // that takes the touch reaches `touch` while the drawn line keeps its
  // size (docs/frontend-method.md §4-34).
  //
  // Until this browser has opened the palette, the line ends in what a
  // press on it does, and on a screen with a fine pointer the chord that
  // does the same (client/Spec.lean 4-10a). The lesson is for the eye:
  // the button's name already says it. It is one run of text, so it is
  // the first thing a narrow column cuts, and it is cut with an ellipsis
  // rather than by pushing a key glyph out of the line.
  //
  // In the box the line takes the placeholder's size and stands in its
  // words, which, like the lesson after them, are for the eye: the box's
  // own name says them, and a press on them falls through to the box.
  import type { Snippet } from "svelte";
  import type { AddressLineLook, BoxWords, Lesson } from "./address_line";

  // Where the line stands, with the placeholder's words when that is the box.
  type Seat = { readonly at: "head" } | ({ readonly at: "box" } & BoxWords);

  interface Props extends AddressLineLook {
    readonly seat: Seat;
    readonly lesson?: Lesson | undefined;
  }

  const { seat, lesson, ...look }: Props = $props();

  // The checker types a `{#snippet}` name as a void call, which the lint
  // lane rejects inside a render tag; each is taken again as its type.
  const lessonOf: Snippet<[Lesson | undefined]> = taught;
  const button: Snippet<[string, Lesson | undefined]> = press;
</script>

{#snippet taught(lesson: Lesson | undefined)}
  {#if lesson !== undefined}
    <!-- The gap before the lesson is a space inside it, not a margin, so a column that cuts the lesson to nothing keeps no gap. -->
    <span class="min-w-0 shrink-10000 truncate text-text-faint" aria-hidden="true"
      >&emsp;{lesson.words}{#if lesson.chord !== ""}<span class="pointer-coarse:hidden"
          >&ensp;{lesson.chord}</span
        >{/if}</span
    >
  {/if}
{/snippet}

{#snippet press(size: string, within: Lesson | undefined)}
  <button
    {...look.wire}
    class={[
      "group relative flex max-w-full min-w-0 items-baseline rounded-control text-text-faint",
      size,
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
    {@render lessonOf(within)}
  </button>
{/snippet}

{#if seat.at === "head"}
  <header aria-label={look.head} class="fade flex h-control shrink-0 items-center">
    {@render button("text-note", lesson)}
  </header>
{:else}
  <span class="flex max-w-full min-w-0 items-baseline">
    <span class="shrink-0 whitespace-pre" aria-hidden="true">{seat.before}</span>
    <span class="pointer-events-auto flex min-w-0">{@render button("", undefined)}</span>
    <span class="shrink-0 whitespace-pre" aria-hidden="true">{seat.after}</span>
    {@render lessonOf(lesson)}
  </span>
{/if}
