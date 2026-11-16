<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import { newestWorking } from "../../core/belief/live";
  import { cancel } from "../../core/commands";
  import { fill, say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { tailOf } from "../../core/stretches";
  import { ago } from "../../core/time";
  import { ui } from "../../ui";
  import type { Address, Origin, SessionLine } from "../../wire";
  import { OVERSCAN, windowOf } from "../run/lanes";
  import { branchFrom } from "../talk/branching";
  import type { BranchHere } from "../talk/branching";
  import { askStretches } from "../world/stretches.svelte";
  import { rowOf } from "./recent_row";
  import Row from "./recent_row.look.svelte";
  import Section from "./section.svelte";

  interface Props {
    readonly scroller: () => HTMLElement | undefined;

    readonly onLeave: () => void;
  }

  const { scroller, onLeave }: Props = $props();

  const u = ui();
  const { lang } = u;
  const { belief } = u.conn;
  const branching = u.branches.held;
  const uid = $props.id();

  interface Session {
    readonly room: Address;
    readonly line: SessionLine;

    readonly origin: Origin | null;

    readonly text: string;
  }

  const stretches = askStretches(u);
  let search = $state("");
  const rows = $derived.by((): Session[] => {
    const needle = search.trim().toLowerCase();
    return stretches.all
      .map((stretch): Session => ({
        room: stretch.room,
        line: stretch.line,
        origin: tailOf(stretch),
        text: [stretch.room, ...stretch.runs.map((each) => each.task ?? "")]
          .join("\n")
          .toLowerCase(),
      }))
      .filter((row) => row.text.includes(needle));
  });
  const earlier = $derived(stretches.earlier);

  function fork(row: Session): void {
    if (row.origin === null) return;
    u.branches.open(row.room, { kind: "branch", origin: row.origin }, onLeave);
  }

  function branchOf(row: Session): BranchHere {
    return row.origin === null ? { kind: "none" } : branchFrom($branching, row.room, row.origin);
  }

  function stopIn(room: Address): (() => void) | null {
    const going = newestWorking($belief, room);
    if (going === undefined) return null;
    return () => {
      u.send(cancel(going.run));
    };
  }

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

<Section
  title="mailbox_recent"
  empty={search.trim() === "" ? "mailbox_recent_none" : "mailbox_search_none"}
  count={rows.length}
>
  {#if stretches.all.length > 0}
    <input
      type="search"
      class="mt-snug w-full rounded-control bg-page px-snug py-tight text-label placeholder:text-text-faint"
      aria-label={say($lang, "mailbox_search")}
      aria-describedby="{uid}-reach"
      placeholder={say($lang, "mailbox_search")}
      bind:value={search}
    />
    <p id="{uid}-reach" class="py-tight text-note text-text-faint">
      {say($lang, "mailbox_search_reach")}
    </p>
  {/if}
  <ul
    bind:this={list}
    style:height={`${String(rows.length * rowPx)}px`}
    style:padding-top={`${String(shown.first * rowPx)}px`}
  >
    {#each rows.slice(shown.first, shown.end) as row (`${row.room}@${String(row.line.began)}`)}
      {@const branch = branchOf(row)}
      <Row
        {...rowOf(
          { room: row.room, line: row.line, forkable: row.origin !== null, branch },
          $lang,
          ago($lang, row.line.at, u.now()),
          {
            href: toFragment({ kind: "talk", address: row.room }),
            follow: onLeave,
            fork: () => {
              fork(row);
            },
            stop: branch.kind === "refused" ? stopIn(row.room) : null,
          },
        )}
      />
    {/each}
  </ul>
  {#if earlier > 0}
    <p class="py-snug text-note text-text-faint">
      {fill(say($lang, "mailbox_earlier"), { n: String(earlier) })}
    </p>
  {/if}
</Section>
