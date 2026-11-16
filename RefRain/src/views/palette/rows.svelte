<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The palette's list: places, or - once the line begins with `/` -
  // the verbs under the section each one names. The box owns the line,
  // the cursor and what a pick does; this is the seat that hands
  // `./rows.ts`'s value to whatever `./rows.look.svelte` is, and reports
  // the pointer.
  import { ui } from "../../ui";
  import type { Entry, Listing } from "./entry";
  import { rowsLookOf } from "./rows";
  import Look from "./rows.look.svelte";

  interface Props {
    readonly listing: Listing;
    // The flat list the cursor walks, in the order the rows are drawn.
    readonly shown: readonly Entry[];
    readonly cursor: number;
    readonly onHover: (at: number) => void;
    readonly onPick: (entry: Entry) => void;
    // The listbox's id, which the box's `aria-controls` and
    // `aria-activedescendant` name; one of its own when the caller
    // names none.
    readonly id?: string | undefined;
  }

  const { listing, shown, cursor, onHover, onPick, id }: Props = $props();
  const own = $props.id();
  const { lang } = ui();

  const look = $derived(
    rowsLookOf(
      { listing, shown, cursor, id: id ?? own, lang: $lang },
      {
        hover: (at) => {
          onHover(at);
        },
        pick: (entry) => {
          onPick(entry);
        },
      },
    ),
  );
</script>

<Look {...look} />
