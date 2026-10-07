<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The city's runs board: the lineage tree of `sprawling view` with a
  // time bar on every run, the runs waiting for the person pinned above
  // it and leading the tree. The keys are the terminal's - the line keys
  // of `core/lines.ts` move (j and k, gg and G, the arrows, Home and End),
  // h and l fold - so a person who learned one learned both.
  //
  // **Only the rows on screen are drawn.** A city of a thousand runs is
  // a thousand rows; the list keeps the viewport's rows and a margin,
  // and pads the rest, so the cost of the board is its height.

  import type { BoardRun } from "./lineage";

  export interface RunsBoardProps {
    readonly runs: readonly BoardRun[];
    readonly now: number;
    readonly level?: 1 | 2;
  }
</script>

<script lang="ts">
  // The seat (client D95): it owns the tree's state - which rows are
  // folded, where the cursor stands, which rows are drawn - and the keys
  // that change it, asks `lookOf` (`./board`) for the whole value, and
  // draws whatever `./board.look.svelte` is.
  import { Option } from "effect";
  import { SvelteSet } from "svelte/reactivity";
  import { createAttachmentKey } from "svelte/attachments";

  import { lineWalker } from "../../core/lines";
  import { pressedOf } from "../../core/press";
  import { ui } from "../../ui";
  import { lookOf } from "./board";
  import Look from "./board.look.svelte";
  import type { Row } from "./lineage";
  import { phaseOf, rowsOf, viewOf, windowOf } from "./lineage";

  const { runs, now, level = 1 }: RunsBoardProps = $props();
  const u = ui();
  const { lang } = u;

  // Rows drawn beyond each edge of the viewport, so a fast scroll meets
  // drawn rows rather than padding.
  const MARGIN = 12;

  const folded = new SvelteSet<string>();
  let scrolled = $state(0);
  let viewport = $state(0);
  let rowPx = $state(0);
  let list = $state<HTMLElement | undefined>();

  const rows = $derived(rowsOf(runs, folded));
  const waiting = $derived(runs.filter((run) => phaseOf(run.doing) === "person"));
  const shown = $derived(windowOf(rows.length, scrolled, viewport, rowPx, MARGIN));
  let cursorKey = $state<string | null>(null);
  const cursor = $derived(Math.max(0, rows.findIndex((row) => row.key === (cursorKey ?? waiting[0]?.run))));

  // Measured here rather than bound: a bound height observes the list,
  // and the padding that stands in for undrawn rows resizes it, which
  // is a resize loop the browser reports as an error.
  $effect(() => {
    if (list === undefined || rows.length === 0) return;
    if (rowPx === 0) rowPx = list.querySelector("li")?.getBoundingClientRect().height ?? 0;
    viewport = list.clientHeight;
  });

  function moveTo(at: number): void {
    const row = rows[Math.min(Math.max(at, 0), rows.length - 1)];
    if (row === undefined) return;
    cursorKey = row.key;
    const index = rows.indexOf(row);
    if (list !== undefined && rowPx > 0) {
      const top = index * rowPx;
      if (top < list.scrollTop) list.scrollTop = top;
      else if (top + rowPx > list.scrollTop + viewport) list.scrollTop = top + rowPx - viewport;
    }
  }

  function foldAt(key: string, shut: boolean): void {
    if (shut) folded.add(key);
    else folded.delete(key);
  }

  const walk = lineWalker();

  function pressed(event: KeyboardEvent): void {
    const row = rows[cursor];
    switch (walk(pressedOf(event), event.timeStamp)) {
      case "line.next":
        moveTo(cursor + 1);
        break;
      case "line.previous":
        moveTo(cursor - 1);
        break;
      case "line.first":
        moveTo(0);
        break;
      case "line.last":
        moveTo(rows.length - 1);
        break;
      case "line.open":
        if (row !== undefined) open(row);
        break;
      case "line.close":
      case null:
        folding(event, row);
        return;
    }
    event.preventDefault();
  }

  // h and l, and the side arrows, fold and unfold the row under the
  // cursor: the tree's own keys, not a list's.
  function folding(event: KeyboardEvent, row: Row | undefined): void {
    const openable = row !== undefined && row.kind !== "run";
    switch (event.key) {
      case "h":
      case "ArrowLeft":
        if (openable) foldAt(row.key, true);
        break;
      case "l":
      case "ArrowRight":
        if (openable) foldAt(row.key, false);
        break;
      default:
        return;
    }
    event.preventDefault();
  }

  // A row opens the page of what it names (S07 E36).
  function open(row: Row): void {
    cursorKey = row.key;
    Option.map(viewOf(row), u.go);
  }

  function pick(run: BoardRun): void {
    folded.delete((run.addr ?? "").split("/")[0] ?? "");
    folded.delete(run.addr ?? "");
    cursorKey = run.run;
    list?.focus();
  }

  const hands = {
    tree: {
      onkeydown: pressed,
      onscroll: (event: Event & { readonly currentTarget: EventTarget & HTMLElement }) => {
        scrolled = event.currentTarget.scrollTop;
        viewport = event.currentTarget.clientHeight;
      },
      [createAttachmentKey()]: (node: HTMLElement) => {
        list = node;
        return () => {
          if (list === node) list = undefined;
        };
      },
    },
    open,
    pick,
  };

  const look = $derived(lookOf({ runs, now, lang: $lang, level, rows, folded, cursor, shown, rowPx }, hands));
</script>

<Look {...look} />
