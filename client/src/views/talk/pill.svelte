<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- One compact control under the box: which model answers, which room
hears it, and how hard the model thinks.
`composer.ts` owns what a pill offers and what a pick means; `pill.ts`
owns what the menu's keys do and builds the value `pill.look.svelte`
draws. This file is the seat (client D95): it holds whether the menu is
open, the typed filter, the cursor and the drawn elements, and draws
whatever the look is. It writes no class. -->
<script lang="ts">
  import { untrack } from "svelte";
  import type { Snippet } from "svelte";
  import type { Attachment } from "svelte/attachments";

  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Pill } from "./composer";
  import { edgeFor, filtered, lookOf, rowId } from "./pill";
  import type { Edge, PillLook } from "./pill";
  import Look from "./pill.look.svelte";

  interface Props {
    readonly spec: Pill;
    // The gallery draws a menu open so it is measured; every screen
    // starts closed. A menu drawn open takes no focus until somebody
    // opens it themselves: the gallery holds several open at once, and
    // each one taking focus would close the one before it on blur,
    // leaving every case but the last an empty frame.
    readonly starts?: "open" | "closed";
    // What the menu reads out above its choices, when the fact the pill
    // stands for has more to it than its value: the room chip's who is
    // listening (docs/frontend-method.md §7I). The list is described by it, so a
    // screen reader hears it with the list.
    readonly told?: Snippet | undefined;
  }

  const { spec, starts = "closed", told }: Props = $props();

  const { lang } = ui();
  const uid = $props.id();

  let open = $state(untrack(() => starts) === "open");
  let drawnOpen = untrack(() => starts) === "open";
  let query = $state("");
  let at = $state(0);
  let edge = $state<Edge>("left");

  // The drawn elements, which no draw reads: a plain record, so the
  // attachments that fill it never write during a derivation.
  const drawn: Record<"frame" | "trigger" | "filter" | "list", HTMLElement | undefined> = {
    frame: undefined,
    trigger: undefined,
    filter: undefined,
    list: undefined,
  };
  const keep = (slot: "frame" | "trigger" | "filter" | "list"): Attachment<HTMLElement> => (node) => {
    drawn[slot] = node;
    return () => {
      if (drawn[slot] === node) drawn[slot] = undefined;
    };
  };

  // The menu hangs from the trigger's left edge unless its own width
  // would then run past the window's right edge, measured once it is
  // drawn rather than guessed from the widest a menu may be.
  const measure: Attachment<HTMLElement> = (node) => {
    const left = drawn.trigger?.getBoundingClientRect().left;
    if (left !== undefined) edge = edgeFor(left, node.getBoundingClientRect().width, window.innerWidth);
  };

  const hold = { frame: keep("frame"), trigger: keep("trigger"), box: measure, filter: keep("filter"), list: keep("list") };

  // The row under the cursor scrolled into view, after the draw that
  // gave it its id.
  function reveal(): void {
    queueMicrotask(() => {
      document.getElementById(rowId(uid, at))?.scrollIntoView({ block: "nearest" });
    });
  }

  // Opening puts the cursor on the value in force and shows it, then
  // gives the focus to the filter or the list.
  $effect(() => {
    if (!open) return;
    at = Math.max(0, spec.choices.findIndex((each) => each.value === untrack(() => spec.value)));
    reveal();
    if (drawnOpen) {
      drawnOpen = false;
      return;
    }
    (filtered(spec) ? drawn.filter : drawn.list)?.focus({ preventScroll: true });
  });

  const look: PillLook = $derived(
    lookOf(
      { spec, uid, told, empty: say($lang, "part_no_match") },
      { open, query, at, edge },
      {
        open: () => {
          edge = "left";
          open = true;
        },
        close: (focus) => {
          open = false;
          query = "";
          if (focus === "opener") drawn.trigger?.focus();
        },
        point: (to) => {
          at = to;
          reveal();
        },
        hover: (to) => {
          at = to;
        },
        query: (text) => {
          query = text;
          at = 0;
        },
        inside: (target) => target instanceof Node && drawn.frame?.contains(target) === true,
        hold,
      },
    ),
  );
</script>

<Look {...look} />
