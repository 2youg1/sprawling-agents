<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The third section of the mailbox: the sessions of every room this
  // page knows, newest activity first (client/Spec.lean §4-49). Each room is
  // asked once (`Query::Sessions`, wire §8-71) and the answers are
  // merged by the time of each session's last line. A row is a link to
  // the room, and beside it, always visible rather than on hover, the
  // way to fork that session from its last turn: the same frame `/fork`
  // sends, with the session's last run as the mother (4-14 - the
  // continuation is a fork, not a resume verb).
  //
  // **Only the rows in view are mounted**: the list keeps its whole
  // height and draws the rows the column shows plus `OVERSCAN` on each
  // side, so a city of a thousand sessions costs the paint of a screen.
  // The row height is the `control` token, read from the stylesheet.
  //
  // **The search box filters what the page fetched, not what it drew**
  // (refrain 3-14 row 9): every session of every room asked is matched
  // by its room and by the task of each run of it this page holds, so a
  // row the virtual list has not mounted is still found. A room this
  // page never saw a run in was never asked, and the line under the box
  // says so rather than let an empty result read as "no such session".
  import { openSession } from "../../core/commands";
  import { fill, say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { tailOf } from "../../core/stretches";
  import { ago } from "../../core/time";
  import { ui } from "../../ui";
  import type { Address, Origin, SessionLine } from "../../wire";
  import { OVERSCAN, windowOf } from "../run/lanes";
  import { askStretches } from "../world/stretches.svelte";
  import { rowOf } from "./recent_row";
  import Row from "./recent_row.look.svelte";
  import Section from "./section.svelte";

  interface Props {
    // The column that scrolls, whose view decides which rows are drawn;
    // a getter, because the column is bound after this section mounts.
    readonly scroller: () => HTMLElement | undefined;
    // Following a row or forking leaves the mailbox for the room.
    readonly onLeave: () => void;
  }

  const { scroller, onLeave }: Props = $props();

  const u = ui();
  const { lang } = u;
  const uid = $props.id();

  interface Session {
    readonly room: Address;
    readonly line: SessionLine;
    // Where a fork of this session would cut: its last run's tail, or
    // nothing when that run is older than the runs this page holds.
    readonly origin: Origin | null;
    // What the search box matches: the room, then the task of each run
    // of this session the page holds, folded to one case.
    readonly text: string;
  }

  // Every session of every room this page knows, the one reading the
  // sessions pane takes too (`world/stretches.svelte.ts`).
  const stretches = askStretches(u);
  let search = $state("");
  const rows = $derived.by((): Session[] => {
    const needle = search.trim().toLowerCase();
    return stretches.all
      .map((stretch): Session => ({
        room: stretch.room,
        line: stretch.line,
        origin: tailOf(stretch),
        text: [stretch.room, ...stretch.runs.map((each) => each.task ?? "")].join("\n").toLowerCase(),
      }))
      .filter((row) => row.text.includes(needle));
  });
  const earlier = $derived(stretches.earlier);

  function fork(row: Session): void {
    if (row.origin === null) return;
    if (!u.send(openSession(row.room, "nothing", row.origin))) return;
    onLeave();
    u.go({ kind: "talk", address: row.room });
  }

  // The window of rows in view, recomputed as the column scrolls or
  // changes size.
  let list = $state<HTMLElement | undefined>(undefined);
  let shown = $state({ first: 0, end: OVERSCAN });
  let rowPx = $state(0);
  $effect(() => {
    const held = list;
    const column = scroller();
    if (held === undefined || column === undefined) return;
    const total = rows.length;
    const measure = (): void => {
      const row = Number.parseFloat(getComputedStyle(held).getPropertyValue("--spacing-control"));
      const top = held.getBoundingClientRect().top - column.getBoundingClientRect().top;
      rowPx = row;
      shown = windowOf(total, row, Math.max(0, -top), column.clientHeight);
    };
    measure();
    const watching = new ResizeObserver(measure);
    watching.observe(column);
    column.addEventListener("scroll", measure, { passive: true });
    return () => {
      watching.disconnect();
      column.removeEventListener("scroll", measure);
    };
  });
</script>

<Section title="mailbox_recent" empty={search.trim() === "" ? "mailbox_recent_none" : "mailbox_search_none"} count={rows.length}>
  {#if stretches.all.length > 0}
    <input
      type="search"
      class="mt-snug w-full rounded-control bg-page px-snug py-tight text-label placeholder:text-text-faint"
      aria-label={say($lang, "mailbox_search")}
      aria-describedby="{uid}-reach"
      placeholder={say($lang, "mailbox_search")}
      bind:value={search}
    />
    <p id="{uid}-reach" class="py-tight text-note text-text-faint">{say($lang, "mailbox_search_reach")}</p>
  {/if}
  <ul
    bind:this={list}
    style:height={`${String(rows.length * rowPx)}px`}
    style:padding-top={`${String(shown.first * rowPx)}px`}
  >
    {#each rows.slice(shown.first, shown.end) as row (`${row.room}@${String(row.line.began)}`)}
      <Row
        {...rowOf({ room: row.room, line: row.line, forkable: row.origin !== null }, $lang, ago($lang, row.line.at, u.now()), {
          href: toFragment({ kind: "talk", address: row.room }),
          follow: onLeave,
          fork: () => {
            fork(row);
          },
        })}
      />
    {/each}
  </ul>
  {#if earlier > 0}
    <p class="py-snug text-note text-text-faint">{fill(say($lang, "mailbox_earlier"), { n: String(earlier) })}</p>
  {/if}
</Section>
