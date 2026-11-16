<script lang="ts" generics="V extends string">
  // This Source Code Form is subject to the terms of the Mozilla Public
  // License, v. 2.0. If a copy of the MPL was not distributed with this
  // file, You can obtain one at https://mozilla.org/MPL/2.0/.
  // Copyright (c) 2026 2youg1 and the sprawling contributors

  // One exclusive choice among a few, drawn as a track with a slider on
  // it. It replaces the pill rows the settings pages grew one at a time,
  // each of which announced itself as a row of unrelated pressed buttons.
  //
  // Three things this control owes a person that a row of pills does not
  // give them. A screen reader hears one question with several answers,
  // because the track is a `radiogroup` and every cell reports whether it
  // is the chosen one. A keyboard crosses the whole control in two keys,
  // because only the chosen cell is a tab stop and the arrows move
  // between cells. And a cell that cannot be chosen says why - through
  // `Tip`, so the reason reaches a pointer, a keyboard and a touch screen
  // alike, and arrives before the click rather than as a refusal after
  // it.
  //
  // This file is the seat (client D95): the one callers import, with the
  // props they have always passed. It owns the drawn cells and the focus
  // that moves between them, asks `lookOf` (`./segmented`) for the whole
  // value - words, state, and the wire bags that carry every role, key
  // and `aria-*` value of client/Spec.lean §7-4 - and draws whatever
  // `./segmented.look.svelte` is. The words are the caller's: this file
  // holds no prose and no class.
  import type { Attachment } from "svelte/attachments";

  import { lookOf } from "./segmented";
  import type { SegmentedLook, SegmentedProps } from "./segmented";
  import Look from "./segmented.look.svelte";

  const props: SegmentedProps<V> = $props();
  const uid = $props.id();

  // Two registries no draw reads, so plain records rather than reactive
  // maps: a reactive map read inside the derived look and written by
  // the attachment it hands out would be a write during a derivation.
  // Keyed by value, so an element stays with its cell when the caller
  // edits the list rather than with the position it happened to hold.
  const drawn: Record<string, HTMLElement | undefined> = {};
  const holds: Record<string, Attachment<HTMLElement> | undefined> = {};

  const hold = (value: V): Attachment<HTMLElement> => {
    const kept = holds[value];
    if (kept !== undefined) return kept;
    const made: Attachment<HTMLElement> = (node) => {
      drawn[value] = node;
      return () => {
        if (drawn[value] === node) drawn[value] = undefined;
      };
    };
    holds[value] = made;
    return made;
  };

  const look: SegmentedLook = $derived(
    lookOf(props, uid, {
      focus: (value) => drawn[value]?.focus(),
      hold,
    }),
  );
</script>

<Look {...look} />
