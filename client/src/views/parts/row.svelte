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
// draw it: `theme.css` owns that judgement under the one density
// attribute the six spacing steps already read, so this file marks the
// line and states no rule about it.

const { primary, secondary, status, actions, onOpen }: RowProps = $props();
</script>

<script module>
import type { Snippet } from "svelte";

export interface RowProps {
  // Already in the person's language, or an identifier.
  readonly primary: string;
  readonly secondary?: string;
  // Usually a Badge: where the thing stands.
  readonly status?: Snippet;
  // Usually Buttons: what can be done to it.
  readonly actions?: Snippet;
  // Present makes the row lead somewhere.
  readonly onOpen?: () => void;
}

export interface RowListProps {
  // The accessible name of the list, already in the person's language.
  readonly label: string;
  // The rows, in the order they are drawn. Each renders an `<li>` -
  // a `Row` here, or a row the page draws for itself.
  readonly rows: Snippet;
}

export { RowList };

// What a keyboard can land on inside a row. A row usually offers one
// control and sometimes three; the walk stops at the first of them, and
// a row offering none - a spacer, a sentinel - is stepped over.
const REACHABLE = 'a[href], button:not([disabled]), [tabindex="0"]';

// A key the control under the pointer is already using is not the
// list's to take: an arrow moves the caret in a box somebody is typing
// in, Home and End reach the ends of that line, and a select opens its
// own menu.
function occupied(target: EventTarget | null): boolean {
  if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) {
    return true;
  }
  if (target instanceof HTMLSelectElement) {
    return true;
  }
  return target instanceof HTMLElement && target.isContentEditable;
}

// Which row an arrow key asks for.
type Step = "next" | "previous" | "first" | "last";

function stepOf(key: string): Step | null {
  switch (key) {
    case "ArrowDown":
      return "next";
    case "ArrowUp":
      return "previous";
    case "Home":
      return "first";
    case "End":
      return "last";
    default:
      return null;
  }
}

// Where a step starts and which way it runs. The ends do not wrap: a
// thousand-row ledger that jumps from the last row to the first has
// moved a person somewhere they cannot see they went.
function courseOf(step: Step, from: number, last: number): readonly [number, number] {
  switch (step) {
    case "next":
      return [from + 1, 1];
    case "previous":
      return [from - 1, -1];
    case "first":
      return [0, 1];
    case "last":
      return [last, -1];
  }
}

function landing(rows: readonly Element[], step: Step, from: number): HTMLElement | null {
  const [start, by] = courseOf(step, from, rows.length - 1);
  for (let at = start; at >= 0 && at < rows.length; at += by) {
    const reach = rows[at]?.querySelector(REACHABLE);
    if (reach instanceof HTMLElement) {
      return reach;
    }
  }
  return null;
}
</script>

{#snippet RowList(props: RowListProps)}
  <!-- The list owns the walk but takes no key of its own: every key
       arrives bubbled from a control inside a row, which is why this
       listener may sit on a list element. -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <ul
    aria-label={props.label}
    onkeydown={(event) => {
      const step = stepOf(event.key);
      if (step === null || occupied(event.target)) {
        return;
      }
      const rows = [...event.currentTarget.children];
      const from = rows.findIndex(
        (row) => event.target instanceof Node && row.contains(event.target),
      );
      if (from < 0) {
        return;
      }
      const next = landing(rows, step, from);
      if (next === null) {
        return;
      }
      // The page must not scroll out from under the row that just
      // took the focus.
      event.preventDefault();
      next.focus();
    }}
  >
    {@render props.rows()}
  </ul>
{/snippet}

{#snippet texts()}
  <span class="truncate text-body text-text">{primary}</span>
  {#if secondary}
    <span class="summary truncate text-note text-text-faint">{secondary}</span>
  {/if}
{/snippet}

<li
  class="flex min-h-[44px] w-full min-w-0 items-center gap-base border-b border-edge px-base py-snug hover:bg-chrome has-[:focus-visible]:bg-chrome"
>
  {#if onOpen}
    {const open = onOpen}
    <button type="button" class="flex min-w-0 flex-1 flex-col text-left" onclick={() => {
        open();
      }}>
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
      {@render texts()}
    </button>
  {:else}
    <div class="flex min-w-0 flex-1 flex-col text-left">
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
      {@render texts()}
    </div>
  {/if}
  {#if status}
    <div class="shrink-0">{@render status()}</div>
  {/if}
  {#if actions}
    <div class="flex shrink-0 items-center gap-tight">{@render actions()}</div>
  {/if}
</li>
