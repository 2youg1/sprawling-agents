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
  import { untrack } from "svelte";

  import { heldIn } from "../../core/belief/rooms";
  import type { RunBelief } from "../../core/belief";
  import { openSession } from "../../core/commands";
  import { fill, say } from "../../core/lang";
  import type { Key } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { ago } from "../../core/time";
  import { ui } from "../../ui";
  import type { Address, Origin, SessionLine, SessionsAnswer } from "../../wire";
  import Tip from "../parts/tip.svelte";
  import { OVERSCAN, windowOf } from "../run/lanes";
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
  const belief = u.conn.belief;

  interface Row {
    readonly room: Address;
    readonly line: SessionLine;
    // Where a fork of this session would cut: its last run's tail, or
    // nothing when that run is older than the runs this page holds.
    readonly origin: Origin | null;
  }

  // Every room this page has seen a run in, spelled as the address its
  // runs carry. A key of the joined names, so the asking below follows
  // the set of rooms and not every record that moves a run.
  const rooms = $derived(
    [...$belief.rooms.keys()].flatMap((room) => {
      const addr = heldIn($belief, room).at(-1)?.addr ?? null;
      return addr === null ? [] : [addr];
    }),
  );
  const roomsKey = $derived(rooms.join("\n"));

  let answers = $state.raw<Readonly<Record<string, SessionsAnswer>>>({});
  $effect(() => {
    // Asked again when the set of rooms changes, and not on every record.
    const stops = (roomsKey === "" ? [] : untrack(() => rooms)).map((room) =>
      u.conn.asking.ask({ sessions: { room } }).subscribe((answer) => {
        // Read without following: the store answers inside this effect,
        // and the effect must not wait on what it writes.
        if (answer !== undefined && "sessions" in answer) answers = { ...untrack(() => answers), [room]: answer.sessions };
      }),
    );
    return () => {
      for (const stop of stops) stop();
    };
  });

  // The run a session ended with: the newest run of the room whose last
  // line falls between this session's first line and the next one's.
  function lastRun(runs: readonly RunBelief[], line: SessionLine, next: SessionLine | undefined): RunBelief | undefined {
    return [...runs].reverse().find((run) => run.lastSeq >= line.began && (next === undefined || run.lastSeq < next.began));
  }

  const rows = $derived.by((): Row[] => {
    const all = Object.values(answers).flatMap((answer) => {
      const runs = heldIn($belief, answer.room);
      return answer.sessions.map((line, at): Row => {
        const run = lastRun(runs, line, answer.sessions[at - 1]);
        return { room: answer.room, line, origin: run === undefined ? null : { run: run.run, at_seq: run.lastSeq } };
      });
    });
    return all.sort((a, b) => b.line.at - a.line.at);
  });
  const earlier = $derived(Object.values(answers).reduce((sum, answer) => sum + answer.earlier, 0));

  function startOf(line: SessionLine): Key {
    if (line.start === "dispatched") return "mailbox_start_dispatched";
    const opened = line.start.opened;
    if (opened.from !== undefined && opened.from !== null) return "mailbox_start_forked";
    return opened.carry === "handoff" ? "mailbox_start_carried" : "mailbox_start_new";
  }

  function fork(row: Row): void {
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

<Section title="mailbox_recent" empty="mailbox_recent_none" count={rows.length}>
  <ul
    bind:this={list}
    style:height={`${String(rows.length * rowPx)}px`}
    style:padding-top={`${String(shown.first * rowPx)}px`}
  >
    {#each rows.slice(shown.first, shown.end) as row (`${row.room}@${String(row.line.began)}`)}
      {@const why = row.origin === null ? say($lang, "mailbox_fork_unheld") : undefined}
      <li class="-mx-snug flex h-control items-center gap-snug rounded-card px-snug hover:wash">
        <a
          href={toFragment({ kind: "talk", address: row.room })}
          class="grid min-w-0 flex-1 grid-cols-[minmax(10ch,1fr)_minmax(0,auto)_var(--spacing-figure)] items-center gap-x-base rounded-control whitespace-nowrap focus-visible:wash"
          data-entry
          onclick={onLeave}
        >
          <span class="min-w-0 truncate">{row.room}</span>
          <span class="truncate text-note text-text-faint">
            {say($lang, startOf(row.line))} · {fill(say($lang, "mailbox_runs"), { n: String(row.line.runs) })}
          </span>
          <span class="figure text-right text-note text-text-faint">{ago($lang, row.line.at, u.now())}</span>
        </a>
        <Tip text={why ?? say($lang, "mailbox_fork_last")}>
          {#snippet children(hint)}
            <button
              type="button"
              class="grid size-control-sm shrink-0 place-items-center rounded-control text-text-quiet hover:bg-raised-hover hover:text-text aria-disabled:text-text-disabled"
              aria-label={say($lang, "mailbox_fork_last")}
              aria-describedby={hint}
              aria-disabled={why !== undefined}
              onclick={() => {
                fork(row);
              }}
            >
              <!-- wording-ok: a drawing in type, not a word; the action's name is the aria-label beside it. -->
              ⑂
            </button>
          {/snippet}
        </Tip>
        <kbd class="entry-n" aria-hidden="true"></kbd>
      </li>
    {/each}
  </ul>
  {#if earlier > 0}
    <p class="py-snug text-note text-text-faint">{fill(say($lang, "mailbox_earlier"), { n: String(earlier) })}</p>
  {/if}
</Section>
