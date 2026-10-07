<script lang="ts">
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One line of a list: what the thing is called, what else is worth
// knowing about it, where it stands, and what can be done to it - and
// the list itself, which is what a keyboard walks.
//
// **The two parts are a row's own, and the row is the list's.** `Row`
// draws one `<li>` - a main text, an optional second line, and
// whatever the page puts at the right-hand end - because a `<ul>` whose
// children are `<div>`s tells a screen reader it holds a list and not
// how many items are in it. The list a keyboard walks is the `RowList`
// snippet, exported from the module script, and a page that draws rows
// without one gets a list nobody can step through with arrows, which is
// a page's decision and not this file's. A caller writes
// `import Row, { RowList } from ".../row.svelte"` and renders a row
// with `{@render Row({ ... })}` beside rows of its own, because a list
// may hold rows this file does not draw (a ledger line, a sentinel).
//
// When the row leads somewhere, the two texts are the button that goes
// there, so the keyboard reaches the same place the pointer does and the
// actions on the right stay separately reachable. The whole row lights
// up while anything inside it holds the focus ring, because a ring
// around one word in a wide row is a position a person has to hunt for.
//
// **44 pixels is the touch floor, not the height.** A row with two
// lines of text is taller than its floor and keeps its second line;
// the floor is what a finger lands on when the second line is one the
// compact page does not draw.
//
// The second text is the row's summary, and a compact page does not
// draw it: `theme/preference.css` owns that judgement under the one density
// attribute the six spacing steps already read, so the look marks the
// line and states no rule about it.
//
// **This file is the seat.** It owns the walk and the wiring - which
// keys the list takes (`./row`), where focus lands, what a screen
// reader is told - and draws a row through `./row.look.svelte`, which
// holds every class a row is painted with. The list itself is a bare
// `<ul>` here: it carries a name and a key handler and nothing a
// person sees.

import Look from "./row.look.svelte";
import { lookOf } from "./row";
import type { RowProps } from "./row";

const props: RowProps = $props();
const look = $derived(lookOf(props));
</script>

<script module>
import { landing, taken } from "./row";
import type { RowListProps, Target } from "./row";

export { RowList };

// What a keyboard can land on inside a row. A row usually offers one
// control and sometimes three; the walk stops at the first of them.
const REACHABLE = 'a[href], button:not([disabled]), [tabindex="0"]';

function targetOf(target: EventTarget | null): Target {
  if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) {
    return "text";
  }
  if (target instanceof HTMLSelectElement) {
    return "select";
  }
  return target instanceof HTMLElement && target.isContentEditable ? "editable" : "row";
}

function walk(event: KeyboardEvent & { readonly currentTarget: EventTarget & HTMLUListElement }): void {
  const step = taken(targetOf(event.target), event.key);
  if (step === null) {
    return;
  }
  const rows = [...event.currentTarget.children];
  const from = rows.findIndex((row) => event.target instanceof Node && row.contains(event.target));
  if (from < 0) {
    return;
  }
  const next = landing(
    rows.map((row) => {
      const reach = row.querySelector(REACHABLE);
      return reach instanceof HTMLElement ? reach : undefined;
    }),
    step,
    from,
  );
  if (next === undefined) {
    return;
  }
  // The page must not scroll out from under the row that just took
  // the focus.
  event.preventDefault();
  next.focus();
}
</script>

{#snippet RowList(props: RowListProps)}
  <!-- The list owns the walk but takes no key of its own: every key
       arrives bubbled from a control inside a row, which is why this
       listener may sit on a list element. -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <ul aria-label={props.label} onkeydown={walk}>
    {@render props.rows()}
  </ul>
{/snippet}

<Look {...look} />
