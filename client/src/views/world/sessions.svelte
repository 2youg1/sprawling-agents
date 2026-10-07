<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The world layer's sessions pane (client/Spec.lean §7K): one row per session,
  // the stretches of every room including the past ones, newest first -
  // the pinned above, then grouped by building. A row is a link that makes
  // its session the one in main: the room's current session is talked to,
  // an earlier one is read (`talk/past.svelte`). Beside each row its menu
  // pins it and gives and strips tags (`session_menu.svelte`); a row of the
  // tags in use above the list filters it by one.
  //
  // A row is titled by the session's name, or its room when it has none,
  // and says the model, effort and workspace of its last run and the
  // start of its last reply, cut to the width the row has
  // (`crates/wire/spec/Answer/Sessions.lean` D27).
  // A session a resident handed down says which resident did, under its
  // title, read off the session line (client D83).
  //
  // The rows are `core/stretches.ts`'s reading of the city's answers
  // (`stretches.svelte.ts`), and the tags are the person's preferences
  // as the city keeps them (`core/tags.ts`).
  //
  // This file is the seat: it asks, filters and groups, and lays the
  // pane out - its label, the column that scrolls - while `./sessions.ts`
  // reads each row and `./sessions.look.svelte` draws the filter and the
  // rows.
  import type { Snippet } from "svelte";

  import { say } from "../../core/lang";
  import { grouped, pinningOf } from "../../core/stretches";
  import type { Stretch } from "../../core/stretches";
  import { inUse, namedIn, tagsOf } from "../../core/tags";
  import type { Named } from "../../core/tags";
  import { ui } from "../../ui";
  import type { Address, Seq, Tag } from "../../wire";
  import { ticker } from "../talk/timing";
  import { filterOf, rowOf } from "./sessions";
  import type { SessionsLook } from "./sessions";
  import Look from "./sessions.look.svelte";
  import { askStretches } from "./stretches.svelte";

  interface Props {
    // The room whose session is in main, and which of its sessions:
    // absent is the room's current one.
    readonly here: Address;
    readonly session?: Seq | undefined;
    // Whether the pane is as narrow as the right pane leaves it, which
    // drops the second line and the time.
    readonly narrow: boolean;
    // The pane's label: a menu that moves it, in the panorama tier.
    readonly head: Snippet;
  }

  const { here, session, narrow, head }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const held = u.tags.held;
  const stretches = askStretches(u);

  // A running row says how long its run has gone, on the page's one
  // clock: it moves only while a row reads it and the page is seen, and
  // every reading is recomputed from the run's own start (client/Spec.lean §4-59).
  const tick = ticker(u.now);

  const city = $derived($belief.city);
  function named(stretch: Stretch): Named | null {
    return namedIn(city, stretch.room, stretch.line.began, stretch.line.workspace ?? null);
  }
  function tagsFor(stretch: Stretch): readonly Tag[] {
    const name = named(stretch);
    return name === null ? [] : tagsOf($held, name);
  }

  // One tag the pane is filtered by, or every row. A tag nobody carries
  // any more filters nothing.
  let filter = $state<Tag | null>(null);
  const offered = $derived(city === null ? [] : inUse(stretches.all.map(tagsFor)));
  const by = $derived(filter !== null && offered.includes(filter) ? filter : null);
  const groups = $derived(grouped(stretches.all, tagsFor, by));

  function chosen(stretch: Stretch): boolean {
    return stretch.room === here && (session === undefined ? stretch.current : stretch.line.began === session);
  }

  const look: SessionsLook = $derived({
    filter: filterOf(offered, by, { label: say($lang, "world_tags"), all: say($lang, "world_tags_all") }, (tag) => {
      filter = tag;
    }),
    narrow,
    groups: groups.map((group) => ({
      key: group.kind === "pinned" ? "" : group.building,
      heading: group.kind === "pinned" ? say($lang, "world_pinned") : group.building,
      rows: group.rows.map((stretch) => {
        const tags = tagsFor(stretch);
        return rowOf(stretch, { lang: $lang, now: $tick, chosen: chosen(stretch), tags, pinning: pinningOf(stretch, tags), named: named(stretch) });
      }),
    })),
  });
</script>

<section class="flex min-h-0 flex-1 flex-col overflow-hidden px-snug" aria-label={say($lang, "world_sessions")}>
  {@render head()}
  <Look {...look} />
</section>
